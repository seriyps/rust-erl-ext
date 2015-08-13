#![feature(convert)]
extern crate erl_ext;

use erl_ext::{Encoder,Decoder};
use std::io;
use std::io::{Write,Read};
use std::fs;
use std::process;
use std::path::Path;

#[test]
fn main() {
    // generate tests/data/*.bin
    match process::Command::new("escript").arg("tests/term_gen.erl").spawn() {
        Ok(_) => (),
        Err(ioerr) => {
            (writeln!(
                &mut io::stderr(),
                "{}:{} [warn] Failed to launch escript - '{}'. Is Erlang installed?",
                file!(), line!(), ioerr)).unwrap();
            return
        }
    };
    // run decode-encode cycle and compare source and resulting binaries
    let data_dir = Path::new("tests/data");
    for path in (fs::read_dir(&data_dir)
                 .unwrap()
                 .map(|de| de.unwrap().path())
                 .filter(|ref p| p.extension().unwrap().to_str() == Some("bin"))) {

        let mut in_f = fs::File::open(&path).unwrap();
        let mut src = Vec::new();
        in_f.read_to_end(&mut src).unwrap();
        let mut rdr = io::BufReader::new(src.as_slice());

        let dest = Vec::new();
        let mut wrtr = io::BufWriter::new(dest);

        {
            let mut decoder = Decoder::new(&mut rdr);
            assert!(true == decoder.read_prelude().unwrap(),
                    "{}: bad prelude", path.display());
            let term = decoder.decode_term().unwrap();

            let mut encoder = Encoder::new(&mut wrtr, false, false, true);
            encoder.write_prelude().unwrap();
            encoder.encode_term(term).unwrap();
        }
        assert!(wrtr.get_ref() == rdr.get_ref(),
                "{}: Before and After isn't equal", path.display());
    }
}
