use crate::bits::{BitReader, BitSrc};
use crate::lmd::{LiteralLen, MatchDistancePack, MatchLen};

use super::constants::*;
use super::error_kind::FseErrorKind;
use super::object::Fse;
use super::weights::Weights;

use std::convert::{From, TryFrom};
use std::fmt::{self, Debug, Formatter};

/// FSE decoding tables.
/// Promises that table is of the correct length and that entries are sound.
#[repr(C)]
pub struct Decoder(
    [VEntry; (L_STATES + M_STATES + D_STATES) as usize],
    [UEntry; U_STATES as usize],
);

impl Decoder {
    pub fn init(&mut self, weights: &Weights) {
        self.init_v_table(weights);
        self.init_u_table(weights);
    }

    fn init_v_table(&mut self, weights: &Weights) {
        assert!(self.0.len() <= u16::MAX as usize);
        let mut offset = 0;
        self.0[0] = VEntry::default();
        offset += 0;
        unsafe {
            build_v_table_block(
                weights.l_block(),
                &L_EXTRA_BITS,
                &L_BASE_VALUE,
                &mut self.0[offset as usize..offset as usize + L_STATES as usize],
                offset,
            )
        };
        offset += L_STATES as i16;
        unsafe {
            build_v_table_block(
                weights.m_block(),
                &M_EXTRA_BITS,
                &M_BASE_VALUE,
                &mut self.0[offset as usize..offset as usize + M_STATES as usize],
                offset,
            )
        };
        offset += M_STATES as i16;
        unsafe {
            build_v_table_block(
                weights.d_block(),
                &D_EXTRA_BITS,
                &D_BASE_VALUE,
                &mut self.0[offset as usize..offset as usize + D_STATES as usize],
                offset,
            )
        };
        offset += D_STATES as i16;
        assert_eq!(offset as usize, self.0.len());
    }

