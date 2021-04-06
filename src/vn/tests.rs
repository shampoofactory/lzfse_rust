use crate::encode::Backend;
use crate::error::Error;
use crate::lmd::MatchDistance;
use crate::ops::{PatchInto, WriteShort};
use crate::types::Idx;
use crate::{base::MagicBytes, ops::Skip};

use test_kit::{Rng, Seq};

use super::backend::BackendVn;
use super::block::VnBlock;
use super::constants::*;
use super::ops;
use super::vn_core::VnCore;

// A series of tests designed to validate VN block encoding/ decoding. Although VN opcode
// encoding/ decoding code is also stressed, this is covered comprehensively by it's own unit tests.
//
// For the most part we just brute force through huge numbers of LMD permutations, nothing
// particularly elegant although we expect it to cover the nooks and crannies.

// Encode the `lmds` as a VN block into `enc`.
fn encode_lmds(enc: &mut Vec<u8>, lmds: &[(&[u8], u32, u32)]) -> crate::Result<u32> {
    let len = lmds.iter().map(|(l, m, _)| l.len() as u32 + m).sum();
    enc.clear();
    let mut backend = BackendVn::default();
    backend.init(enc, Some(len))?;
    let mut n_raw_bytes = 0;
    for &(literals, match_len, match_distance) in lmds {
        if match_len == 0 {
            backend.push_literals(enc, literals)?;
        } else {
            let match_distance = MatchDistance::new(match_distance);
            backend.push_match(enc, literals, match_len, match_distance)?;
        }
        n_raw_bytes += literals.len() as u32 + match_len;
    }
    backend.finalize(enc)?;
    enc.write_short_u32(MagicBytes::Eos.into())?;
    Ok(n_raw_bytes)
}

// Check that `dec` corresponds to `lmds` decoded.
fn check_lmds(dec: &mut [u8], lmds: &[(&[u8], u32, u32)]) -> bool {
    let mut index = 0;
    lmds.iter().all(|&(literals, match_len, match_distance)| {
        index += literals.len();
        let copy = index - literals.len()..index;
        let match_index = index - match_distance as usize;
        let match_dst = match_index..match_index + match_len as usize;
        let match_src = index..index + match_len as usize;
        index += match_len as usize;
        literals == &dec[copy] && dec[match_dst] == dec[match_src]
    }) && index == dec.len()
}

// Encode using our backend and decode using our decode n logic.
fn check_encode_decode_n(
    enc: &mut Vec<u8>,
    dec: &mut Vec<u8>,
    lmds: &[(&[u8], u32, u32)],
    n: u32,
) -> crate::Result<bool> {
    let n_raw_bytes = encode_lmds(enc, lmds)?;
    dec.clear();
    let mut src = enc.as_slice();
    let mut block = VnBlock::default();
    src.skip(block.load(src)? as usize);
    let mut core = VnCore::from(block);
    while core.decode_n(dec, &mut src, n)? {}
    Ok(dec.len() == n_raw_bytes as usize && check_lmds(dec, lmds))
}

// Encode using our backend and decode using our decode logic.
fn check_encode_decode(
    enc: &mut Vec<u8>,
    dec: &mut Vec<u8>,
    lmds: &[(&[u8], u32, u32)],
) -> crate::Result<bool> {
    let n_raw_bytes = encode_lmds(enc, lmds)?;
    dec.clear();
    let mut src = enc.as_slice();
    ops::vn_decompress(dec, &mut src)?;
    assert_eq!(src.len(), 4);
    Ok(dec.len() == n_raw_bytes as usize && check_lmds(dec, lmds))
}

fn twin_encode_decode(
    enc: &mut Vec<u8>,
    dec: &mut Vec<u8>,
    lmds: &[(&[u8], u32, u32)],
) -> crate::Result<bool> {
    let is_ok = check_encode_decode(enc, dec, lmds)?;
    Ok(is_ok)
}

fn mutate(
    enc: &mut Vec<u8>,
    n_raw_bytes_delta: i32,
    n_payload_bytes_delta: i32,
) -> crate::Result<bool> {
    let mut block = VnBlock::default();
    block.load(enc)?;
    let n_raw_bytes = (block.n_raw_bytes() as i32 + n_raw_bytes_delta) as u32;
    let n_payload_bytes = (block.n_payload_bytes() as i32 + n_payload_bytes_delta) as u32;
    if let Ok(block) = VnBlock::new(n_raw_bytes, n_payload_bytes) {
        let bytes = enc.patch_into(Idx::default(), VN_HEADER_SIZE as usize);
        block.store(bytes);
        Ok(true)
    } else {
        Ok(false)
    }
}

// Literal encoding test.
#[test]
#[ignore = "expensive"]
fn literals() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(4096).collect::<Vec<_>>();
    // LZFSE reference chokes on empty VN payloads, so we start at 1.
    for literal_len in 1..buf.len() {
        assert!(twin_encode_decode(&mut enc, &mut dec, &[(&buf[..literal_len], 0, 0)])?);
    }
    Ok(())
}

