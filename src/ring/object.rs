use crate::match_kit;
use crate::types::Idx;

use super::ring_box::RingBox;
use super::ring_size::RingSize;
use super::ring_type::RingType;
use super::ring_view::RingView;

use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;

#[inline(always)]
pub const fn overmatch_len(len: usize) -> usize {
    len + 5 * mem::size_of::<usize>()
}

pub struct Ring<'a, T>(*mut u8, PhantomData<T>, PhantomData<&'a mut ()>);

// Implementation notes:
//
// Hybrid ring buffer.
//
// |tttt|HHHH|...............................................|TTTT|hhhh|S|
// <-------------------------- RING_CAPACITY ---------------------------->
//      <----------------------- RING_SIZE ----------------------->
//      ^ PTR *mut u8
//
// Tag  | Zone           | Size
// ----------------------------------
// HHHH | head           | RING_LIMIT
// TTTT | tail           | RING_LIMIT
// hhhh | head shadow    | RING_LIMIT
// tttt | tail shadow    | RING_LIMIT
// S    | Slack          | WIDTH

impl<'a, T: RingType> Ring<'a, T> {
    /// May overmatch `max` by  `overmatch_len(len)` bytes
    #[inline(always)]
    pub fn coarse_match_inc(&self, idxs: (Idx, Idx), len: usize, max: usize) -> usize {
        assert!(overmatch_len(len) <= T::RING_LIMIT as usize);
        debug_assert!(self.head_shadowed_len(overmatch_len(len)));
        let indexes = (
            (usize::from(idxs.0)) % T::RING_SIZE as usize,
            (usize::from(idxs.1)) % T::RING_SIZE as usize,
        );
        let u_0 = unsafe { self.0.add(indexes.0 + len).cast::<usize>().read_unaligned() };
        let u_1 = unsafe { self.0.add(indexes.1 + len).cast::<usize>().read_unaligned() };
        let x = u_0 ^ u_1;
        if x != 0 {
            // Likely
            len + match_kit::nclz_bytes(x) as usize
        } else {
            // Unlikely.
            unsafe { self.coarse_match_inc_cont(indexes, len + mem::size_of::<usize>(), max) }
        }
    }

    unsafe fn coarse_match_inc_cont(
        &self,
        mut indexes: (usize, usize),
        mut len: usize,
        max: usize,
    ) -> usize {
        let base_len = len;
        loop {
            for i in 0..4 {
                let off = base_len + i * mem::size_of::<usize>();
                let u_0 = self.0.add(indexes.0 + off).cast::<usize>().read_unaligned();
                let u_1 = self.0.add(indexes.1 + off).cast::<usize>().read_unaligned();
                let x = u_0 ^ u_1;
                if x != 0 {
                    return len + i * mem::size_of::<usize>() + match_kit::nclz_bytes(x) as usize;
                }
            }
            if len >= max {
                break;
            }
            len += 4 * mem::size_of::<usize>();
            indexes = (
                indexes.0.wrapping_add(4 * mem::size_of::<usize>()) % T::RING_SIZE as usize,
                indexes.1.wrapping_add(4 * mem::size_of::<usize>()) % T::RING_SIZE as usize,
            );
        }
        max
    }

    /// May overmatch `max` by  `overmatch_len(len)` bytes
    #[inline(always)]
    pub fn match_dec_coarse(&self, idxs: (Idx, Idx), len: usize, max: usize) -> usize {
        assert!(overmatch_len(len) <= T::RING_LIMIT as usize);
        debug_assert!(self.head_shadowed_len(overmatch_len(len)));
        let off = overmatch_len(len);
        let indexes = (
            (usize::from(idxs.0).wrapping_sub(off)) % T::RING_SIZE as usize,
            (usize::from(idxs.1).wrapping_sub(off)) % T::RING_SIZE as usize,
        );
        let off = 4 * mem::size_of::<usize>();
        let u_0 = unsafe { self.0.add(indexes.0 + off).cast::<usize>().read_unaligned() };
        let u_1 = unsafe { self.0.add(indexes.1 + off).cast::<usize>().read_unaligned() };
        let x = u_0 ^ u_1;
        if x != 0 {
            // Likely
            len + match_kit::nctz_bytes(x) as usize
        } else {
            // Unlikely.
            unsafe { self.match_dec_cont(indexes, len + mem::size_of::<usize>(), max) }
        }
    }

