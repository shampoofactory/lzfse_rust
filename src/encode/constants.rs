use crate::ring::{RingBlock, RingSize, RingType};

pub const GOOD_MATCH_LEN: u32 = 0x0028;

pub const RAW_CUTOFF: u32 = 0x0014;

// Conservative value.
pub const RAW_LIMIT: u32 = 0x4000;

pub const VN_CUTOFF: u32 = 0x1000;

// Fixed constants. Do NOT change.
// u32::MAX quarter bounds
#[allow(dead_code)]
pub const Q0: u32 = 0x0000_0000;
pub const Q1: u32 = 0x4000_0000;
pub const Q2: u32 = 0x8000_0000;
pub const Q3: u32 = 0xC000_0000;

#[derive(Copy, Clone, Debug)]
pub struct Input;

unsafe impl RingSize for Input {
    const RING_SIZE: u32 = 0x0008_0000;
}

unsafe impl RingType for Input {
    const RING_LIMIT: u32 = 0x0140;
}

unsafe impl RingBlock for Input {
    const RING_BLK_SIZE: u32 = 0x0000_4000;
}

#[derive(Copy, Clone, Debug)]
pub struct Output;

unsafe impl RingSize for Output {
    const RING_SIZE: u32 = 0x0002_0000;
}

unsafe impl RingType for Output {
    const RING_LIMIT: u32 = 0x0400;
}

unsafe impl RingBlock for Output {
    const RING_BLK_SIZE: u32 = 0x2000;
}
