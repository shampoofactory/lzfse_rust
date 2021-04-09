# lzfse_sys

Statically linked reference lzfse library bindings. Benchmark and testing use only.

https://github.com/lzfse/lzfse

## Building

To simplify, or complicate matters, we build the reference lzfse library/ `liblzfse.a` to our specifications and inform rustc of it's whereabouts.

```bash
RUSTFLAGS='-L /usr/local/lib/x86_64-linux-gnu' cargo build
```

To give the reference library a fair chance in benchmarking we should optimize the the Makefile CFLAGS:

 ```makefile
 CFLAGS := -O3 -march=native -Wall -Wno-unknown-pragmas -Wno-unused-variable -DNDEBUG -D_POSIX_C_SOURCE -std=c99 -fvisibility=hidden
 ```
