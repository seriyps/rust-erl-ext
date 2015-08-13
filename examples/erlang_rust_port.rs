// see erlang_rust_port.erl
#![feature(rustc_private)]
#![feature(core)]
#![feature(collections)]

extern crate erl_ext;
extern crate getopts;
extern crate core;

use getopts::{optflag,getopts};
use erl_ext::{Decoder,Encoder,Error};
use core::array::FixedSizeArray;

use std::io;
use std::env;


fn main() {
    let args: Vec<String> = env::args().collect();
    let opts = [
        optflag("u", "utf8-atoms", "Use utf-8 atoms feature"),
        optflag("s", "small-atoms", "Use small atoms feature"),
        optflag("f", "fair-new-fun", "Fairly calculate NEW_FUN size (requires extra memory)"),
        ];
    let matches = match getopts(args.tail(), opts.as_slice()) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    let mut in_f = io::stdin();
    let mut out_f = io::stdout();
    // let mut out_writer = std::io::BufferedWriter::with_capacity(20480,
    //                                                             out_f.unwrap());
    let decoder = Decoder::new(&mut in_f);
    let encoder = Encoder::new(&mut out_f,
                               matches.opt_present("u"),
                               matches.opt_present("s"),
                               matches.opt_present("f"));
    match read_write_loop(decoder, encoder) {
        Err(Error::ByteorderUnexpectedEOF) => (), // port was closed
        Err(ref err) =>
            panic!("Error: {}", err),
        Ok(()) => ()            // unreachable in this example
    };
}

fn read_write_loop<R: io::Read>(mut decoder: Decoder<R>, mut encoder: Encoder) -> Result<(), Error> {
    loop {
        assert!(true == try!(decoder.read_prelude()));
        let term = try!(decoder.decode_term());
        try!(encoder.write_prelude());
        try!(encoder.encode_term(term));
        try!(encoder.flush());
    }
}
