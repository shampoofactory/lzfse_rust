use crate::bits::ByteBits;
use crate::lmd::{LiteralLenPack, LmdPack, MatchDistancePack, MatchLenPack};

use test_kit::Rng;

use super::constants::*;
use super::decoder::Decoder;
use super::encoder::Encoder;
use super::literals::Literals;
use super::lmds::Lmds;
use super::weights::Weights;
use super::Fse;

use std::io::Write;

fn literal_encode_decode_check(
    mut data: &[u8],
    literals: &mut Literals,
    weights: &mut Weights,
    encoder: &mut Encoder,
    decoder: &mut Decoder,
    store: &mut Vec<u8>,
) -> crate::Result<()> {
    assert!(data.len() <= LITERALS_PER_BLOCK as usize);

    let data_len = data.len();
    literals.reset();
    unsafe { literals.push_unchecked(&mut data, data_len as u32) };

    let u = weights.load(&[], literals.as_ref());
    literals.pad_u(u);
    encoder.init(&weights);

    store.clear();
    store.write_all(&[0; 8])?;
    let param = literals.store(store, &encoder)?;

    literals.reset();
    decoder.init(&weights);

    let src = ByteBits::new(store.as_slice());
    literals.load(src, &decoder, &param)?;

    assert_eq!(data, &literals.as_ref()[..data.len()]);
    Ok(())
}

fn lmds_encode_decode_check(
    data: &[LmdPack<Fse>],
    lmds: &mut Lmds,
    weights: &mut Weights,
    encoder: &mut Encoder,
    decoder: &mut Decoder,
    store: &mut Vec<u8>,
) -> crate::Result<()> {
    assert!(data.len() <= LMDS_PER_BLOCK as usize);

    lmds.reset();
    data.iter().for_each(|&lmd| unsafe { lmds.push_unchecked(lmd) });

    let _ = weights.load(lmds.as_ref(), &[]);
    encoder.init(&weights);

    store.clear();
    store.write_all(&[0; 8])?;
    let param = lmds.store(store, &encoder)?;

    lmds.reset();
    decoder.init(&weights);

    let src = ByteBits::new(store.as_slice());
    lmds.load(src, &decoder, &param)?;

    assert_eq!(data, &lmds.as_ref()[..data.len()]);
    Ok(())
}

#[test]
fn literal_encode_decode() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();

    let data = b"Full fathom five thy father lies; \
                 Of his bones are coral made; \
                 Those are pearls that were his eyes: \
                 Nothing of him that doth fade; \
                 But doth suffer a sea-change; \
                 Into something rich and strange."; // William Shakespeare. The Tempest.

    literal_encode_decode_check(
        data,
        &mut literals,
        &mut weights,
        &mut encoder,
        &mut decoder,
        &mut store,
    )
}

#[test]
#[ignore = "expensive"]
fn literal_encode_decode_rng() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let mut data = Vec::default();
    for seed in 0..4096 {
        data.clear();
        Rng::new(seed).take(LITERALS_PER_BLOCK as usize).for_each(|u| data.push(u as u8));
        literal_encode_decode_check(
            &data,
            &mut literals,
            &mut weights,
            &mut encoder,
            &mut decoder,
            &mut store,
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn literal_encode_decode_len() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let mut data = Vec::default();
    for n in 0..4096 {
        data.clear();
        Rng::default().take(n).for_each(|u| data.push(u as u8));
        literal_encode_decode_check(
            &data,
            &mut literals,
            &mut weights,
            &mut encoder,
            &mut decoder,
            &mut store,
        )?;
    }
    Ok(())
}

#[test]
fn literal_encode_decode_seq() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();

    let data = (0..255).collect::<Vec<_>>();

    literal_encode_decode_check(
        data.as_ref(),
        &mut literals,
        &mut weights,
        &mut encoder,
        &mut decoder,
        &mut store,
    )
}