    unsafe fn match_dec_cont(
        &self,
        mut indexes: (usize, usize),
        mut len: usize,
        max: usize,
    ) -> usize {
        loop {
            for i in 0..4 {
                let off = (3 - i) * mem::size_of::<usize>();
                let u_0 = self.0.add(indexes.0 + off).cast::<usize>().read_unaligned();
                let u_1 = self.0.add(indexes.1 + off).cast::<usize>().read_unaligned();
                let x = u_0 ^ u_1;
                if x != 0 {
                    return len + i * mem::size_of::<usize>() + match_kit::nctz_bytes(x) as usize;
                }
            }
            if len >= max {
                break;
            }
            len += 4 * mem::size_of::<usize>();
            indexes = (
                indexes.0.wrapping_sub(4 * mem::size_of::<usize>()) % T::RING_SIZE as usize,
                indexes.1.wrapping_sub(4 * mem::size_of::<usize>()) % T::RING_SIZE as usize,
            );
        }
        max
    }

    pub fn head_shadowed(&self) -> bool {
        self.head_shadowed_len(T::RING_LIMIT as usize)
    }

    #[inline(always)]
    pub fn head_shadowed_len(&self, len: usize) -> bool {
        unsafe { zone_eq::<T>(self.0, len) }
    }

    #[inline(always)]
    pub fn head_copy_out(&mut self) {
        self.head_copy_out_len(T::RING_LIMIT as usize);
    }

    /// Copy head -> head shadow
    #[inline(always)]
    pub fn head_copy_out_len(&mut self, len: usize) {
        unsafe { zone_copy_1::<T>(self.0, len) };
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn head_copy_in(&mut self) {
        self.head_copy_in_len(T::RING_LIMIT as usize);
    }

    /// Copy head shadow -> head
    #[inline(always)]
    pub fn head_copy_in_len(&mut self, len: usize) {
        unsafe { zone_copy_2::<T>(self.0, len) };
    }

    #[allow(dead_code)]
    pub fn tail_shadowed(&self) -> bool {
        self.tail_shadowed_len(T::RING_LIMIT as usize)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn tail_shadowed_len(&self, len: usize) -> bool {
        unsafe { zone_eq::<T>(self.0.sub(T::RING_LIMIT as usize), len) }
    }

    #[inline(always)]
    pub fn tail_copy_out(&mut self) {
        self.tail_copy_out_len(T::RING_LIMIT as usize);
    }

    /// Copy tail -> tail shadow
    #[inline(always)]
    pub fn tail_copy_out_len(&mut self, len: usize) {
        unsafe { zone_copy_2::<T>(self.0.sub(T::RING_LIMIT as usize), len) };
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn tail_copy_in(&mut self) {
        self.tail_copy_in_len(T::RING_LIMIT as usize);
    }

    /// Copy tail shadow -> tail
    #[allow(dead_code)]
    #[inline(always)]
    pub fn tail_copy_in_len(&mut self, len: usize) {
        assert!(len <= T::RING_LIMIT as usize);
        unsafe { zone_copy_1::<T>(self.0.sub(T::RING_LIMIT as usize), len) };
    }

    #[inline(always)]
    pub fn view(&self, head: Idx, tail: Idx) -> RingView<T> {
        RingView::new(&self, head, tail)
    }
}

impl<'a, T: RingSize> Ring<'a, T> {
    #[inline(always)]
    pub fn get_u32(&self, idx: Idx) -> u32 {
        let index = idx % T::RING_SIZE;
        unsafe { self.0.add(index as usize).cast::<u32>().read_unaligned() }
    }

    #[inline(always)]
    pub unsafe fn set_quad_index(&mut self, index: usize, u: u32) {
        debug_assert!(index < T::RING_SIZE as usize);
        self.0.add(index).cast::<u32>().write_unaligned(u);
    }
}

impl<'a, T: RingSize> Deref for Ring<'a, T> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.0, T::RING_SIZE as usize) }
    }
}

impl<'a, T: RingSize> DerefMut for Ring<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.0, T::RING_SIZE as usize) }
    }
}

impl<'a, T: RingType> From<&'a mut RingBox<T>> for Ring<'a, T> {
    #[inline(always)]
    fn from(ring_box: &'a mut RingBox<T>) -> Self {
        Self(
            unsafe { ring_box.0.as_mut_ptr().add(T::RING_LIMIT as usize) },
            PhantomData::default(),
            PhantomData::default(),
        )
    }
}

#[inline(always)]
unsafe fn zone_copy_1<T: RingType>(ptr: *mut u8, len: usize) {
    assert!(len <= T::RING_LIMIT as usize);
    ptr::copy_nonoverlapping(ptr, ptr.add(T::RING_SIZE as usize), len);
}

#[inline(always)]
unsafe fn zone_copy_2<T: RingType>(ptr: *mut u8, len: usize) {
    ptr::copy_nonoverlapping(ptr.add(T::RING_SIZE as usize), ptr, len);
}

#[inline(always)]
unsafe fn zone_eq<T: RingType>(ptr: *mut u8, len: usize) -> bool {
    assert!(len <= T::RING_LIMIT as usize);
    let u = slice::from_raw_parts(ptr.add(T::RING_SIZE as usize), len);
    let v = slice::from_raw_parts(ptr, len);
    u == v
}