// Small literal len, match len and match distance fine.
#[test]
#[ignore = "expensive"]
fn matches_1() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(64).collect::<Vec<_>>();
    for literal_len in 0..buf.len() {
        for match_len in 3..literal_len as u32 {
            for match_distance in 1..literal_len as u32 {
                assert!(twin_encode_decode(
                    &mut enc,
                    &mut dec,
                    &[(&buf[..literal_len], match_len, match_distance)]
                )?);
            }
        }
    }
    Ok(())
}

// Small literal len, match len and match distance fine with previous match distance.
#[allow(clippy::clippy::identity_op)]
#[test]
#[ignore = "expensive"]
fn matches_2() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(64).collect::<Vec<_>>();
    for literal_len in 3..buf.len() {
        for match_len in 3..literal_len as u32 {
            for match_distance in 1..literal_len as u32 {
                assert!(twin_encode_decode(
                    &mut enc,
                    &mut dec,
                    [
                        (&buf[..literal_len - 0], match_len, match_distance),
                        (&buf[..literal_len - 1], match_len, match_distance),
                        (&buf[..literal_len - 2], match_len, match_distance),
                        (&buf[..literal_len - 3], match_len, match_distance),
                    ]
                    .as_ref()
                )?);
            }
        }
    }
    Ok(())
}

// Match distance small fine.
#[test]
#[ignore = "expensive"]
fn matches_3() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for match_len in 3..64 {
        for match_distance in 1..=512 {
            assert!(twin_encode_decode(
                &mut enc,
                &mut dec,
                &[(&buf[..match_distance as usize], match_len, match_distance)]
            )?);
        }
    }
    Ok(())
}

// Match distance large coarse.
#[test]
#[ignore = "expensive"]
fn matches_4() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for match_len in 3..64 {
        for match_distance in (512..65536).step_by(64) {
            assert!(twin_encode_decode(
                &mut enc,
                &mut dec,
                &[(&buf[..match_distance as usize], match_len, match_distance)]
            )?);
        }
    }
    Ok(())
}

// Match distance crossover boundary.
#[test]
#[ignore = "expensive"]
fn matches_5() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for match_len in 3..64 {
        for &match_distance in &[0x05FF, 0x0600, 0x601, 0x3FFF, 0x4000, 0xFFFF] {
            assert!(twin_encode_decode(
                &mut enc,
                &mut dec,
                &[(&buf[..match_distance as usize], match_len, match_distance)]
            )?);
        }
    }
    Ok(())
}

// Match distance crossover boundary with previous match distance.
#[allow(clippy::clippy::identity_op)]
#[test]
#[ignore = "expensive"]
fn matches_6() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for match_len in 3..64 {
        for &match_distance in &[0x05FF, 0x0600, 0x601, 0x3FFF, 0x4000, 0xFFFF] {
            assert!(twin_encode_decode(
                &mut enc,
                &mut dec,
                &[
                    (&buf[..match_distance as usize - 0], match_len, match_distance),
                    (&buf[..match_distance as usize - 1], match_len, match_distance),
                    (&buf[..match_distance as usize - 2], match_len, match_distance),
                    (&buf[..match_distance as usize - 3], match_len, match_distance),
                ]
            )?);
        }
    }
    Ok(())
}

// Random LMD generation and decoding. We expect the large blocks to overflow the
// `VN_PAYLOAD_LIMIT` and stress the continuation mechanism.
#[test]
#[ignore = "expensive"]
fn rng() -> crate::Result<()> {
    let mut lmds = Vec::default();
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for i in 0..65536 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push((&buf[..1], 0, 0));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * MAX_L_VALUE as u32) >> 14;
            let m = ((rng.gen() & 0x0000_FFFF) * MAX_M_VALUE as u32) >> 14;
            let d = ((rng.gen() & 0x0000_FFFF) * MAX_D_VALUE as u32) >> 16;
            if buf.len() < index + l as usize {
                break;
            }
            let literals = &buf[index..index + l as usize];
            n_raw_bytes += l;
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push((literals, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(twin_encode_decode(&mut enc, &mut dec, &lmds)?);
    }
    Ok(())
}

// Random LMD generation and n decoding. We expect the large blocks to overflow the
// `VN_PAYLOAD_LIMIT` and stress the continuation mechanism.
#[test]
#[ignore = "expensive"]
fn rng_n() -> crate::Result<()> {
    let mut lmds = Vec::default();
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(65536).collect::<Vec<_>>();
    for i in 0..65536 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push((&buf[..1], 0, 0));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * MAX_L_VALUE as u32) >> 14;
            let m = ((rng.gen() & 0x0000_FFFF) * MAX_M_VALUE as u32) >> 14;
            let d = ((rng.gen() & 0x0000_FFFF) * MAX_D_VALUE as u32) >> 16;
            if buf.len() < index + l as usize {
                break;
            }
            let literals = &buf[index..index + l as usize];
            n_raw_bytes += l;
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push((literals, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(check_encode_decode_n(&mut enc, &mut dec, &lmds, 1 + i)?);
    }
    Ok(())
}

