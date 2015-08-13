#![feature(exit_status)]
#![feature(core)]

extern crate erl_ext;
extern crate core;

use std::convert::AsRef;

use erl_ext::Decoder;
use std::io;
use std::env;
use std::fs;

fn main() {
    let mut args = env::args();
    if args.len() < 2 {
        println!("Usage: parser <filename or '-'>");
        env::set_exit_status(1);
        return
    }
    let mut f = match args.nth(1).unwrap().as_ref() {
        "-" => Box::new(io::stdin()) as Box<io::Read>,
        other =>
            Box::new(fs::File::open(other).unwrap()) as Box<io::Read>,
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
