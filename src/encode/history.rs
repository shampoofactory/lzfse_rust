use crate::encode::constants::{Q1, Q3};
use crate::encode::MatchUnit;
use crate::types::Idx;

use std::ops::Deref;

#[cfg(test)]
use crate::encode::constants::Q2;

pub const HASH_BITS: u32 = 14;

// Aligned/ power of two values. Minimum 4.
pub const HASH_WIDTH: usize = 4;

pub struct HistoryTable(Box<[History]>, #[cfg(test)] Ward);

impl HistoryTable {
    const SIZE: usize = 1 << HASH_BITS;

    // TODO consider new with idx method

    /// Push a new history item.
    ///
    /// Items must be pushed in strict sequential order and must not wrap around.
    #[inline(always)]
    pub fn push<M: MatchUnit>(&mut self, item: Item) -> History {
        #[cfg(test)]
        debug_assert!(self.1.push(item));
        let queue = self.get_mut::<M>(item.val);
        let copy = *queue;
        queue.push(item);
        copy
    }

    #[inline(always)]
    fn get_mut<M: MatchUnit>(&mut self, val: u32) -> &mut History {
        unsafe { self.0.get_unchecked_mut(index::<M>(val)) }
    }

    /// Clamp all history `idx` values to a maximum of `idx - Q1` with respect to the specified
    /// `idx` value.
    ///
    /// Clamping removes old items which might otherwise wrap back around and corrupt our
    /// history.
    ///
    /// Allows us to push a maximum of 0x8000_0000 items, with sequential `idx` values, without
    /// additional clamping.
    #[cold]
    pub fn clamp(&mut self, idx: Idx) {
        #[cfg(test)]
        debug_assert!(self.1.clamp(idx));
        self.0.iter_mut().for_each(|u| u.clamp_rebias(idx, 0));
    }

    /// Clamp all history `idx` values to a maximum of `idx - Q1` with respect to the specified
    /// `idx` value and then subtract the specified `delta` offset.
    ///  
    /// Clamping removes old items which might otherwise wrap back around and corrupt our
    /// history.
    ///
    /// Allows us to push a maximum of 0x8000_0000 items, with sequential `idx - delta ` values,
    /// without additional clamping.
    #[cold]
    pub fn clamp_rebias(&mut self, idx: Idx, delta: u32) {
        #[cfg(test)]
        debug_assert!(self.1.clamp_rebias(idx, delta));
        self.0.iter_mut().for_each(|u| u.clamp_rebias(idx, delta));
    }

    /// All history `idx` values are set to `Idx::Q3`.
    ///
    /// Allows us to push a maximum of 0x8000_0000 items, with sequential `idx` values starting from
    /// `Idx::Q0`, without additional clamping.
    pub fn reset(&mut self) {
        self.reset_with_idx(Idx::Q0)
    }

    #[cold]
    pub fn reset_with_idx(&mut self, idx: Idx) {
        self.0.iter_mut().for_each(|u| *u = History::new(Item::new(0, idx - Q1)));
        #[cfg(test)]
        {
            self.1 = Ward::new(idx);
        }
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self(
            vec![History::default(); Self::SIZE].into_boxed_slice(),
            #[cfg(test)]
            Ward::default(),
        )
    }
}

/// Ordered (checked on push) history fixed length item queue.
/// [ 0, 1, 2, ... , HASH_WIDTH - 1 ]
///   ^ new          ^ old
#[repr(align(32))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct History([Item; HASH_WIDTH]);

impl History {
    #[inline(always)]
    const fn new(item: Item) -> Self {
        Self([item; HASH_WIDTH])
    }

    #[inline(always)]
    fn push(&mut self, item: Item) {
        debug_assert!(!is_wrapping(item.idx, self.0[HASH_WIDTH - 1].idx));
        let mut i = HASH_WIDTH - 1;
        while i != 0 {
            self.0[i] = self.0[i - 1];
            i -= 1;
        }
        self.0[0] = item;
    }

    #[inline(always)]
    fn clamp_rebias(&mut self, idx: Idx, delta: u32) {
        for item in self.0.iter_mut() {
            debug_assert!(!is_wrapping(idx, item.idx));
            if (idx - item.idx) as u32 > Q1 {
                item.idx = idx - Q1 - delta;
            } else {
                item.idx -= delta;
            }
        }
    }
}

impl Deref for History {
    type Target = [Item];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Item {
    pub val: u32,
    pub idx: Idx,
}

impl Item {
    #[inline(always)]
    pub const fn new(val: u32, idx: Idx) -> Self {
        Self { val, idx }
    }
}

/// Ward. Test/ debug assistant.
#[cfg(test)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct Ward {
    opt_val: Option<u32>,
    idx: Idx,
    clamp: Idx,
}

// Implementation notes:
//
// These checks enforce History usage constraints.
//
// They may be implemented/ duplicated in callees, however it's simple to add them here.

#[cfg(test)]
impl Ward {
    fn new(idx: Idx) -> Self {
        Self { opt_val: None, idx, clamp: idx + Q2 }
    }

    fn push(&mut self, item: Item) -> bool {
        if self.idx != item.idx && self.clamp != item.idx {
            return false;
        }
        if let Some(val) = self.opt_val {
            #[cfg(target_endian = "little")]
            if (val >> 8) != item.val & 0x00FF_FFFF {
                return false;
            }
            #[cfg(target_endian = "big")]
            if (val << 8) != item.val & 0xFFFF_FF00 {
                return false;
            }
        }
        self.opt_val = Some(item.val);
        self.idx += 1;
        true
    }

    fn clamp(&mut self, idx: Idx) -> bool {
        if self.idx == idx {
            self.clamp = idx + Q2;
            true
        } else {
            false
        }
    }

    fn clamp_rebias(&mut self, idx: Idx, delta: u32) -> bool {
        if self.idx == idx {
            self.idx -= delta;
            self.clamp = idx + Q2;
            true
        } else {
            false
        }
    }
}

// `a - b` >= Q3
fn is_wrapping(a: Idx, b: Idx) -> bool {
    ((a - b) as u32) >= Q3
}

#[inline(always)]
fn index<M: MatchUnit>(u: u32) -> usize {
    (M::hash_u(u) >> (32 - HASH_BITS)) as usize
}

#[cfg(test)]
mod tests {
    use crate::encode::constants::{Q0, Q2};
    use crate::encode::dummy::Dummy;

    use super::*;

    #[test]
    #[ignore = "expensive"]
    fn clamp_rebias() {
        let mut table = HistoryTable::default();
        table.reset_with_idx(Idx::Q0);
        for val in 0..Q2 {
            // Bypass Ward protection as item values are not sequential.
            table.get_mut::<Dummy>(val).push(Item::new(val, val.into()));
            table.1.idx += 1;
        }
        table.clamp(Idx::Q2);
        for history in table.0.iter() {
            for &Item { val, idx } in history.0.iter() {
                if val <= Q1 {
                    assert_eq!(idx, Idx::Q1);
                } else {
                    assert!(!is_wrapping(idx, Idx::Q3));
                    assert!((Idx::Q2 - idx) as u32 <= Q1);
                }
            }
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn clamp_rebias_q1() {
        let mut table = HistoryTable::default();
        table.reset_with_idx(Idx::Q0);
        for val in 0..Q2 {
            // Bypass Ward protection as item values are not sequential.
            table.get_mut::<Dummy>(val).push(Item::new(val, val.into()));
            table.1.idx += 1;
        }
        table.clamp_rebias(Idx::Q2, Q1);
        for history in table.0.iter() {
            for &Item { val, idx } in history.0.iter() {
                if val <= Q1 {
                    assert_eq!(idx, Idx::Q0);
                } else {
                    assert!(!is_wrapping(idx, Idx::Q3));
                    assert!((Idx::Q1 - idx) as u32 <= Q1);
                }
            }
        }
    }

    #[test]
    fn history_clamp_rebias_q0_0() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q0, 0);
        assert_eq!(history, History([Item::new(0, Idx::Q0); 4]));
    }

    #[test]
    fn history_clamp_rebias_q0_q1() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q0, Q1);
        assert_eq!(history, History([Item::new(0, Idx::Q0 - Q1); 4]));
    }

    #[test]
    fn history_clamp_rebias_q1_0() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q1, 0);
        assert_eq!(history, History([Item::new(0, Idx::Q0); 4]));
    }

    #[test]
    fn history_clamp_rebias_q1_q1() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q1, Q1);
        assert_eq!(history, History([Item::new(0, Idx::Q0 - Q1); 4]));
    }

    #[test]
    fn history_clamp_rebias_q2_0() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q2, 0);
        assert_eq!(history, History([Item::new(0, Idx::Q1); 4]));
    }

    #[test]
    fn history_clamp_rebias_q2_q1() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q2, Q1);
        assert_eq!(history, History([Item::new(0, Idx::Q1 - Q1); 4]));
    }

    #[test]
    #[should_panic]
    fn history_clamp_rebias_q3_0() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q3, 0);
    }

    #[test]
    #[should_panic]
    fn history_clamp_rebias_q3_q1() {
        let mut history = History::default();
        history.clamp_rebias(Idx::Q3, Q1);
    }

    #[test]
    #[should_panic]
    fn history_push_q0_sub_1() {
        History::default().push(Item::new(0, Idx::Q0 - 1));
    }

    #[test]
    fn history_push_q0() {
        History::default().push(Item::new(0, Idx::Q0));
    }

    #[test]
    fn history_push_q3_sub_1() {
        History::default().push(Item::new(0, Idx::Q3 - 1));
    }

    #[test]
    #[should_panic]
    fn history_push_q3() {
        History::default().push(Item::new(0, Idx::Q3));
    }

    #[test]
    fn is_wrapping_q0_sub_1() {
        (0..4).map(|u| Idx::new(u * Q1)).for_each(|idx| assert!(is_wrapping(idx - Q0 - 1, idx)));
    }

    #[test]
    fn is_wrapping_q1() {
        (0..4).map(|u| Idx::new(u * Q1)).for_each(|idx| assert!(is_wrapping(idx - Q1, idx)));
    }

    #[test]
    fn not_wrapping_q1_sub_1() {
        (0..4).map(|u| Idx::new(u * Q1)).for_each(|idx| assert!(!is_wrapping(idx - Q1 - 1, idx)));
    }

    #[test]
    fn not_is_wrapping_q0() {
        (0..4).map(|u| Idx::new(u * Q1)).for_each(|idx| assert!(!is_wrapping(idx - Q0, idx)));
    }
}
