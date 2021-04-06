# lzfoo

NAME powered lzfse command line tool clone.

```
$ lzfoo
lzfoo 0.1.0
Vin Singh <github.com/shampoofactory>
LZFSE compressor/ decompressor

USAGE:
    lzfoo <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    -decode    Decode (decompress)
    -encode    Encode (compress)
    help       Prints this message or the help of the given subcommand(s)

See 'lzfoo help <command>' for more information on a specific command.
```

## Installation

TODO


## Basic usage

```
$ lzfoo help -encode
lzfoo--encode 
Encode (compress)

USAGE:
    lzfoo -encode [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -i <FILE>        input
    -o <FILE>        output

If no input/ output specified reads/ writes from standard input/ output
```

Compress `a.txt` to `a.txt.lzfse`:
```
$ lzfoo -encode -i a.txt -o a.txt.lzfse
```

Compress with stdin/ stdout:
```
$ lzfoo -encode -i < a.txt > a.txt.lzfse
```
```
$ echo "semper fidelis" | lzfoo -encode > a.txt.lzfse
```

```
$ lzfoo help -decode
lzfoo--decode 
Decode (decompress)

USAGE:
    lzfoo -decode [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -i <FILE>        input
    -o <FILE>        output

If no input/ output specified reads/ writes from standard input/ output.
```

Decompress `a.txt.lzfse` to `a.txt`:
```
$ lzfoo -decode -i a.txt.lzfse -o a.txt
```

Decompress with stdin/ stdout:
```
$ lzfoo -decode -i < a.txt.lzfse > a.txt
```
```
$ cat a.txt.lzfse | lzfoo -decode
```

## Internals

In contrast to the reference `lzfse` implementation `lzfoo` uses a streaming engine.
More specifically `lzfse` operates by loading/ storing files in their entirely whereas `lzfoo` operates by sequentially loading/ storing small file chunks.
The big upshots being that both latency and max memory usage are reduced.
Whilst `lzfse`'s memory usage is at least the sum of the input and output byte length, `lzfoo` is capped at below 2MB regardless. It should be noted that when operating on very small files `lzfse` will use less memory.

TODO: Probably better to use multiple datasets/ graph.

As an example taking the [`enwik8`](https://cs.fit.edu/~mmahoney/compression/textdata.html) dataset.

Decode memory usage (bytes): 
* `lzfse` 253,152,164
* `lzfoo` 794,720

Encode memory usage (bytes):
* `lzfse` 400,684,384
* `lzfoo` 1,315, 506

```
$ valgrind lzfse -decode -i enwik8.lzfse -o /dev/null
...
==31127== HEAP SUMMARY:
==31127==     in use at exit: 0 bytes in 0 blocks
==31127==   total heap usage: 4 allocs, 4 frees, 253,152,164 bytes allocated
...

$ valgrind lzfoo -decode -i enwik8.lzfse -o /dev/null
...
==31002== HEAP SUMMARY:
==31002==     in use at exit: 32 bytes in 1 blocks
==31002==   total heap usage: 75 allocs, 74 frees, 794,720 bytes allocated
...

$ valgrind lzfse -encode -i enwik8 -o /dev/null
...
==31211== HEAP SUMMARY:
==31211==     in use at exit: 0 bytes in 0 blocks
==31211==   total heap usage: 4 allocs, 4 frees, 400,684,384 bytes allocated
...

$ valgrind lzfoo -encode -i enwik8 -o /dev/null
...
==31258== HEAP SUMMARY:
==31258==     in use at exit: 32 bytes in 1 blocks
==31258==   total heap usage: 76 allocs, 75 frees, 1,315,506 bytes allocated
...

```


