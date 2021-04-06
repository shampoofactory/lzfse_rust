// Random short repeating sequences.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use test_kit::{Rng, Seq};

            use crate::ops;

            use std::io;

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let literals = Seq::default().take(0x4000).collect::<Vec<_>>();
                let mut data = Vec::default();
                for seed in 0..0x10 {
                    data.clear();
                    let mut rng = Rng::new(seed);
                    let mut literals = literals.as_slice();
                    while literals.len() != 0 {
                        let l = rng.gen() as usize % 0x20 + 1;
                        let l = l.min(literals.len());
                        data.extend_from_slice(&literals[..l]);
                        literals = &literals[l..];
                        let m = rng.gen() as usize % 0x0100;
                        for _ in 0..m {
                            let b = data[data.len() - l];
                            data.push(b);
                        }
                        ops::check_encode_decode(&data, $encoder)?;
                    }
                }
                Ok(())
            }
        }
    };
}

test_pattern!(encode, ops::encode);
test_pattern!(encode_ring, ops::encode_ring);
test_pattern!(encode_writer, ops::encode_ring_writer_bytes);
test_pattern!(encode_ring_writer, ops::encode_ring_writer);
