// FrontendBytes big memory tests.
macro_rules! test_pattern {
    ($name:ident, $encoder:expr, $len:expr) => {
        mod $name {
            use lzfse_rust::{LzfseDecoder, LzfseEncoder};
            use test_kit::Rng;

            use std::io;

            #[test]
            fn rng() -> io::Result<()> {
                let mut dec = Vec::default();
                {
                    let mut enc = Vec::with_capacity($len + ($len / 4));
                    {
                        let src = Rng::default().gen_vec($len).unwrap();
                        LzfseEncoder::default().encode_bytes(&src, &mut enc)?;
                        // src drops, free src memory
                    }
                    LzfseDecoder::default().decode_bytes(&enc, &mut dec)?;
                    // enc drops, free enc memory
                }
                assert_eq!(dec.len(), $len);
                assert!(Rng::default().check_bytes(&dec));
                Ok(())
            }
        }
    };
}

test_pattern!(big_2gb_sub_1, ops::encode, 0x7FFF_FFFF);
test_pattern!(big_2gb, ops::encode, 0x8000_0000);
#[cfg(target_pointer_width = "64")]
test_pattern!(big_8gb, ops::encode, 0x2_0000_0000);
