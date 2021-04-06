use crate::ring::{RingBlock, RingSize, RingType};

#[derive(Copy, Clone, Debug)]
pub struct Input;

unsafe impl RingSize for Input {
    const RING_SIZE: u32 = 0x0002_0000;
}

unsafe impl RingType for Input {
    const RING_LIMIT: u32 = 0x02D4;
}

unsafe impl RingBlock for Input {
    const RING_BLK_SIZE: u32 = 0x2000;
}

#[derive(Copy, Clone, Debug)]
pub struct Output;

unsafe impl RingSize for Output {
    const RING_SIZE: u32 = 0x0008_0000;
}

unsafe impl RingType for Output {
    const RING_LIMIT: u32 = 0x0940;
}

unsafe impl RingBlock for Output {
    const RING_BLK_SIZE: u32 = 0x0001_0000;
}
