# bench

[`Criterion`](https://github.com/bheisler/criterion.rs) powered benchmarks.

## Basic usage

```bash
$ cd bench
$ RUSTFLAGS="-C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap
```

```bash
$ cd bench
$ RUSTFLAGS="-C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap -- --save-baseline before
```

## Lzfse reference

Build the local `lzfse_sys` crate using the supplied instructions. To enable the reference lzfse benchmarks we need to pass the `lzfse_ref` feature flag and inform rustc of the reference lzfse library `liblzfse.a` location, in this case `/usr/local/lib/x86_64-linux-gnu`.

```bash
$ cd bench
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu -C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap --features lzfse_ref
```
## Organization

The benchmarks are organized by: engine, operation and dataset.

* engine: lzfse_ref, rust, rust_ring.

* operation: encode, decode.

* dataset: snappy, synth

Output is formatted as: engine/operation/dataset_data

As a matter of expedience the [`snappy`](https://github.com/google/snappy) data is used as a generalized set and is our primary reference. As an alternative the synth(etic) data is comprised of noise/ naive patterns and is useful in tuning internal components.

## Critcmp

To compare benchmarks we can use [`critcmp`](https://github.com/BurntSushi/critcmp).

Baseline `new` compare all engines.

```bash
$ critcmp new -g '.*?/(.*$)'
```

Baseline `new` compare `rust` with `rust_ring`.

```bash
$ critcmp new -g '[t|g]/(.*$)'
```

Baseline `new` compare `lzfse_ref` with `rust`.

```bash
$ critcmp new -g '[f|t]/(.*$)'
```

Baseline `new` compare `lzfse_ref` with `rust_ring`.

```bash
$ critcmp new -g '[f|g]/(.*$)'
```
