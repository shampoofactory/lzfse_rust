# lzfse_sys

Statically linked reference LZFSE library bindings. Benchmark and testing use only.

https://github.com/lzfse/lzfse

## Building

To simplify, or complicate matters, we build the reference LZFSE library/ `liblzfse.a` to our specifications and inform rustc of it's whereabouts.

* Download the LZFSE reference [source](https://github.com/lzfse/lzfse) and extract to a folder of your choice.

* To give the reference library a fair chance in benchmarking we should optimize the the `Makefile` CFLAGS.
```makefile
CFLAGS := -O3 -march=native -Wall -Wno-unknown-pragmas -Wno-unused-variable -DNDEBUG -D_POSIX_C_SOURCE -std=c99 -fvisibility=hidden
```

* Compile using the instructions given in the `LZFSE` reference `README.md`. Note the final destination of the `liblzfse.a` file.

* Build `lzfse_sys` by passing the location of `liblzfse.a`. In this example the location is `/usr/local/lib/x86_64-linux-gnu`, although your machine may differ.
```bash
$ cd lzfse_sys
$ RUSTFLAGS='-L /usr/local/lib/x86_64-linux-gnu' cargo build
```
