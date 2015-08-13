#![feature(rustc_private)]
extern crate erl_ext;
extern crate getopts;

use getopts::{optflag,getopts};
use erl_ext::{Decoder,Encoder};
use std::io::Write;
use std::io;
use std::env;
use std::fs;
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    let opts = [
        optflag("u", "utf8-atoms", "Use utf-8 atoms feature"),
        optflag("s", "small-atoms", "Use small atoms feature"),
        optflag("f", "fair-new-fun", "Fairly calculate NEW_FUN size (requires extra memory)"),
        ];
    // skip(1)
    let matches = match getopts(args.as_ref(), opts.as_ref()) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.free.len() != 2 {
        println!("Usage: {} [opts] <in-file or '-'> <out-file or '-'>", args[0]);
        for o in opts.iter() {
            println!("-{}\t--{}\t{}", o.short_name, o.long_name, o.desc);
        }
        exit(1);
    }
    let mut in_f = match matches.free[0].as_ref() {
        "-" => Box::new(io::stdin()) as Box<io::Read>,
        other =>
            Box::new(fs::File::open(other).unwrap()) as Box<io::Read>
    };
    let mut out_f = match matches.free[1].as_ref() {
        "-" => Box::new(io::stdout()) as Box<io::Write>,
        other =>
            Box::new(fs::File::create(other).unwrap()) as Box<io::Write>
    };

    let mut src = Vec::new();
    in_f.read_to_end(&mut src).unwrap();
    let dest = Vec::new();

    let mut rdr = io::Cursor::new(src);
    let mut wrtr = io::BufWriter::new(dest);
    {
        // decode term
        let mut decoder = Decoder::new(&mut rdr);
        match decoder.read_prelude() {
            Ok(false) =>
                panic!("Invalid eterm!"),
            Err(e) =>
                panic!("DecodeError: {}", e),
            _ => ()
        }
        let term = decoder.decode_term().unwrap();
        // print it to stderr
        (write!(&mut io::stderr(), "{:?}\n", term)).unwrap();
        // and encode it
        let mut encoder = Encoder::new(&mut wrtr,
                                       matches.opt_present("u"),
                                       matches.opt_present("s"),
                                       matches.opt_present("f"));
        encoder.write_prelude().unwrap();
        encoder.encode_term(term).unwrap();
    }
    // write encoded result to out_f
    out_f.write(wrtr.get_ref()).unwrap();

    // compare original and encoded
    if wrtr.get_ref() != rdr.get_ref() {
        (write!(&mut io::stderr(), "Before and After isn't equal\n")).unwrap();
        exit(1);
    }
}