#[test]
#[ignore = "expensive"]
fn literal_encode_decode_twin_interleave_balanced() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let mut data = Vec::default();
    for i in 0..U_STATES as usize {
        for j in 0..U_STATES as usize {
            data.clear();
            data.resize(data.len() + 4 * i, 0);
            data.resize(data.len() + 4 * j, 1);
            if i < j {
                data.resize(data.len() + 4 * (j - i), 0);
            } else {
                data.resize(data.len() + 4 * (i - j), 1);
            };
            literal_encode_decode_check(
                data.as_slice(),
                &mut literals,
                &mut weights,
                &mut encoder,
                &mut decoder,
                &mut store,
            )?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn literal_encode_decode_twin_interleave() -> crate::Result<()> {
    let mut literals = Literals::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let mut data = Vec::default();
    for i in 0..U_STATES as usize {
        for j in 0..U_STATES as usize {
            data.clear();
            data.resize(data.len() + 4 * i, 0);
            data.resize(data.len() + 4 * j, 1);
            literal_encode_decode_check(
                data.as_slice(),
                &mut literals,
                &mut weights,
                &mut encoder,
                &mut decoder,
                &mut store,
            )?;
        }
    }
    Ok(())
}

#[test]
fn lmd_encode_decode() -> crate::Result<()> {
    let mut lmds = Lmds::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let data = unsafe {
        [
            LmdPack::<Fse>::new_unchecked(128, 1, 256),
            LmdPack::<Fse>::new_unchecked(128, 0, 0),
            LmdPack::<Fse>::new_unchecked(256, 128, 1),
            LmdPack::<Fse>::new_unchecked(256, 128, 0),
            LmdPack::<Fse>::new_unchecked(0, 128, 1),
            LmdPack::<Fse>::new_unchecked(0, 128, 0),
        ]
    };
    lmds_encode_decode_check(&data, &mut lmds, &mut weights, &mut encoder, &mut decoder, &mut store)
}

#[test]
#[ignore = "expensive"]
fn lmd_encode_decode_rng() -> crate::Result<()> {
    let mut lmds = Lmds::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let mut data = Vec::default();
    for seed in 0..4096 {
        let mut rng = Rng::new(seed);
        data.clear();
        for _ in 0..LMDS_PER_BLOCK {
            let l = (rng.gen() as u64 * MAX_L_VALUE as u64) >> 32;
            let m = (rng.gen() as u64 * MAX_M_VALUE as u64) >> 32;
            let d = (rng.gen() as u64 * MAX_D_VALUE as u64) >> 32;
            let lmd = unsafe { LmdPack::new_unchecked(l as u16, m as u16, d as u32) };
            data.push(lmd);
        }
        lmds_encode_decode_check(
            &data,
            &mut lmds,
            &mut weights,
            &mut encoder,
            &mut decoder,
            &mut store,
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn l_encode_decode_seq() -> crate::Result<()> {
    let mut lmds = Lmds::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let data = (0..=MAX_L_VALUE)
        .map(|u| LiteralLenPack::new(u as u16))
        .map(|u| LmdPack(u, MatchLenPack::default(), MatchDistancePack::default()))
        .collect::<Vec<_>>();
    lmds_encode_decode_check(&data, &mut lmds, &mut weights, &mut encoder, &mut decoder, &mut store)
}

#[test]
#[ignore = "expensive"]
fn m_encode_decode_seq() -> crate::Result<()> {
    let mut lmds = Lmds::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let data = (0..=MAX_M_VALUE)
        .map(|u| MatchLenPack::new(u as u16))
        .map(|u| LmdPack(LiteralLenPack::default(), u, MatchDistancePack::default()))
        .collect::<Vec<_>>();
    lmds_encode_decode_check(&data, &mut lmds, &mut weights, &mut encoder, &mut decoder, &mut store)
}

#[test]
#[ignore = "expensive"]
fn d_encode_decode_seq() -> crate::Result<()> {
    let mut lmds = Lmds::default();
    let mut weights = Weights::default();
    let mut encoder = Encoder::default();
    let mut decoder = Decoder::default();
    let mut store = Vec::default();
    let data = (0..=MAX_D_VALUE)
        .map(MatchDistancePack::new)
        .map(|u| LmdPack(LiteralLenPack::default(), MatchLenPack::default(), u))
        .collect::<Vec<_>>();
    for chunk in data.chunks(LMDS_PER_BLOCK as usize) {
        lmds_encode_decode_check(
            chunk,
            &mut lmds,
            &mut weights,
            &mut encoder,
            &mut decoder,
            &mut store,
        )?;
    }
    Ok(())
}
