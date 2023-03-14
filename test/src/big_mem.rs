// FrontendBytes big memory tests.
macro_rules! test_pattern_rng {
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

#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_512mb, ops::encode, 0x2000_0000);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_7FFF_FFFD, ops::encode, 0x7FFF_FFFD);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_7FFF_FFFE, ops::encode, 0x7FFF_FFFE);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_7FFF_FFFF, ops::encode, 0x7FFF_FFFF);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_8000_0000, ops::encode, 0x8000_0000);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_8000_0001, ops::encode, 0x8000_0001);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_8000_0002, ops::encode, 0x8000_0002);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_8000_0003, ops::encode, 0x8000_0003);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_8000_0004, ops::encode, 0x8000_0004);
#[cfg(target_pointer_width = "64")]
test_pattern_rng!(big_rng_2_0000_0000, ops::encode, 0x2_0000_0000);

macro_rules! test_pattern_zeros {
    ($name:ident, $encoder:expr, $len:expr) => {
        mod $name {
            use lzfse_rust::{LzfseDecoder, LzfseEncoder};

            use std::io;

            #[test]
            fn zeros() -> io::Result<()> {
                let mut dec = Vec::default();
                {
                    let mut enc = Vec::with_capacity($len / 4);
                    {
                        let src = vec![0; $len];
                        LzfseEncoder::default().encode_bytes(&src, &mut enc)?;
                        // src drops, free src memory
                    }
                    LzfseDecoder::default().decode_bytes(&enc, &mut dec)?;
                    // enc drops, free enc memory
                }
                assert_eq!(dec.len(), $len);
                for b in dec {
                    assert_eq!(b, 0);
                }
                Ok(())
            }
        }
    };
}

// TODO should be consistent with rng
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_512mb, ops::encode, 0x2000_0000);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_7FFF_FFFD, ops::encode, 0x7FFF_FFFD);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_7FFF_FFFE, ops::encode, 0x7FFF_FFFE);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_7FFF_FFFF, ops::encode, 0x7FFF_FFFF);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_8000_0000, ops::encode, 0x8000_0000);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_8000_0001, ops::encode, 0x8000_0001);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_8000_0002, ops::encode, 0x8000_0002);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_8000_0003, ops::encode, 0x8000_0003);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_8000_0004, ops::encode, 0x8000_0004);
#[cfg(target_pointer_width = "64")]
test_pattern_zeros!(big_zeros_2_0000_0000, ops::encode, 0x2_0000_0000);
