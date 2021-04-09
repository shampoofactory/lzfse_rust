use lzfse_rust::{LzfseRingDecoder, LzfseRingEncoder};
use sha2::{Digest, Sha256};

#[cfg(feature = "lzfse_ref")]
use super::ops;

use std::io;

// Decode output estimation
const DECODE_F: usize = 4;

// Encode output estimation
const ENCODE_F: usize = 1;

pub struct Monkey {
    decoder: LzfseRingDecoder,
    encoder: LzfseRingEncoder,
    data: Vec<u8>,
    enc: Vec<u8>,
    dec: Vec<u8>,
}

impl Monkey {
    pub fn blind_decode<F>(&mut self, enc: &[u8], decode: F) -> io::Result<()>
    where
        F: Fn(&mut LzfseRingDecoder, &[u8], &mut Vec<u8>) -> io::Result<()>,
    {
        // Decode enc > self.data
        self.data.clear();
        self.data.reserve(enc.len() * DECODE_F);
        decode(&mut self.decoder, enc, &mut self.data)
    }

    pub fn decode_hash<F>(&mut self, enc: &[u8], hash: &[u8], decode: F) -> io::Result<()>
    where
        F: Fn(&mut LzfseRingDecoder, &[u8], &mut Vec<u8>) -> io::Result<()>,
    {
        // Decode enc -> self.data
        self.data.clear();
        self.data.reserve(enc.len() * DECODE_F);
        decode(&mut self.decoder, enc, &mut self.data)?;
        // Hash self.data
        let mut hasher = Sha256::default();
        hasher.update(&self.data);
        // Validate
        assert_eq!(hasher.finalize().as_slice(), hash);
        Ok(())
    }

    pub fn encode_decode<F, U>(&mut self, data: &[u8], encode: F) -> io::Result<()>
    where
        F: Fn(&mut LzfseRingEncoder, &[u8], &mut Vec<u8>) -> io::Result<U>,
    {
        // Encode data -> self.enc
        self.enc.clear();
        self.enc.reserve(data.len() * ENCODE_F);
        encode(&mut self.encoder, data, &mut self.enc)?;
        // Decode self.enc -> self.dec
        self.dec.clear();
        self.dec.reserve(self.data.len());
        self.decoder.decode(&mut self.enc.as_slice(), &mut self.dec)?;
        // Validate
        assert!(data == self.dec);
        #[cfg(feature = "lzfse_ref")]
        {
            // Decode self.enc -> self.dec
            ops::decode_lzfse(&mut self.enc.as_slice(), &mut self.dec);
            // Validate
            assert!(data == self.dec);
            // TODO encode lzfse
        }
        Ok(())
    }

    pub fn decode_encode_decode<F, U>(&mut self, mut enc: &[u8], encode: F) -> io::Result<()>
    where
        F: Fn(&mut LzfseRingEncoder, &[u8], &mut Vec<u8>) -> io::Result<U>,
    {
        // Decode enc -> self.data
        self.data.clear();
        self.data.reserve(enc.len() * DECODE_F);
        self.decoder.decode(&mut enc, &mut self.data)?;
        // Encode self.data -> self.enc
        self.enc.clear();
        self.enc.reserve(enc.len());
        encode(&mut self.encoder, &self.data, &mut self.enc)?;
        // Decode self.enc -> self.dec
        self.dec.clear();
        self.dec.reserve(self.data.len());
        self.decoder.decode(&mut self.enc.as_slice(), &mut self.dec)?;
        // Validate
        assert!(self.data == self.dec);
        #[cfg(feature = "lzfse_ref")]
        {
            // Decode self.enc -> self.dec
            ops::decode_lzfse(&mut self.enc.as_slice(), &mut self.dec);
            // Validate
            assert!(self.data == self.dec);
            // TODO encode lzfse
        }
        Ok(())
    }
}

impl Default for Monkey {
    fn default() -> Self {
        Self {
            decoder: LzfseRingDecoder::default(),
            encoder: LzfseRingEncoder::default(),
            data: Vec::default(),
            enc: Vec::default(),
            dec: Vec::default(),
        }
    }
}
