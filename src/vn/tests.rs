use crate::error::Error;
use crate::lmd::{self, Lmd};
use crate::ops::{PatchInto, WriteShort};
use crate::test_utils;
use crate::types::Idx;
use crate::{base::MagicBytes, ops::Skip};

use test_kit::{Rng, Seq};

use super::backend::BackendVn;
use super::block::VnBlock;
use super::constants::*;
use super::object::Vn;
use super::vn_core::VnCore;

use std::io;

// A series of tests designed to validate VN block encoding/ decoding. Although VN opcode
// encoding/ decoding code is also stressed, this is covered comprehensively by it's own unit tests.

struct Monkey {
    backend: BackendVn,
    enc: Vec<u8>,
    dec: Vec<u8>,
}

impl Monkey {
    fn encode_lmds(&mut self, literals: &[u8], lmds: &[Lmd<Vn>]) -> io::Result<(u32, u32)> {
        test_utils::encode_lmds(&mut self.enc, &mut self.backend, literals, lmds)
    }

    fn decode(&mut self) -> crate::Result<(u32, u32)> {
        self.dec.clear();
        let mut src = self.enc.as_slice();
        let mut block = VnBlock::default();
        let n_header_bytes = block.load(src)?;
        src.skip(n_header_bytes as usize);
        let mut core = VnCore::from(block);
        let n_payload_bytes = core.decode(&mut self.dec, &mut src)?;
        let n_raw_bytes = self.dec.len() as u32;
        Ok((n_header_bytes + n_payload_bytes, n_raw_bytes))
    }

    fn decode_n(&mut self, n: u32) -> crate::Result<(u32, u32)> {
        self.dec.clear();
        let mut src = self.enc.as_slice();
        let mut block = VnBlock::default();
        let n_header_bytes = block.load(src)?;
        src.skip(n_header_bytes as usize);
        let mut core = VnCore::from(block);
        while core.decode_n(&mut self.dec, &mut src, n)? {}
        let n_payload_bytes = self.enc.len() as u32;
        let n_raw_bytes = self.dec.len() as u32;
        Ok((n_payload_bytes, n_raw_bytes))
    }

    fn check_encode_decode(&mut self, literals: &[u8], lmds: &[Lmd<Vn>]) -> crate::Result<bool> {
        let (e_raw_bytes, e_payload_bytes) = self.encode_lmds(literals, lmds)?;
        let (d_payload_bytes, d_raw_bytes) = self.decode()?;
        Ok(e_raw_bytes == d_raw_bytes
            && e_payload_bytes == d_payload_bytes
            && self.check_lmds(literals, lmds))
    }

    fn check_encode_decode_n(
        &mut self,
        literals: &[u8],
        lmds: &[Lmd<Vn>],
        n: u32,
    ) -> crate::Result<bool> {
        let (e_raw_bytes, e_payload_bytes) = self.encode_lmds(literals, lmds)?;
        let (d_payload_bytes, d_raw_bytes) = self.decode_n(n)?;
        Ok(e_raw_bytes == d_raw_bytes
            && e_payload_bytes == d_payload_bytes
            && self.check_lmds(literals, lmds))
    }

