extern crate erl_ext;

use std::convert::AsRef;

use erl_ext::Decoder;
use std::io;
use std::env;
use std::fs;
use std::process::exit;

fn main() {
    let mut args = env::args();
    if args.len() < 2 {
        println!("Usage: parser <filename or '-'>");
        exit(1);
    }
    let mut f: Box<io::Read> = match args.nth(1).unwrap().as_ref() {
        "-" => Box::new(io::stdin()),
        other =>
            Box::new(fs::File::open(other).unwrap()),
    };
    let mut decoder = Decoder::new(&mut f);
    match decoder.read_prelude() {
        Ok(false) =>
            panic!("Invalid eterm!"),
        Err(err) =>
            panic!("IoError: {}", err),
        _ => ()
    }
    let term_opt = decoder.decode_term();
    println!("{:?}", term_opt.unwrap());
}
