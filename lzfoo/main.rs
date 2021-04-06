use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use lzfse_rust::{LzfseRingDecoder, LzfseRingEncoder};

use core::panic;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;
use std::time::Instant;

const STDIN: &str = "stdin";
const STDOUT: &str = "stdout";
const ENCODE: &str = "encode";
const DECODE: &str = "decode";

fn main() {
    process::exit(match execute() {
        Ok(()) => 0,
        Err(lzfse_rust::Error::Io(err)) if err.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(lzfse_rust::Error::Io(err)) => {
            eprint!("Error: IO: {}", err);
            1
        }
        Err(lzfse_rust::Error::BufferOverflow) => {
            eprint!("Error: Buffer overflow");
            1
        }
        Err(err) => {
            eprintln!("Error: Decode: {}", err);
            1
        }
    });
}

fn execute() -> lzfse_rust::Result<()> {
    let matches = arg_matches();
    match matches.subcommand() {
        ("-encode", Some(m)) => {
            encode(m.value_of("input"), m.value_of("output"), m.occurrences_of("v") != 0)?
        }
        ("-decode", Some(m)) => {
            decode(m.value_of("input"), m.value_of("output"), m.occurrences_of("v") != 0)?
        }
        _ => panic!(),
    };

    Ok(())
}

fn encode(input: Option<&str>, output: Option<&str>, verbose: bool) -> io::Result<()> {
    let instant = if verbose { Some(Instant::now()) } else { None };
    let mut src: Box<dyn Read> = match input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin()),
    };
    let mut dst: Box<dyn Write> = match output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };
    let (n_raw_bytes, n_payload_bytes) = LzfseRingEncoder::default().encode(&mut src, &mut dst)?;
    if let Some(start) = instant {
        stats(start, n_raw_bytes, n_payload_bytes, input, output, ENCODE)
    }
    Ok(())
}

fn decode(input: Option<&str>, output: Option<&str>, verbose: bool) -> lzfse_rust::Result<()> {
    let instant = if verbose { Some(Instant::now()) } else { None };
    let mut src: Box<dyn Read> = match input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin()),
    };
    let mut dst: Box<dyn Write> = match output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };
    let (n_raw_bytes, n_payload_bytes) = LzfseRingDecoder::default().decode(&mut src, &mut dst)?;
    if let Some(start) = instant {
        stats(start, n_raw_bytes, n_payload_bytes, input, output, DECODE)
    }
    Ok(())
}

#[cold]
fn stats(
    start: Instant,
    n_raw_bytes: u64,
    n_payload_bytes: u64,
    input: Option<&str>,
    output: Option<&str>,
    mode: &str,
) {
    let duration = Instant::now() - start;
    let secs = duration.as_secs_f64();
    let ns_per_byte = 1.0e9 * secs / n_raw_bytes as f64;
    let mb_per_sec = n_raw_bytes as f64 / 1024.0 / 1024.0 / secs;
    if output.is_none() {
        eprintln!();
    }
    eprintln!("LZFSE {}", mode);
    eprintln!("Input: {}", input.unwrap_or(STDIN));
    eprintln!("Output: {}", output.unwrap_or(STDOUT));
    eprintln!("Input size: {} B", n_raw_bytes);
    eprintln!("Output size: {} B", n_payload_bytes);
    eprintln!("Compression ratio: {:.3}", n_raw_bytes as f64 / n_payload_bytes as f64);
    eprintln!("Speed: {:.2} ns/B, {:.2} MB/s", ns_per_byte, mb_per_sec);
}

fn arg_matches() -> ArgMatches<'static> {
    App::new("lzfoo")
        .version(crate_version!())
        .author("Vin Singh <github.com/shampoofactory>")
        .about("LZFSE compressor/ decompressor")
        .after_help("See 'lzfoo help <command>' for more information on a specific command.")
        .subcommand(
            SubCommand::with_name("-decode")
                .alias("decode")
                .about("Decode (decompress)")
                .after_help(
                    "If no input/ output specified reads/ writes from standard input/ output.",
                )
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .help("input")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .help("output")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(Arg::with_name("v").short("v").help("Sets the level of verbosity")),
        )
        .subcommand(
            SubCommand::with_name("-encode")
                .alias("encode")
                .about("Encode (compress)")
                .after_help(
                    "If no input/ output specified reads/ writes from standard input/ output",
                )
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .help("input")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .help("output")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(Arg::with_name("v").short("v").help("Sets the level of verbosity")),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches()
}
