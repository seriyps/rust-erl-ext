// see json_port.erl
extern crate erl_ext;
extern crate serialize;

use erl_ext::Eterm;
use serialize::json::{mod, Json};
use std::io;


fn main() {
    let in_f = io::stdin();
    let out_f = io::stdout();
    match read_write_loop(in_f, out_f) {
        Err(io::IoError{kind: io::EndOfFile, ..}) => (), // port was closed
        Err(io::IoError{kind, desc, ..}) =>
            panic!("kind: {}, desc: '{}'", kind, desc),
        Ok(()) => ()            // unreachable in this example
    };
}

fn read_write_loop<R: io::Reader, W: io::Writer>(mut r: R, mut w: W) -> io::IoResult<()> {
    loop {
        // {packet, 2}
        let _in_packet_size = r.read_be_u16();
        {
            let mut decoder = erl_ext::Decoder::new(&mut r);
            assert!(true == try!(decoder.read_prelude()));
            let term = try!(decoder.decode_term());
            // incoming message should be simple `binary()`
            let response = match term {
                Eterm::Binary(bytes) => {
                    bytes_to_json(bytes)
                },
                _ =>
                    // {error, not_binary}
                    Eterm::Tuple(vec!(
                        Eterm::Atom(String::from_str("error")),
                        Eterm::Atom(String::from_str("not_binary"))
                        ))
            };
            // Temp buffer to calculate response term size
            let mut wrtr = io::MemWriter::new();
            {
                // encode response term
                let mut encoder = erl_ext::Encoder::new(&mut wrtr,
                                                    true, true, true);
                try!(encoder.write_prelude());
                try!(encoder.encode_term(response));
            }
            // response packet size
            let out_packet_size = wrtr.get_ref().len() as u16;
            try!(w.write_be_u16(out_packet_size));
            // response term itself
            try!(w.write(wrtr.get_ref()));
            try!(w.flush());
        }
    }
}

fn bytes_to_json(json_bytes: Vec<u8>) -> erl_ext::Eterm {
    // Vec<u8> to utf-8 String
    let json_string = match String::from_utf8(json_bytes) {
        Ok(s) => s,
        Err(_) =>
            return Eterm::Tuple(vec!(
                Eterm::Atom(String::from_str("error")),
                Eterm::Atom(String::from_str("bad_utf8"))))
    };
    // &str to json::Json
    let json_obj = match json::from_str(json_string.as_slice()) {
        Ok(o) => o,
        Err(json::ParserError::SyntaxError(err_code, _, _)) => {
            let err_str = json::error_str(err_code);
            return Eterm::Tuple(vec!(
                Eterm::Atom(String::from_str("error")),
                Eterm::String(err_str.as_bytes().to_vec())
                    ))
        },
        Err(json::ParserError::IoError(_, err_str)) =>
            return Eterm::Tuple(vec!(
                Eterm::Atom(String::from_str("error")),
                Eterm::String(err_str.as_bytes().to_vec())
                    ))
    };
    // json::Json to erl_ext::Eterm
    Eterm::Tuple(vec!(Eterm::Atom(String::from_str("ok")), json_to_erl(json_obj)))
}

fn json_to_erl(json: json::Json) -> erl_ext::Eterm {
    /*
    -type json() :: float() | binary() | bool()
                    | [json()] | #{binary() => json()}.

    Json   | Erlang
    -------+------------------
    -0.23  | -0.23      // float()
    "wasd" | <<"wasd">> // binary()
    true   | true       // bool()
    []     | []         // [json()]
    {}     | {}         // #{binary() => json()}
    null   | 'undefined'
     */
    match json {
        Json::F64(num) => Eterm::Float(num),
        Json::I64(num) => Eterm::Integer(num as i32), // FIXME: BigNum
        Json::U64(num) => Eterm::Integer(num as i32), // FIXME: BigNum
        Json::String(string) => Eterm::Binary(string.into_bytes()),
        Json::Boolean(true) => Eterm::Atom(String::from_str("true")),
        Json::Boolean(false) => Eterm::Atom(String::from_str("false")),
        Json::Array(lst) => {
            let mut eterm_lst: erl_ext::List =
                lst.into_iter().map(json_to_erl).collect();
            eterm_lst.push(Eterm::Nil);
            Eterm::List(eterm_lst)
        },
        Json::Object(obj) => {
            let eterm_map: erl_ext::Map =
                obj.into_iter().map(
                    |(k, v)| {
                        let ek = Eterm::Binary(k.into_bytes());
                        let ev = json_to_erl(v);
                        (ek, ev)
                    }).collect();
            Eterm::Map(eterm_map)
        },
        Json::Null => Eterm::Atom(String::from_str("undefined")),
    }
}
