// see json_port.erl
#![feature(rustc_private)]

extern crate erl_ext;
extern crate serialize;
extern crate num;
extern crate byteorder;

// use std::num::ToPrimitive;
use std::io;
use std::io::Write;

use num::bigint::ToBigInt;
use num::traits::FromPrimitive;
use num::traits::ToPrimitive;
use serialize::json::{self, Json}; // TODO: use rustc-serialize
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use erl_ext::{Eterm, Error};


fn main() {
    let in_f = io::stdin();
    let out_f = io::stdout();
    match read_write_loop(in_f, out_f) {
        Err(Error::ByteorderUnexpectedEOF) => (), // port was closed
        Err(ref err) =>
            panic!("Error: '{}'", err),
        Ok(()) => ()            // unreachable in this example
    };
}

fn read_write_loop<R: io::Read, W: io::Write>(mut r: R, mut w: W) -> Result<(), Error> {
    loop {
        // {packet, 2}
        let _in_packet_size = r.read_u16::<BigEndian>();
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
                        Eterm::Atom(String::from("error")),
                        Eterm::Atom(String::from("not_binary"))
                        ))
            };
            // Temp buffer to calculate response term size
            let mut wrtr = Vec::new();
            {
                // encode response term
                let mut encoder = erl_ext::Encoder::new(&mut wrtr,
                                                    true, true, true);
                try!(encoder.write_prelude());
                try!(encoder.encode_term(response));
                try!(encoder.flush());
            }
            // response packet size
            let out_packet_size = wrtr.len() as u16;
            try!(w.write_u16::<BigEndian>(out_packet_size));
            // response term itself
            try!(w.write_all(wrtr.as_ref()));
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
                Eterm::Atom(String::from("error")),
                Eterm::Atom(String::from("bad_utf8"))))
    };
    // &str to json::Json
    let json_obj = match json::from_str(json_string.as_ref()) {
        Ok(o) => o,
        Err(json::ParserError::SyntaxError(err_code, _, _)) => {
            let err_str = json::error_str(err_code);
            return Eterm::Tuple(vec!(
                Eterm::Atom(String::from("error")),
                Eterm::String(err_str.as_bytes().to_vec())
                    ))
        },
        Err(json::ParserError::IoError(_, err_str)) =>
            return Eterm::Tuple(vec!(
                Eterm::Atom(String::from("error")),
                Eterm::String(err_str.as_bytes().to_vec())
                    ))
    };
    // json::Json to erl_ext::Eterm
    Eterm::Tuple(vec!(Eterm::Atom(String::from("ok")), json_to_erl(json_obj)))
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
        Json::I64(num) if (num <= (i32::max_value() as i64) && num >= (i32::min_value() as i64)) =>
            Eterm::Integer(num as i32),
        Json::I64(num) =>
            Eterm::BigNum(num.to_bigint().unwrap()),
        Json::U64(num) => {
            match num.to_i32() {
                Some(i32_num) => Eterm::Integer(i32_num),
                None => Eterm::BigNum(FromPrimitive::from_u64(num).unwrap())
            }
        },
        Json::String(string) => Eterm::Binary(string.into_bytes()),
        Json::Boolean(true) => Eterm::Atom(String::from("true")),
        Json::Boolean(false) => Eterm::Atom(String::from("false")),
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
        Json::Null => Eterm::Atom(String::from("undefined")),
    }
}
