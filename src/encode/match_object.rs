use crate::types::Idx;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Match {
    pub idx: Idx,
    pub match_idx: Idx,
    pub match_len: u32,
}

impl Match {
    #[inline(always)]
    pub fn select<const T: u32>(&mut self, incoming: Match) -> Option<Match> {
        let select;
        if incoming.match_len == 0 {
            select = None;
        } else if incoming.match_len >= T {
            select = Some(incoming);
            self.match_len = 0;
        } else if self.match_len == 0 {
            select = None;
            *self = incoming;
        } else if self.idx + self.match_len <= incoming.idx {
            select = Some(*self);
            *self = incoming;
        } else if incoming.match_len > self.match_len {
            select = Some(incoming);
            self.match_len = 0;
        } else {
            select = Some(*self);
            self.match_len = 0;
        }
        select
    }
}
