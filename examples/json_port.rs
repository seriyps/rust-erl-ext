// see json_port.erl
extern crate erl_ext;
extern crate serialize;

use erl_ext::{Binary, Tuple, Atom, Nil};
use serialize::json;
use std::io;


fn main() {
    let in_f = io::stdin().unwrap();
    let out_f = io::stdout().unwrap();
    match read_write_loop(in_f, out_f) {
        Err(io::IoError{kind: io::EndOfFile, ..}) => (), // port was closed
        Err(io::IoError{kind: kind, desc: desc, ..}) =>
            fail!("kind: {}, desc: '{}'", kind, desc),
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
                Binary(bytes) => {
                    bytes_to_json(bytes)
                },
                _ =>
                    // {error, not_binary}
                    Tuple(vec!(
                        Atom(String::from_str("error")),
                        Atom(String::from_str("not_binary"))
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
            return Tuple(vec!(
                Atom(String::from_str("error")),
                Atom(String::from_str("bad_utf8"))))
    };
    // &str to json::Json
    let json_obj = match json::from_str(json_string.as_slice()) {
        Ok(o) => o,
        Err(json::SyntaxError(err_code, _, _)) => {
            let err_str = json::error_str(err_code);
            return Tuple(vec!(
                Atom(String::from_str("error")),
                erl_ext::String(Vec::from_slice(err_str.as_bytes()))
                    ))
        },
        Err(json::IoError(_, err_str)) =>
            return Tuple(vec!(
                Atom(String::from_str("error")),
                erl_ext::String(Vec::from_slice(err_str.as_bytes()))
                    ))
    };
    // json::Json to erl_ext::Eterm
    Tuple(vec!(Atom(String::from_str("ok")), json_to_erl(json_obj)))
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
        json::Number(num) => erl_ext::Float(num),
        json::String(string) => erl_ext::Binary(string.into_bytes()),
        json::Boolean(true) => erl_ext::Atom(String::from_str("true")),
        json::Boolean(false) => erl_ext::Atom(String::from_str("false")),
        json::List(lst) => {
            let mut eterm_lst: erl_ext::List =
                lst.move_iter().map(json_to_erl).collect();
            eterm_lst.push(Nil);
            erl_ext::List(eterm_lst)
        },
        json::Object(obj) => {
            let eterm_map: erl_ext::Map =
                obj.move_iter().map(
                    |(k, v)| {
                        let ek = erl_ext::Binary(k.into_bytes());
                        let ev = json_to_erl(v);
                        (ek, ev)
                    }).collect();
            erl_ext::Map(eterm_map)
        },
        json::Null => erl_ext::Atom(String::from_str("undefined")),
    }
}
