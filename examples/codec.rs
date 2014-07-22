extern crate erl_ext;

use erl_ext::{Decoder,Encoder};
use std::io;
use std::os;


fn main() {
    let args = os::args();
    if args.len() < 3 {
        // TODO: make encoder opts configurable
        println!("Usage: codec <in-file or '-'> <out-file or '-'>");
        os::set_exit_status(1);
        return
    }
    let mut in_f = match args[1].as_slice() {
        "-" => box io::stdin() as Box<io::Reader>,
        other =>
            box io::File::open(&Path::new(other)).unwrap() as Box<io::Reader>
    };
    let mut out_f = match args[2].as_slice() {
        "-" => box io::stdout() as Box<io::Writer>,
        other =>
            box io::File::create(&Path::new(other)).unwrap() as Box<io::Writer>
    };

    let src = in_f.read_to_end().unwrap();

    let mut rdr = io::MemReader::new(src);
    let mut wrtr = io::MemWriter::new();
    {
        // decode term
        let mut decoder = Decoder::new(&mut rdr);
        match decoder.read_prelude() {
            Ok(false) =>
                fail!("Invalid eterm!"),
            Err(io::IoError{desc: d, ..}) =>
                fail!("IoError: {}", d),
            _ => ()
        }
        let term = decoder.decode_term().unwrap();
        // print it to stderr
        (write!(io::stderr(), "{}", term)).unwrap();
        // and encode it
        let mut encoder = Encoder::new(&mut wrtr, false, false, true);
        encoder.write_prelude().unwrap();
        encoder.encode_term(term).unwrap();
    }
    // write encoded result to out_f
    out_f.write(wrtr.get_ref()).unwrap();

    // compare original and encoded
    if wrtr.get_ref() != rdr.get_ref() {
        (write!(io::stderr(), "Before and After isn't equal")).unwrap();
        os::set_exit_status(1);
        return
    }
}
