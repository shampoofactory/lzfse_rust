/// Zero the most significant `n_bits` of `lhs`.
/// Results when `n_bits >= size_of::<usize>()` are undefined.
#[cfg(target_feature = "bmi2")]
#[inline(always)]
pub fn mask(lhs: usize, n_bits: usize) -> usize {
    // Leverage BMI2 BZHI instructions.
    mask_shift(lhs, n_bits)
}

#[cfg(not(target_feature = "bmi2"))]
#[inline(always)]
pub fn mask(lhs: usize, n_bits: usize) -> usize {
    // Avoid slow x86/ x64 SHL instructions using a lookup table.
    // Assumes that the function is invoked with sufficient regularity to maintain table data
    // entries in L1 cache.
    mask_table(lhs, n_bits)
}

#[allow(dead_code)]
#[inline(always)]
fn mask_table(lhs: usize, n_bits: usize) -> usize {
    lhs & MASK_TABLE[n_bits & (MASK_TABLE_LEN - 1)]
}

#[allow(dead_code)]
#[inline(always)]
fn mask_shift(lhs: usize, n_bits: usize) -> usize {
    // TODO consider `unchecked_shl` when stable.
    lhs & ((1 << n_bits) - 1)
}

#[cfg(target_pointer_width = "64")]
pub const MASK_TABLE_LEN: usize = 64;

#[cfg(target_pointer_width = "64")]
pub const MASK_TABLE: [usize; MASK_TABLE_LEN] = [
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0001,
    0x0000_0000_0000_0003,
    0x0000_0000_0000_0007,
    0x0000_0000_0000_000F,
    0x0000_0000_0000_001F,
    0x0000_0000_0000_003F,
    0x0000_0000_0000_007F,
    0x0000_0000_0000_00FF,
    0x0000_0000_0000_01FF,
    0x0000_0000_0000_03FF,
    0x0000_0000_0000_07FF,
    0x0000_0000_0000_0FFF,
    0x0000_0000_0000_1FFF,
    0x0000_0000_0000_3FFF,
    0x0000_0000_0000_7FFF,
    0x0000_0000_0000_FFFF,
    0x0000_0000_0001_FFFF,
    0x0000_0000_0003_FFFF,
    0x0000_0000_0007_FFFF,
    0x0000_0000_000F_FFFF,
    0x0000_0000_001F_FFFF,
    0x0000_0000_003F_FFFF,
    0x0000_0000_007F_FFFF,
    0x0000_0000_00FF_FFFF,
    0x0000_0000_01FF_FFFF,
    0x0000_0000_03FF_FFFF,
    0x0000_0000_07FF_FFFF,
    0x0000_0000_0FFF_FFFF,
    0x0000_0000_1FFF_FFFF,
    0x0000_0000_3FFF_FFFF,
    0x0000_0000_7FFF_FFFF,
    0x0000_0000_FFFF_FFFF,
    0x0000_0001_FFFF_FFFF,
    0x0000_0003_FFFF_FFFF,
    0x0000_0007_FFFF_FFFF,
    0x0000_000F_FFFF_FFFF,
    0x0000_001F_FFFF_FFFF,
    0x0000_003F_FFFF_FFFF,
    0x0000_007F_FFFF_FFFF,
    0x0000_00FF_FFFF_FFFF,
    0x0000_01FF_FFFF_FFFF,
    0x0000_03FF_FFFF_FFFF,
    0x0000_07FF_FFFF_FFFF,
    0x0000_0FFF_FFFF_FFFF,
    0x0000_1FFF_FFFF_FFFF,
    0x0000_3FFF_FFFF_FFFF,
    0x0000_7FFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
    0x0001_FFFF_FFFF_FFFF,
    0x0003_FFFF_FFFF_FFFF,
    0x0007_FFFF_FFFF_FFFF,
    0x000F_FFFF_FFFF_FFFF,
    0x001F_FFFF_FFFF_FFFF,
    0x003F_FFFF_FFFF_FFFF,
    0x007F_FFFF_FFFF_FFFF,
    0x00FF_FFFF_FFFF_FFFF,
    0x01FF_FFFF_FFFF_FFFF,
    0x03FF_FFFF_FFFF_FFFF,
    0x07FF_FFFF_FFFF_FFFF,
    0x0FFF_FFFF_FFFF_FFFF,
    0x1FFF_FFFF_FFFF_FFFF,
    0x3FFF_FFFF_FFFF_FFFF,
    0x7FFF_FFFF_FFFF_FFFF,
];

#[cfg(target_pointer_width = "32")]
pub const MASK_TABLE_LEN: usize = 32;

#[cfg(target_pointer_width = "32")]
pub const MASK_TABLE: [usize; MASK_TABLE_LEN] = [
    0x0000_0000,
    0x0000_0001,
    0x0000_0003,
    0x0000_0007,
    0x0000_000F,
    0x0000_001F,
    0x0000_003F,
    0x0000_007F,
    0x0000_00FF,
    0x0000_01FF,
    0x0000_03FF,
    0x0000_07FF,
    0x0000_0FFF,
    0x0000_1FFF,
    0x0000_3FFF,
    0x0000_7FFF,
    0x0000_FFFF,
    0x0001_FFFF,
    0x0003_FFFF,
    0x0007_FFFF,
    0x000F_FFFF,
    0x001F_FFFF,
    0x003F_FFFF,
    0x007F_FFFF,
    0x00FF_FFFF,
    0x01FF_FFFF,
    0x03FF_FFFF,
    0x07FF_FFFF,
    0x0FFF_FFFF,
    0x1FFF_FFFF,
    0x3FFF_FFFF,
    0x7FFF_FFFF,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn bit_mask() {
        let lhs = (-1isize) as usize;
        for n_bits in 0..mem::size_of::<usize>() * 8 {
            assert_eq!(mask_shift(lhs, n_bits), mask_table(lhs, n_bits));
        }
    }
}
