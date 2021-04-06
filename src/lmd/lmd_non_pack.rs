use super::lmd_type::LmdMax;
use super::lmd_type::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Lmd<T: LmdMax>(pub LiteralLen<T>, pub MatchLen<T>, pub MatchDistance<T>);

impl<T: LmdMax> Lmd<T> {
    #[allow(dead_code)]
    pub fn new(literal_len: u32, match_len: u32, match_distance: u32) -> Self {
        Self(
            LiteralLen::new(literal_len),
            MatchLen::new(match_len),
            MatchDistance::new(match_distance),
        )
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn new_unchecked(literal_len: u32, match_len: u32, match_distance: u32) -> Self {
        Self(
            LiteralLen::new_unchecked(literal_len),
            MatchLen::new_unchecked(match_len),
            MatchDistance::new_unchecked(match_distance),
        )
    }
}

impl<T: LmdMax> Default for Lmd<T> {
    #[inline(always)]
    fn default() -> Self {
        Self(LiteralLen::default(), MatchLen::default(), MatchDistance::default())
    }
}
