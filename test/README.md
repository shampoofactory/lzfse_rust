# test

API tests.

## Basic usage

Quick test:
```
$ cd test
$ cargo test
```

Full test:
```
$ cd test
$ cargo test -- --ignored
```

Extended test:
```
$ cd test
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu" cargo test --features "large_data huge_data lzfse_ref" -- --ignored 
```


The quick test takes minutes. The full test takes hours. The extended test takes many hours.

## Large data

Test large data files.

Before enabling we need to download, hash and compress the large data file set into `data/large`. As a prerequisite we require the reference [LZFSE](https://github.com/lzfse/lzfse) binary and a working internet connection.

Then from the project root:
```
$ ./scripts/init_large.sh

```

We can then pass the `large_data` feature flag to enable large data tests.

```
$ cd test
$ cargo test large --features large_data
```
```
$ cd test
$ cargo test --features large_data
```

## Lzfse reference

Test `lzfse_rust`/ `lzfse` compatibility. Here `lzfse_rust` compressed data is handed over to `lzfse` to decompress and vice versa.

Build the local `lzfse_sys` crate using the supplied instructions. To enable the reference lzfse compatibility testing we need to pass the `lzfse_ref` feature flag and inform rustc of the reference lzfse library `liblzfse.a` location, in this case `/usr/local/lib/x86_64-linux-gnu`.

```
$ cd test
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu" cargo test --features lzfse_ref -- --ignored
```

## Huge data

Test huge virtual synthetic data files using concurrent `lzfse_rust` process invocations.
Although we are testing 64GB+ data files the actual memory requirements should not exceed 2MB.

```
$ cd test
$ cargo test huge --features huge_data
```

```
$ cd test
$ cargo test --features huge_data
```

## Test organization

The library exposes:
* encoders: `LzfseEncoder`, `LzfseRingEncoder`
* encode writers: `LzfseWriterBytes`, `LzfseWriter`
* decoders: `LzfseDecoder`, `LzfseRingDecoder`
* decode readers: `LzfseReaderBytes`, `LzfseReader`

We need to ensure that the compression and decompression methods work as intended and that data corruption does not occur.
Additionally the library is designed to validate and reject input data; it should not hang, segfault, panic or break in a any other fashion.
Internally the code base is packed with debug statements that trip on invalid states, these are hard errors and should NOT occur.

Quick tests.
Small data sets and fast execution patterns.

* data - [`Snappy`](https://google.github.io/snappy/) data set.
* pattern - synthetic data pattern variations.

Extended tests.
We resort to throwing huge amounts of valid and invalid data at the API.

* large data - large data files: 100MB+.
* huge data - huge virtual synthetic data files: 64GB+.
* pattern - synthetic data pattern variations.
* patchwork - patchwork data.
* mutate - RAW, Vn, Vx1, Vx2 data mutation.
* fuzz - fuzzed read/ write.
* random - low entropy random data.