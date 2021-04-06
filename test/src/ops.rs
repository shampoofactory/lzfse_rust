use lzfse_rust::{LzfseDecoder, LzfseEncoder, LzfseRingDecoder, LzfseRingEncoder};
use sha2::{Digest, Sha256};

use std::io::{self, Read, Write};
use std::mem;

const DECODE_F: usize = 4;
const ENCODE_F: usize = 1;

pub fn check_decode_mutate<F>(enc: &[u8], decode: F, dst: &mut Vec<u8>) -> io::Result<()>
where
    F: Fn(&[u8], &mut Vec<u8>) -> io::Result<()>,
{
    dst.clear();
    decode(enc, dst)
}

pub fn check_decode_hash<F>(enc: &[u8], hash: &[u8], decode: F) -> io::Result<()>
where
    F: Fn(&[u8], &mut Vec<u8>) -> io::Result<()>,
{
    let mut dec = Vec::with_capacity(enc.len() * DECODE_F);
    decode(enc, &mut dec)?;
    let mut hasher = Sha256::default();
    hasher.update(dec);
    assert_eq!(hasher.finalize().as_slice(), hash);
    Ok(())
}

pub fn check_encode_decode<F>(data: &[u8], encode: F) -> io::Result<()>
where
    F: Fn(&[u8], &mut Vec<u8>) -> io::Result<()>,
{
    let mut enc = Vec::with_capacity(data.len() * ENCODE_F);
    encode(data, &mut enc)?;
    let mut dec = Vec::with_capacity(data.len());
    decode(&enc, &mut dec)?;
    assert!(data == dec);
    #[cfg(feature = "lzfse_ref")]
    {
        dec.clear();
        decode_lzfse(&enc, &mut dec)?;
        assert!(data == dec);
        enc.clear();
        encode_lzfse(data, &mut enc)?;
        dec.clear();
        decode(&enc, &mut dec)?;
        assert!(data == dec);
    }
    Ok(())
}

pub fn check_decode_encode_decode<F>(enc: &[u8], encode: F) -> io::Result<()>
where
    F: Fn(&[u8], &mut Vec<u8>) -> io::Result<()>,
{
    let mut data = Vec::with_capacity(enc.len() * DECODE_F);
    decode(enc, &mut data)?;
    let mut enc = Vec::with_capacity(enc.len());
    encode(&data, &mut enc)?;
    let mut dec = Vec::with_capacity(data.len());
    decode(&enc, &mut dec)?;
    assert!(data == dec);
    #[cfg(feature = "lzfse_ref")]
    {
        dec.clear();
        decode_lzfse(&enc, &mut dec)?;
        assert!(data == dec);
        enc.clear();
        encode_lzfse(&data, &mut enc)?;
        dec.clear();
        decode(&enc, &mut dec)?;
        assert!(data == dec);
    }
    Ok(())
}

pub fn decode(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    LzfseDecoder::default().decode_bytes(src, dst)?;
    Ok(())
}

pub fn decode_ring(mut src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    LzfseRingDecoder::default().decode(&mut src, dst)?;
    Ok(())
}

pub fn decode_ring_reader(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let mut decoder = LzfseRingDecoder::default();
    let mut rdr = decoder.reader(src);
    let mut byte = [0u8];
    while rdr.read(&mut byte)? != 0 {
        dst.push(byte[0]);
    }
    Ok(())
}

pub fn decode_ring_reader_bytes(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let mut decoder = LzfseRingDecoder::default();
    let mut rdr = decoder.reader_bytes(src);
    let mut byte = [0u8];
    while rdr.read(&mut byte)? != 0 {
        dst.push(byte[0]);
    }
    Ok(())
}

pub fn encode(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    LzfseEncoder::default().encode_bytes(src, dst)?;
    Ok(())
}

pub fn encode_ring(mut src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    LzfseRingEncoder::default().encode(&mut src, dst)?;
    Ok(())
}

pub fn encode_ring_writer(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let mut encoder = LzfseRingEncoder::default();
    let mut wtr = encoder.writer(dst);
    for &b in src.iter() {
        wtr.write_all(&[b])?;
    }
    wtr.finalize()?;
    Ok(())
}

pub fn encode_ring_writer_bytes(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let mut encoder = LzfseRingEncoder::default();
    let t = mem::take(dst);
    let mut wtr = encoder.writer_bytes(t);
    for &b in src.iter() {
        wtr.write_all(&[b])?;
    }
    let t = wtr.finalize()?;
    *dst = t;
    Ok(())
}

#[cfg(feature = "lzfse_ref")]
pub fn decode_lzfse(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    lzfse_sys::decode_all(src, dst);
    Ok(())
}

#[cfg(feature = "lzfse_ref")]
pub fn encode_lzfse(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    lzfse_sys::encode_all(src, dst);
    Ok(())
}
