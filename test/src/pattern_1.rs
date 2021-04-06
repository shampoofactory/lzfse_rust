// Empty/ zero byte files, increasing in size.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {

            use crate::ops;

            use std::io;

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x8000);
                while vec.len() != 0x8000 {
                    ops::check_encode_decode(&vec, $encoder)?;
                    vec.push(0);
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_1() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x0010_0000);
                while vec.len() != 0x0008_0200 {
                    ops::check_encode_decode(&vec, $encoder)?;
                    vec.extend_from_slice(&[0u8; 0x100]);
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
