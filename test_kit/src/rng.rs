use std::convert::TryInto;
use std::mem;

/// Deterministic random number generator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    #[inline(always)]
    pub fn gen(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }

    pub fn check_bytes(&mut self, bytes: &[u8]) -> bool {
        let mut index = 0;
        while index <= bytes.len() - 4 {
            let src: [u8; 4] = (bytes[index..index + 4]).try_into().unwrap();
            let u = u32::from_ne_bytes(src);
            if u != self.0 {
                return false;
            }
            self.gen();
            index += 4;
        }
        let mut u = self.0;
        while index < bytes.len() {
            if bytes[index] != u as u8 {
                return false;
            }
            u >>= 8;
            index += 1;
        }
        true
    }

    pub fn gen_vec(&mut self, len: usize) -> Option<Vec<u8>> {
        let mut vec = Vec::default();
        match vec.try_reserve_exact(len) {
            Ok(()) => {
                for _ in 0..(len / mem::size_of::<u32>()) {
                    vec.extend_from_slice(&self.0.to_ne_bytes());
                    self.gen();
                }
                let mut u = self.0;
                for _ in 0..(len % mem::size_of::<u32>()) {
                    vec.push(u as u8);
                    u >>= 8;
                }
                Some(vec)
            }
            Err(_) => None,
        }
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Iterator for Rng {
    type Item = u32;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.gen())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bytes() {
        let mut rng = Rng::default();
        let bytes = rng.gen_vec(1023).unwrap();
        let mut rng = Rng::default();
        assert!(rng.check_bytes(&bytes));
    }
}
