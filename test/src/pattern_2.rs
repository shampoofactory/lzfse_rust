// Nonoverlapping matches, decreasing in size.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use test_kit::Seq;

            use crate::ops;

            use std::io;

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x0008_0200);
                Seq::default().take(0x0400).for_each(|u| vec.push(u));
                for u in (1..0x0400).rev() {
                    let i = vec.len();
                    vec.resize(i + u, 0);
                    vec.copy_within(i - u..i, i);
                    ops::check_encode_decode(&vec, $encoder)?;
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
