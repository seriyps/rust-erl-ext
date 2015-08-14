// see erlang_rust_port.erl

extern crate erl_ext;
extern crate getopts;

use getopts::Options;
use erl_ext::{Decoder,Encoder,Error};

use std::io;
use std::env;


fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optflag("u", "utf8-atoms", "Use utf-8 atoms feature");
    opts.optflag("s", "small-atoms", "Use small atoms feature");
    opts.optflag("f", "fair-new-fun", "Fairly calculate NEW_FUN size (requires extra memory)");

    let matches = match opts.parse(&args[1..]) {
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
