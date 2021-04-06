// Basic repeating sequences.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use crate::ops;

            use std::io;

            fn gen(u: u32, v: u8) -> Vec<u8> {
                let mut vec = Vec::with_capacity(u as usize);
                for i in 0..u {
                    vec.push((i % v as u32) as u8);
                }
                vec
            }

            #[test]
            fn encode_decode_2() -> io::Result<()> {
                let vec = gen(0x0010_0000, 2);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_3() -> io::Result<()> {
                let vec = gen(0x0010_0000, 3);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_4() -> io::Result<()> {
                let vec = gen(0x0010_0000, 4);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_5() -> io::Result<()> {
                let vec = gen(0x0010_0000, 5);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_6() -> io::Result<()> {
                let vec = gen(0x0010_0000, 6);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_7() -> io::Result<()> {
                let vec = gen(0x0010_0000, 7);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_8() -> io::Result<()> {
                let vec = gen(0x0010_0000, 8);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_9() -> io::Result<()> {
                let vec = gen(0x0010_0000, 9);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_10() -> io::Result<()> {
                let vec = gen(0x0010_0000, 10);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_11() -> io::Result<()> {
                let vec = gen(0x0010_0000, 11);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_12() -> io::Result<()> {
                let vec = gen(0x0010_0000, 12);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_13() -> io::Result<()> {
                let vec = gen(0x0010_0000, 13);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_14() -> io::Result<()> {
                let vec = gen(0x0010_0000, 14);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_15() -> io::Result<()> {
                let vec = gen(0x0010_0000, 15);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_16() -> io::Result<()> {
                let vec = gen(0x0010_0000, 16);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_32() -> io::Result<()> {
                let vec = gen(0x0010_0000, 32);
                ops::check_encode_decode(&vec, $encoder)?;
                Ok(())
            }

            #[test]
            fn encode_decode_64() -> io::Result<()> {
                let vec = gen(0x0010_0000, 64);
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