// Mutate `n_payload_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_1() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    // LZFSE reference chokes on empty VN payloads, so we start at 1.
    for literal_len in 1..buf.len() {
        encode_lmds(&mut enc, &[(&buf[..literal_len], 0, 0)])?;
        if !mutate(&mut enc, 0, 1)? {
            continue;
        }
        match ops::vn_decompress(&mut dec, &mut enc.as_slice()) {
            Err(Error::PayloadOverflow) => {}
            _ => panic!(),
        }
        dec.clear();
    }
    Ok(())
}

// Mutate `n_payload_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_2() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    // LZFSE reference chokes on empty VN payloads, so we start at 1.
    for literal_len in 1..buf.len() {
        encode_lmds(&mut enc, &[(&buf[..literal_len], 0, 0)])?;
        if !mutate(&mut enc, 0, -1)? {
            continue;
        }
        match ops::vn_decompress(&mut dec, &mut enc.as_slice()) {
            Err(Error::PayloadUnderflow) => {}
            _ => panic!(),
        }
        dec.clear();
    }
    Ok(())
}

// Mutate `n_raw_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_3() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    // LZFSE reference chokes on empty VN payloads, so we start at 1.
    for literal_len in 1..buf.len() {
        encode_lmds(&mut enc, &[(&buf[..literal_len], 0, 0)])?;
        if !mutate(&mut enc, 1, 0)? {
            continue;
        }
        match ops::vn_decompress(&mut dec, &mut enc.as_slice()) {
            Err(Error::Vn(super::Error::BadPayload)) => {}
            _ => panic!(),
        }
        dec.clear();
    }
    Ok(())
}

// Mutate `n_raw_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_4() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    // LZFSE reference chokes on empty VN payloads, so we start at 1.
    for literal_len in 1..buf.len() {
        encode_lmds(&mut enc, &[(&buf[..literal_len], 0, 0)])?;
        if !mutate(&mut enc, -1, 0)? {
            continue;
        }
        match ops::vn_decompress(&mut dec, &mut enc.as_slice()) {
            Err(Error::Vn(super::Error::BadPayload)) => {}
            _ => panic!(),
        }
        dec.clear();
    }
    Ok(())
}

// Random payload generation with mutations. We are looking to break the decoder. In all cases the
// decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
// segfault/ panic/ trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_rng() -> crate::Result<()> {
    let mut lmds = Vec::default();
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(4096).collect::<Vec<_>>();
    for i in 0..4096 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push((&buf[..1], 0, 0));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * MAX_L_VALUE as u32) >> 15;
            let m = ((rng.gen() & 0x0000_FFFF) * MAX_M_VALUE as u32) >> 15;
            let d = ((rng.gen() & 0x0000_FFFF) * MAX_D_VALUE as u32) >> 16;
            if buf.len() < index + l as usize {
                break;
            }
            let literals = &buf[index..index + l as usize];
            n_raw_bytes += l;
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push((literals, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        encode_lmds(&mut enc, &lmds)?;
        for j in 4..enc.len() {
            let n = enc[j];
            enc[j] = enc[j].wrapping_add(1);
            dec.clear();
            let _ = ops::vn_decompress(&mut dec, &mut enc.as_slice());
            enc[j] = enc[j].wrapping_sub(2);
            dec.clear();
            let _ = ops::vn_decompress(&mut dec, &mut enc.as_slice());
            enc[j] = n;
        }
    }
    Ok(())
}

// Random payload generation with mutations. We are looking to break the decoder. In all cases the
// decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
// segfault/ panic/ trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_rng_2() -> crate::Result<()> {
    let mut lmds = Vec::default();
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    let buf = Seq::default().take(4096).collect::<Vec<_>>();
    for i in 0..32 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push((&buf[..1], 0, 0));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * MAX_L_VALUE as u32) >> 15;
            let m = ((rng.gen() & 0x0000_FFFF) * MAX_M_VALUE as u32) >> 15;
            let d = ((rng.gen() & 0x0000_FFFF) * MAX_D_VALUE as u32) >> 16;
            if buf.len() < index + l as usize {
                break;
            }
            let literals = &buf[index..index + l as usize];
            n_raw_bytes += l;
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push((literals, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        encode_lmds(&mut enc, &lmds)?;
        for _ in 0..255 {
            for j in 4..enc.len() {
                enc[j] = enc[j].wrapping_add(1);
                dec.clear();
                let _ = ops::vn_decompress(&mut dec, &mut enc.as_slice());
            }
        }
    }
    Ok(())
}

// Random noise for payload. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn fuzz_noise() -> crate::Result<()> {
    let mut enc = Vec::default();
    let mut dec = Vec::default();
    for i in 0..1024 * 65536 {
        let mut seq = Seq::new(Rng::new(i));
        enc.clear();
        enc.write_short_u32(MagicBytes::Vxn.into())?;
        for _ in 0..1024 {
            enc.push(seq.next().unwrap());
        }
        let _ = ops::vn_decompress(&mut dec, &mut enc.as_slice());
        dec.clear();
    }
    Ok(())
}
