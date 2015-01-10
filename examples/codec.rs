extern crate erl_ext;
extern crate getopts;

use getopts::{optflag,getopts};
use erl_ext::{Decoder,Encoder};
use std::io;
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
    if matches.free.len() != 2 {
        println!("Usage: {} [opts] <in-file or '-'> <out-file or '-'>", args[0]);
        for o in opts.iter() {
            println!("-{}\t--{}\t{}", o.short_name, o.long_name, o.desc);
        }
        os::set_exit_status(1);
        return
    }
    let mut in_f = match matches.free[0].as_slice() {
        "-" => Box::new(io::stdin()) as Box<io::Reader>,
        other =>
            Box::new(io::File::open(&Path::new(other)).unwrap()) as Box<io::Reader>
    };
    let mut out_f = match matches.free[1].as_slice() {
        "-" => Box::new(io::stdout()) as Box<io::Writer>,
        other =>
            Box::new(io::File::create(&Path::new(other)).unwrap()) as Box<io::Writer>
    };

    let src = in_f.read_to_end().unwrap();

    let mut rdr = io::MemReader::new(src);
    let mut wrtr = io::MemWriter::new();
    {
        // decode term
        let mut decoder = Decoder::new(&mut rdr);
        match decoder.read_prelude() {
            Ok(false) =>
                panic!("Invalid eterm!"),
            Err(io::IoError{desc: d, ..}) =>
                panic!("IoError: {}", d),
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
        os::set_exit_status(1);
        return
    }
}
