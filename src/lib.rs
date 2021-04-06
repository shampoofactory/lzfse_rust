#![warn(missing_docs)]
/*!
This crate provides an enhanced implementation of the [Lzfse](https://github.com/lzfse/lzfse)
compression library.

### Install

Simply configure your `Cargo.toml`:

```toml
[dependencies]
lzfse_rust = "1"
```

### Overview.

This crate provides two Lzfse engines: one operating over memory buffers and one operating over
IO streams using internal ring buffers. The latter in addition to being highly memory efficient
is able to expose `Read` and `Write` interfaces.

### Example: compress IO data

This program compresses data from `stdin` into `stdout`. This example can be found in
 `examples/compress_ring.rs`

```no_run
use lzfse_rust::LzfseRingEncoder;
use std::io;

fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut encoder = LzfseRingEncoder::default();
    encoder.encode(&mut rdr, &mut wtr)?;
    Ok(())
}

```

### Example: decompress IO data

This program decompresses data from `stdin` into `stdout`. This example can be found in
 `examples/decompress_ring.rs`

```no_run
use lzfse_rust::LzfseRingDecoder;
use std::io;

fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut decoder = LzfseRingDecoder::default();
    decoder.decode(&mut rdr, &mut wtr)?;
    Ok(())
}

```

### Example: compress buffered data

This program compresses data from `stdin` into `stdout`. This example can be found in
 `examples/compress.rs`

```no_run
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Compress src into dst.
    let mut dst = Vec::default();
    lzfse_rust::encode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
```

### Example: decompress buffered data

This program decompresses data from `stdin` into `stdout`. This example can be found in
 `examples/decompress.rs`

```no_run
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Compress src into dst.
    let mut dst = Vec::default();
    lzfse_rust::encode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
```

### Additional notes

The memory buffered engine is exposed as [LzfseDecoder] and [LzfseEncoder] along with the helper
methods [decode_bytes] and [encode_bytes]. This engine should be considered when operating directly
with `&[u8]` slices and `Vec<u8>` types. The helper methods whilst convenient should not be used
repeatedly, in this situation it is more efficient to create either a [LzfseDecoder] or
[LzfseEncoder] to reuse.

The ring buffered engine is exposed as [LzfseRingDecoder] and [LzfseRingEncoder]. This engine
should be considered when operating over IO streams or when [Read](std::io::Read)
or [Write](std::io::Write) functionality is required.

Kindly refer to individual struct and method documentation as there are additional and important
details that are not covered here.
*/

mod base;
mod bits;
mod decode;
mod encode;
mod error;
mod fse;
mod io;
mod kit;
mod lmd;
mod lz;
mod match_kit;
mod ops;
mod raw;
mod ring;
mod types;
mod vn;

pub use decode::{decode_bytes, LzfseDecoder, LzfseReader, LzfseReaderBytes, LzfseRingDecoder};
pub use encode::{encode_bytes, LzfseEncoder, LzfseRingEncoder, LzfseWriter, LzfseWriterBytes};
pub use error::{Error, Result};
