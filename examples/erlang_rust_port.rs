// see erlang_rust_port.erl
extern crate erl_ext;
extern crate getopts;

use getopts::{optflag,getopts};
use erl_ext::{Decoder,Encoder};
use std::io;
use std::io::stdio;
use std::os;


fn main() {
    let args = os::args();
    let opts = [
        optflag("u", "utf8-atoms", "Use utf-8 atoms feature"),
        optflag("s", "small-atoms", "Use small atoms feature"),
        optflag("f", "fair-new-fun", "Fairly calculate NEW_FUN size (requires extra memory)"),
        ];
    let matches = match getopts(args.tail(), opts.as_slice()) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    let mut in_f = stdio::stdin_raw();
    let mut out_f = stdio::stdout_raw();
    // let mut out_writer = std::io::BufferedWriter::with_capacity(20480,
    //                                                             out_f.unwrap());
    let decoder = Decoder::new(&mut in_f);
    let encoder = Encoder::new(&mut out_f,
                                   matches.opt_present("u"),
                                   matches.opt_present("s"),
                                   matches.opt_present("f"));
    match read_write_loop(decoder, encoder) {
        Err(io::IoError{kind: io::EndOfFile, ..}) => (), // port was closed
        Err(io::IoError{kind, desc, ..}) =>
            panic!("kind: {}, desc: '{}'", kind, desc),
        Ok(()) => ()            // unreachable in this example
    };
}

fn read_write_loop(mut decoder: Decoder, mut encoder: Encoder) -> io::IoResult<()> {
    loop {
        assert!(true == try!(decoder.read_prelude()));
        let term = try!(decoder.decode_term());
        try!(encoder.write_prelude());
        try!(encoder.encode_term(term));
        // out_writer.flush();
    }
}