    fn mutate(
        &mut self,
        n_raw_bytes_delta: i32,
        n_payload_bytes_delta: i32,
    ) -> crate::Result<bool> {
        let mut block = VnBlock::default();
        block.load(&self.enc)?;
        let n_raw_bytes = (block.n_raw_bytes() as i32 + n_raw_bytes_delta) as u32;
        let n_payload_bytes = (block.n_payload_bytes() as i32 + n_payload_bytes_delta) as u32;
        if let Ok(block) = VnBlock::new(n_raw_bytes, n_payload_bytes) {
            let bytes = self.enc.patch_into(Idx::default(), VN_HEADER_SIZE as usize);
            block.store(bytes);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn check_lmds(&self, literals: &[u8], lmds: &[Lmd<Vn>]) -> bool {
        test_utils::check_lmds(&self.dec, literals, lmds)
    }
}

impl Default for Monkey {
    fn default() -> Self {
        Self { backend: BackendVn::default(), enc: Vec::default(), dec: Vec::default() }
    }
}

// Literal encoding test.
#[test]
#[ignore = "expensive"]
fn literals() -> crate::Result<()> {
    let bytes = Seq::default().take(0x1_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        assert!(monkey.check_encode_decode(&bytes[..literal_len], &lmds)?);
    }
    Ok(())
}

// Small literal len, match len and match distance.
#[test]
#[ignore = "expensive"]
fn matches_1() -> crate::Result<()> {
    let bytes = Seq::default().take(0x0100).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        for match_len in 3..literal_len as u32 {
            for match_distance in 1..literal_len as u32 {
                lmds.clear();
                lmd::split_lmd(&mut lmds, literal_len as u32, match_len, match_distance);
                assert!(monkey.check_encode_decode(&bytes[..literal_len], &lmds)?);
            }
        }
    }
    Ok(())
}

// Small match len, all match distance.
#[test]
#[ignore = "expensive"]
fn matches_2() -> crate::Result<()> {
    let bytes = Seq::default().take(0x1_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for match_len in 3..0x40 {
        for match_distance in 1..=0xFFFF {
            let literal_len = match_distance;
            lmds.clear();
            lmd::split_lmd(&mut lmds, literal_len, match_len, match_distance);
            assert!(monkey.check_encode_decode(&bytes[..literal_len as usize], &lmds)?);
        }
    }
    Ok(())
}

// Coarse match len, all match distance.
#[test]
#[ignore = "expensive"]
fn matches_3() -> crate::Result<()> {
    let bytes = Seq::default().take(0x1_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for match_len in (0x40..0x400).step_by(0x10) {
        for match_distance in 1..=0xFFFF {
            let literal_len = match_distance;
            lmds.clear();
            lmd::split_lmd(&mut lmds, literal_len, match_len, match_distance);
            assert!(monkey.check_encode_decode(&bytes[..literal_len as usize], &lmds)?);
        }
    }
    Ok(())
}

// Random LMD generation.
#[test]
#[ignore = "expensive"]
fn fuzz_lmd() -> crate::Result<()> {
    let bytes = Seq::default().take(0x8_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_FFFF) * (MAX_D_VALUE as u32 + 1)) >> 16;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(monkey.check_encode_decode(&bytes[..index as usize], &lmds)?);
    }
    Ok(())
}

// Random LMD generation with n decoding.
#[test]
#[ignore = "expensive"]
fn fuzz_lmd_n() -> crate::Result<()> {
    let bytes = Seq::default().take(0x8_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_FFFF) * (MAX_D_VALUE as u32 + 1)) >> 16;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(monkey.check_encode_decode_n(&bytes[..index as usize], &lmds, 1 + i)?);
    }
    Ok(())
}

// Mutate `n_payload_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_1() -> crate::Result<()> {
    let bytes = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 0..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        monkey.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !monkey.mutate(0, 1)? {
            continue;
        }
        match monkey.decode() {
            Err(Error::PayloadOverflow) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Mutate `n_payload_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_2() -> crate::Result<()> {
    let bytes = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 0..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        monkey.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !monkey.mutate(0, -1)? {
            continue;
        }
        match monkey.decode() {
            Err(Error::PayloadUnderflow) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Mutate `n_raw_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_3() -> crate::Result<()> {
    let bytes = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 0..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        monkey.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !monkey.mutate(1, 0)? {
            continue;
        }
        match monkey.decode() {
            Err(Error::Vn(super::VnErrorKind::BadPayload)) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Mutate `n_raw_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_block_4() -> crate::Result<()> {
    let bytes = Seq::default().take(VN_PAYLOAD_LIMIT as usize * 2).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for literal_len in 0..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        monkey.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !monkey.mutate(-1, 0)? {
            continue;
        }
        match monkey.decode() {
            Err(Error::Vn(super::VnErrorKind::BadPayload)) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Random payload generation with mutations. We are looking to break the decoder. In all cases the
// decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
// segfault/ panic/ trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_rng_1() -> crate::Result<()> {
    let bytes = Seq::default().take(0x0004_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_FFFF) * (MAX_D_VALUE as u32 + 1)) >> 16;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        monkey.encode_lmds(&bytes[..index as usize], &lmds)?;
        for j in 4..monkey.enc.len() {
            monkey.enc[j] = monkey.enc[j].wrapping_add(1);
            let _ = monkey.decode();
            monkey.enc[j] = monkey.enc[j].wrapping_sub(2);
            let _ = monkey.decode();
            monkey.enc[j] = monkey.enc[j].wrapping_add(1);
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
    let bytes = Seq::default().take(0x0004_0000).collect::<Vec<_>>();
    let mut monkey = Monkey::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_FFFF) * (MAX_D_VALUE as u32 + 1)) >> 16;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        monkey.encode_lmds(&bytes[..index as usize], &lmds)?;
        for _ in 0..255 {
            for j in 4..monkey.enc.len() {
                monkey.enc[j] = monkey.enc[j].wrapping_add(1);
                let _ = monkey.decode();
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
    let mut monkey = Monkey::default();
    for i in 0..0x0100_0000 {
        let mut seq = Seq::new(Rng::new(i));
        monkey.enc.write_short_u32(MagicBytes::Vxn.into())?;
        monkey.enc.resize(0x400, 0);
        for i in 0x0004..0x0400 {
            monkey.enc[i] = seq.next().unwrap();
        }
        let _ = monkey.decode();
    }
    Ok(())
}