    fn init_u_table(&mut self, weights: &Weights) {
        assert!(self.0.len() <= u16::MAX as usize);
        unsafe { build_u_table(weights.u_block(), &mut self.1) };
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_L_BITS`
    #[inline(always)]
    pub unsafe fn l<T>(&self, reader: &mut BitReader<T>, state: &mut L) -> LiteralLen<Fse>
    where
        T: BitSrc,
    {
        debug_assert!(state.check());
        debug_assert!(state.0 < self.0.len());
        LiteralLen::new_unchecked(self.0.get_unchecked(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_M_BITS`
    #[allow(clippy::int_plus_one)]
    #[inline(always)]
    pub unsafe fn m<T>(&self, reader: &mut BitReader<T>, state: &mut M) -> MatchLen<Fse>
    where
        T: BitSrc,
    {
        debug_assert!(state.check());
        debug_assert!(state.0 < self.0.len());
        MatchLen::new_unchecked(self.0.get_unchecked(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_D_BITS`
    #[inline(always)]
    pub unsafe fn d<T: BitSrc>(
        &self,
        reader: &mut BitReader<T>,
        state: &mut D,
    ) -> MatchDistancePack<Fse> {
        debug_assert!(state.check());
        debug_assert!(state.0 < self.0.len());
        MatchDistancePack::new_unchecked(self.0.get_unchecked(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_U_BITS`
    #[inline(always)]
    pub unsafe fn u<T>(&self, reader: &mut BitReader<T>, state: &mut U) -> u8
    where
        T: BitSrc,
    {
        debug_assert!(state.check());
        debug_assert!(state.0 < self.1.len());
        self.1.get_unchecked(state.0).decode(reader, &mut state.0)
    }
}

impl Debug for Decoder {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), fmt::Error> {
        f.debug_tuple("Decoder").field(&self.0.as_ref()).field(&self.1.as_ref()).finish()
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self(
            [VEntry::default(); L_STATES as usize + M_STATES as usize + D_STATES as usize],
            [UEntry::default(); U_STATES as usize],
        )
    }
}

macro_rules! create_state_struct {
    ($name:ident, $off: expr, $len:expr, $err:expr) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        pub struct $name(usize);

        impl $name {
            pub fn new(v: usize) -> Self {
                assert!(v < $len);
                Self($off + v)
            }

            #[allow(clippy::int_plus_one)]
            #[allow(dead_code)]
            #[allow(unused_comparisons)]
            fn check(&self) -> bool {
                $off <= self.0 && self.0 < $off + $len
            }

            #[inline(always)]
            pub fn get(&self) -> usize {
                self.0
            }
        }

        impl TryFrom<usize> for $name {
            type Error = crate::Error;

            #[inline(always)]
            fn try_from(v: usize) -> Result<Self, Self::Error> {
                if v < $len {
                    Ok(Self($off + v))
                } else {
                    Err($err)
                }
            }
        }

        impl From<$name> for usize {
            #[allow(unused_comparisons)]
            #[allow(clippy::int_plus_one)]
            #[inline(always)]
            fn from(t: $name) -> usize {
                debug_assert!(t.0 - $off < $len);
                t.0 - $off
            }
        }

        impl Default for $name {
            #[inline(always)]
            fn default() -> Self {
                Self($off)
            }
        }
    };
}

create_state_struct!(L, 0, L_STATES as usize, FseErrorKind::BadLmdState.into());
create_state_struct!(M, L_STATES as usize, M_STATES as usize, FseErrorKind::BadLmdState.into());
create_state_struct!(
    D,
    L_STATES as usize + M_STATES as usize,
    D_STATES as usize,
    FseErrorKind::BadLmdState.into()
);
create_state_struct!(U, 0usize, U_STATES as usize, FseErrorKind::BadLiteralState.into());

#[derive(Copy, Clone, Debug, Default)]
#[repr(align(8))]
pub struct VEntry {
    k: u8,
    v_bits: u8,
    delta: i16,
    v_base: u32,
}

impl VEntry {
    #[inline(always)]
    unsafe fn decode<T: BitSrc>(self, bsi: &mut BitReader<T>, state: &mut usize) -> u32 {
        *state = (bsi.pull(self.k as usize) as isize + self.delta as isize) as usize;
        self.v_base + bsi.pull(self.v_bits as usize) as u32
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(align(4))]
pub struct UEntry {
    k: u8,
    symbol: u8,
    delta: i16,
}

impl UEntry {
    #[inline(always)]
    pub unsafe fn decode<T: BitSrc>(self, reader: &mut BitReader<T>, state: &mut usize) -> u8 {
        *state = (reader.pull(self.k as usize) as isize + self.delta as isize) as usize;
        self.symbol
    }
}

/// # Safety
///
/// `weights` table totals <= `table.len()`
/// `offset` with respect to the entire v_table is correct
#[allow(arithmetic_overflow)]
#[allow(clippy::needless_range_loop)]
unsafe fn build_v_table_block(
    weights: &[u16],
    v_bits_table: &[u8],
    v_base_table: &[u32],
    table: &mut [VEntry],
    offset: i16,
) {
    assert_eq!(v_bits_table.len(), weights.len());
    assert_eq!(v_base_table.len(), weights.len());
    let n_states = table.len() as u32;
    assert!(n_states.is_power_of_two());
    let n_clz = n_states.leading_zeros();
    let mut e = VEntry::default();
    let mut total = 0;
    for i in 0..weights.len() {
        let w = *weights.get_unchecked(i) as u32;
        if w == 0 {
            continue;
        }
        debug_assert!(total + w <= n_states);
        let k = w.leading_zeros() - n_clz;
        let x = ((n_states << 1) >> k) - w;
        let v_bits = *v_bits_table.get_unchecked(i);
        let v_base = *v_base_table.get_unchecked(i);
        e.k = k as u8;
        e.v_bits = v_bits;
        e.v_base = v_base;
        for j in 0..x {
            e.delta = (((w as i32 + j as i32) << k) - n_states as i32) as i16 + offset;
            *table.get_unchecked_mut((total + j) as usize) = e;
        }
        e.k = (k as i32 - 1) as u8;
        for j in x..w {
            e.delta = ((j - x) << (k - 1)) as i16 + offset;
            *table.get_unchecked_mut((total + j) as usize) = e;
        }
        total += w;
    }
    // At this point, if our weights are correctly normalized, we are done.
    // However, with broken or malicious inputs we may have unpopulated and invalid states that
    // are reachable.
    // To cover this, we'll configure the entries to work as latches that lock in the invalid state
    // and consume no additional input bits. This invalid state, or the lack of a final resting
    // init state, can be easily detected and handled by callers.
    for i in (total as usize)..table.len() {
        *table.get_unchecked_mut(i) =
            VEntry { k: 0, v_bits: 0, delta: offset + i as i16, v_base: 0 };
    }
}

/// # Safety
///
/// `weights` table totals <= `table.len()`
#[allow(arithmetic_overflow)]
#[allow(clippy::needless_range_loop)]
pub unsafe fn build_u_table(weights: &[u16], table: &mut [UEntry]) {
    let n_states = table.len() as u32;
    assert!(n_states.is_power_of_two());
    let n_clz = n_states.leading_zeros();
    let mut e = UEntry::default();
    let mut total = 0;
    for i in 0..weights.len() {
        let w = *weights.get_unchecked(i) as u32;
        if w == 0 {
            continue;
        }
        debug_assert!(total + w <= n_states);
        let k = w.leading_zeros() - n_clz;
        let x = ((n_states << 1) >> k) - w;
        e.symbol = i as u8;
        e.k = k as u8;
        for j in 0..x {
            e.delta = (((w as i32 + j as i32) << k) - n_states as i32) as i16;
            *table.get_unchecked_mut((total + j) as usize) = e;
        }
        e.k = (k as i32 - 1) as u8;
        for j in x..w {
            e.delta = ((j - x) << (k - 1)) as i16;
            *table.get_unchecked_mut((total + j) as usize) = e;
        }
        total += w;
    }
    // At this point, if our weights are correctly normalized, we are done.
    // However, with broken or malicious inputs we may have unpopulated and invalid states that
    // are reachable.
    // To cover this, we'll configure the entries to work as latches that lock in the invalid state
    // and consume no additional input bits. This invalid state, or the lack of a final resting
    // init state, can be easily detected and handled by callers.
    for i in (total as usize)..table.len() {
        *table.get_unchecked_mut(i) = UEntry { k: 0, symbol: 0, delta: i as i16 };
    }
}
