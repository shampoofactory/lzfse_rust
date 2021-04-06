// No matching 4 byte sequences.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use test_kit::Useq;

            use crate::ops;

            use std::io;

            #[test]
            fn encode_decode_0() -> io::Result<()> {
                let vec = Useq::default().take(0x0010_0000).collect::<Vec<_>>();
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }
        }
    };
}

test_pattern!(encode, ops::encode);
test_pattern!(encode_ring, ops::encode_ring);
test_pattern!(encode_writer, ops::encode_ring_writer_bytes);
test_pattern!(encode_ring_writer, ops::encode_ring_writer);
