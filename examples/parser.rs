extern crate erl_ext;

use erl_ext::Decoder;
use std::io;
use std::os;


fn main() {
    let args = os::args();
    if args.len() < 2 {
        println!("Usage: parser <filename or '-'>");
        os::set_exit_status(1);
        return
    }
    let mut f = match args[1].as_slice() {
        "-" => box io::stdin() as Box<io::Reader>,
        other =>
            box io::File::open(&Path::new(other)).unwrap() as Box<io::Reader>
    };
    let mut decoder = Decoder::new(&mut f);
    match decoder.read_prelude() {
        Ok(false) =>
            panic!("Invalid eterm!"),
        Err(io::IoError{desc: d, ..}) =>
            panic!("IoError: {}", d),
        _ => ()
    }
    let term_opt = decoder.decode_term();
    println!("{}", term_opt.unwrap());
}
