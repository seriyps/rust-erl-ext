// See erts-6.1/doc/html/erl_ext_dist.html

#![feature(struct_variant)]     // this is for enum Eterm
#![allow(non_camel_case_types)] // this is for enum ErlTermTag
#![feature(macro_rules)]        // for try_io! and decode_some!

extern crate num;
extern crate collections;
extern crate serialize;

use std::string::String;
use std::vec::Vec;
use std::num::FromPrimitive;
use std::io;
use num::bigint;
// use serialize::{Encodable, Decodable};


#[deriving(FromPrimitive, Show, PartialEq)]
enum ErlTermTag {
    // ATOM_CACHE_REF = 82,
    SMALL_INTEGER_EXT = 97,
    INTEGER_EXT = 98,
    FLOAT_EXT = 99,
    ATOM_EXT = 100,
    REFERENCE_EXT = 101,
    PORT_EXT = 102,
    PID_EXT = 103,
    SMALL_TUPLE_EXT = 104,
    LARGE_TUPLE_EXT = 105,
    MAP_EXT = 116,
    NIL_EXT = 106,
    STRING_EXT = 107,
    LIST_EXT = 108,
    BINARY_EXT = 109,
    SMALL_BIG_EXT = 110,
    LARGE_BIG_EXT = 111,
    NEW_REFERENCE_EXT = 114,
    SMALL_ATOM_EXT = 115,
    FUN_EXT = 117,
    NEW_FUN_EXT = 112,
    EXPORT_EXT = 113,
    BIT_BINARY_EXT = 77,
    NEW_FLOAT_EXT = 70,
    ATOM_UTF8_EXT = 118,
    SMALL_ATOM_UTF8_EXT = 119,
}

#[deriving(Show)]
pub enum Eterm {
    SmallInteger(u8),           // small_integer
    Integer(i32),               // integer
    Float(f64),                 // float, new_float
    Atom(Atom),                 // atom, small_atom, atom_utf8, small_atom_utf8
    Reference {                 // reference, new_reference
        node: Atom,
        id: Vec<u8>,
        creation: u8},
    Port {                      // poort
        node: Atom,
        id: u32,
        creation: u8},
    Pid(Pid),                   // pid
    Tuple(Tuple),               // small_tuple, large_tuple
    Map(Map),                   // map
    Nil,                        // nil
    String(Vec<u8>),            // string; it's not String, because not guaranteed to be valid UTF-8
    List(List),                 // list
    Binary(Vec<u8>),            // binary
    BigNum(bigint::BigInt),     // small_big, large_big
    Fun {                       // fun
        pid: Pid,
        module: Atom,
        index: u32,
        uniq: u32,
        free_vars: Vec<Eterm>},
    NewFun {                    // new_fun
        arity: u8,
        uniq: Vec<u8>, //[u8, ..16],
        index: u32,
        module: Atom,
        old_index: u32,
        old_uniq: u32,
        pid: Pid,
        free_vars: Vec<Eterm>},
    Export {                    // export
        module: Atom,
        function: Atom,
        arity: u8,
    },
    BitBinary {                 // bit_binary; maybe implement .to_bitv() -> Bitv for it?
        bits: u8,
        data: Vec<u8>,
    },
}
pub type Atom = String;
pub type Tuple = Vec<Eterm>;
pub type Map = Vec<(Eterm, Eterm)>; // k-v pairs
pub type List = Vec<Eterm>;


#[deriving(Show)]
pub struct Pid {                // moved out from enum because it used in Eterm::{Fun,NewFun}
    node: Atom,
    id: u32,
    serial: u32,                // maybe [u8, ..4]?
    creation: u8,
}


// enum DecodeError {
//     IoError(io::IoError),
//     TagNotImplemented(u8, ErlTermTag),
//     InvalidTag(u8),
//     UnexpectedTerm(ErlTermTag,       // got
//                    Vec<ErlTermTag>), // expected one of
//     BadUTF8(Vec<u8>),
//     BadFloat(Vec<u8>),
// }

type DecodeResult = io::IoResult<Eterm>;//, DecodeError>;

struct Decoder<'a> {
    rdr: &'a mut io::Reader,
}

// macro_rules! try_io(
//     ($e:expr) => (
//         match $e {
//             Ok(e) => e,
//             Err(e) => return IoError(e)
//         }
//         )
// )

macro_rules! decode_some(
    ($e:expr, $($t:ident),+ ) => (
        {
            match try!($e._decode_tag()) {
                $(
                    $t =>
                        try!($e.decode_concrete_term($t)),
                    )+
                    bad =>
                    return Err(io::IoError {
                        kind: io::OtherIoError,
                        desc: "Assertion failed, unexpected tag",
                        detail: Some(format!("Got {}", bad)),
                    })
            }
        }
        )
)

impl<'a> Decoder<'a> {
    fn new(rdr: &'a mut io::Reader) -> Decoder<'a> {
        Decoder{rdr: rdr}
    }
    fn read_prelude(&mut self) -> io::IoResult<bool> {
        Ok(131 == try!(self.rdr.read_u8()))
    }
    fn decode_small_integer(&mut self) -> DecodeResult {
        Ok(SmallInteger(try!(self.rdr.read_u8())))
    }
    fn decode_integer(&mut self) -> DecodeResult {
        Ok(Integer(try!(self.rdr.read_be_i32())))
    }
    fn _read_str(&mut self, len: uint) -> io::IoResult<String> {
        let utf8 = try!(self.rdr.read_exact(len));
        match String::from_utf8(utf8) {
            Ok(s) => Ok(s),
            Err(_) =>
                return Err(io::IoError{
                    kind: io::OtherIoError,
                    desc: "Bad utf-8",
                    detail: None
                })
        }
    }
    fn decode_float(&mut self) -> DecodeResult {
        let float_str = try!(self._read_str(31));
        match from_str::<f32>(float_str.as_slice()) {
            Some(num) => Ok(Float(num as f64)),
            _ =>
                Err(io::IoError{
                    kind: io::OtherIoError,
                    desc: "Bad float",
                    detail: None // format!("{}", data)
                })
        }
    }
    fn _decode_any_atom(&mut self) -> DecodeResult {
        match try!(self._decode_tag()) {
            ATOM_EXT | ATOM_UTF8_EXT => self.decode_atom(),
            SMALL_ATOM_EXT | SMALL_ATOM_UTF8_EXT => self.decode_small_atom(),
            _ =>
                return Err(io::IoError {
                    kind: io::OtherIoError,
                    desc: "Assertion failed, unexpected tag",
                    detail: None
                })
        }
    }
    fn decode_atom(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_be_u16());
        let atom_str = try!(self._read_str(len as uint));
        // XXX: data is in latin1 in case of ATOM_EXT
        Ok(Atom(atom_str))
    }
    fn decode_reference(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self.rdr.read_exact(4));
        let creation = try!(self.rdr.read_u8());
        Ok(Reference {
            node: node,
            id: id,
            creation: creation
        })
    }
    fn decode_port(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self.rdr.read_be_u32());
        let creation = try!(self.rdr.read_u8());
        Ok(Port {
            node: node,
            id: id,
            creation: creation
        })
    }
    fn decode_pid(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self.rdr.read_be_u32());
        let serial = try!(self.rdr.read_be_u32());
        let creation = try!(self.rdr.read_u8());
        Ok(Pid(Pid {
            node: node,
            id: id,
            serial: serial,
            creation: creation
        }))
    }

    fn _decode_small_tuple_arity(&mut self) -> io::IoResult<u8> {
        self.rdr.read_u8()
    }
    fn decode_small_tuple(&mut self) -> DecodeResult {
        let arity = try!(self._decode_small_tuple_arity());
        let mut tuple: Tuple = Vec::with_capacity(arity as uint);
        for _ in range(0, arity) {
            let term = try!(self.decode_term());
            tuple.push(term)
        }
        Ok(Tuple(tuple))
    }

    fn _decode_large_tuple_arity(&mut self) -> io::IoResult<u32> {
        self.rdr.read_be_u32()
    }
    fn decode_large_tuple(&mut self) -> DecodeResult {
        let arity = try!(self._decode_large_tuple_arity());
        let mut tuple: Tuple = Vec::with_capacity(arity as uint);
        for _ in range(0, arity) {
            let term = try!(self.decode_term());
            tuple.push(term)
        }
        Ok(Tuple(tuple))
    }

    fn _decode_map_arity(&mut self) -> io::IoResult<u32> {
        self.rdr.read_be_u32()
    }
    fn decode_map(&mut self) -> DecodeResult {
        let arity: u32 = try!(self._decode_map_arity());
        let mut map: Map = Vec::with_capacity(arity as uint);
        for _ in range(0, arity) {
            let key = try!(self.decode_term());
            let val = try!(self.decode_term());
            map.push((key, val))
        }
        Ok(Map(map))
    }
    fn decode_nil(&mut self) -> DecodeResult {
        Ok(Nil)
    }
    fn decode_string(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_be_u16());
        Ok(String(try!(self.rdr.read_exact(len as uint))))
    }

    fn _decode_list_len(&mut self) -> io::IoResult<u32> {
        self.rdr.read_be_u32()
    }
    fn decode_list(&mut self) -> DecodeResult {
        // XXX: should we push Nil as last element or may ignore it?
        let len = try!(self._decode_list_len()) + 1;
        let mut list = Vec::with_capacity(len as uint);
        for _ in range(0, len) {
            let term = try!(self.decode_term());
            list.push(term)
        }
        Ok(List(list))
    }
    fn decode_binary(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_be_u32());
        Ok(Binary(try!(self.rdr.read_exact(len as uint))))
    }
    fn _decode_big(&mut self, n: uint) -> DecodeResult {
        let sign_int = try!(self.rdr.read_u8());
        let sign = if sign_int == 0 {
            bigint::Plus
        } else {
            bigint::Minus
        };
        // In erlang:
        // B = 256 % base is 2^8
        // (d0*B^0 + d1*B^1 + d2*B^2 + ... d(N-1)*B^(n-1))
        // In rust:
        // BigDigit::base is 2^32
        // (a + b * BigDigit::base + c * BigDigit::base^2)
        let mut numbers = Vec::<u32>::with_capacity((n / 4) as uint);
        let mut cur_num: u32 = 0;
        for i in range(0, n) {
            let byte = try!(self.rdr.read_u8()) as u32;
            cur_num = match i % 4 {
                0 => cur_num + byte,
                1 => cur_num + byte * 256,
                2 => cur_num + byte * 65536,
                _ => {
                    numbers.push(cur_num + byte * 16777216);
                    0
                }
            }
        }
        if cur_num != 0 { // if 'n' isn't multiple of 4
            numbers.push(cur_num)
        }
        Ok(BigNum(bigint::BigInt::new(sign, numbers)))
    }
    fn decode_small_big(&mut self) -> DecodeResult {
        let n = try!(self.rdr.read_u8());
        self._decode_big(n as uint)
    }
    fn decode_large_big(&mut self) -> DecodeResult {
        let n = try!(self.rdr.read_be_u32());
        self._decode_big(n as uint)
    }
    fn decode_new_reference(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_be_u16());
        let node = match try!(self._decode_any_atom()) {
            Atom(a) => a,
            _ => unreachable!()
        };
        let creation = try!(self.rdr.read_u8());
        let id = try!(self.rdr.read_exact(4 * len as uint));
        Ok(Reference{
            node: node,
            id: id, // here id should be Vec<u32>, but since it's not interpreted, leave it as is
            creation: creation
        })
    }
    fn decode_small_atom(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u8());
        let atom_str = try!(self._read_str(len as uint));
        // XXX: data is in latin1 in case of SMALL_ATOM_EXT
        Ok(Atom(atom_str))
    }
    fn decode_fun(&mut self) -> DecodeResult {
        // TODO: cleanup error handling (generalize)
        let num_free = try!(self.rdr.read_be_u32());
        let pid = match decode_some!(self, PID_EXT) {
            Pid(pid) => pid,
            _ => unreachable!()
        };
        let module = match try!(self._decode_any_atom()) {
            Atom(atom) => atom,
            _ => unreachable!()
        };
        let index = match decode_some!(self, SMALL_INTEGER_EXT, INTEGER_EXT) {
            SmallInteger(idx) => idx as u32,
            Integer(idx) => idx as u32,
            _ => unreachable!()
        };
        let uniq = match decode_some!(self, SMALL_INTEGER_EXT, INTEGER_EXT) {
            SmallInteger(uq) => uq as u32,
            Integer(uq) => uq as u32,
            _ => unreachable!()
        };
        let mut free_vars = Vec::<Eterm>::with_capacity(num_free as uint);
        for _ in range(0, num_free) {
            free_vars.push(try!(self.decode_term()));
        }
        Ok(Fun {
            pid: pid,
            module: module,
            index: index,
            uniq: uniq,
            free_vars: free_vars,
        })
    }
    fn decode_new_fun(&mut self) -> DecodeResult {
        let _size = try!(self.rdr.read_be_u32());
        let arity = try!(self.rdr.read_u8());
        let uniq = try!(self.rdr.read_exact(16));
        let index = try!(self.rdr.read_be_u32());
        let num_free = try!(self.rdr.read_be_u32());

        let module = match try!(self._decode_any_atom()) {
            Atom(atom) => atom,
            _ => unreachable!()
        };
        let old_index = match decode_some!(self, SMALL_INTEGER_EXT, INTEGER_EXT) {
            SmallInteger(idx) => idx as u32,
            Integer(idx) => idx as u32,
            _ => unreachable!()
        };
        let old_uniq = match decode_some!(self, SMALL_INTEGER_EXT, INTEGER_EXT) {
            SmallInteger(uq) => uq as u32,
            Integer(uq) => uq as u32,
            _ => unreachable!()
        };
        let pid = match decode_some!(self, PID_EXT) {
            Pid(pid) => pid,
            _ => unreachable!()
        };
        let mut free_vars = Vec::<Eterm>::with_capacity(num_free as uint);
        for _ in range(0, num_free) {
            free_vars.push(try!(self.decode_term()));
        }
        Ok(NewFun {
            arity: arity,
            uniq: uniq,
            index: index,
            module: module,
            old_index: old_index,
            old_uniq: old_uniq,
            pid: pid,
            free_vars: free_vars,
        })
    }
    fn decode_export(&mut self) -> DecodeResult {
        let module = match try!(self._decode_any_atom()) {
            Atom(atom) => atom,
            _ => unreachable!()
        };
        let function = match try!(self._decode_any_atom()) {
            Atom(atom) => atom,
            _ => unreachable!()
        };
        let arity = match decode_some!(self, SMALL_INTEGER_EXT) {
            SmallInteger(uq) => uq,
            _ => unreachable!()
        };
        Ok(Export {
            module: module,
            function: function,
            arity: arity, // arity > u8 possible in practice
        })
    }
    fn decode_bit_binary(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_be_u32()) as uint;
        let bits = try!(self.rdr.read_u8());
        Ok(BitBinary {
            bits: bits,
            data: try!(self.rdr.read_exact(len)),
        })
    }
    fn decode_new_float(&mut self) -> DecodeResult {
        Ok(Float(try!(self.rdr.read_be_f64())))
    }


    fn _decode_tag(&mut self) -> io::IoResult<ErlTermTag> {
        let int_tag = try!(self.rdr.read_u8());
        let tag: Option<ErlTermTag> = FromPrimitive::from_u8(int_tag);
        match tag {
            Some(t) => Ok(t),
            None =>
                Err(io::IoError{
                    kind: io::OtherIoError,
                    desc: "Invalid term type",
                    detail: Some(format!("Tag: #{}", int_tag))
                })
        }
    }
    fn decode_term(&mut self) -> DecodeResult {
        let tag = try!(self._decode_tag());
        self.decode_concrete_term(tag)
    }
    fn decode_concrete_term(&mut self, tag: ErlTermTag) -> DecodeResult {
        match tag {
            SMALL_INTEGER_EXT => self.decode_small_integer(),
            INTEGER_EXT => self.decode_integer(),
            FLOAT_EXT => self.decode_float(),
            ATOM_EXT | ATOM_UTF8_EXT => self.decode_atom(),
            REFERENCE_EXT => self.decode_reference(),
            PORT_EXT => self.decode_port(),
            PID_EXT => self.decode_pid(),
            SMALL_TUPLE_EXT => self.decode_small_tuple(),
            LARGE_TUPLE_EXT => self.decode_large_tuple(),
            MAP_EXT => self.decode_map(),
            NIL_EXT => self.decode_nil(),
            STRING_EXT => self.decode_string(),
            LIST_EXT => self.decode_list(),
            BINARY_EXT => self.decode_binary(),
            SMALL_BIG_EXT => self.decode_small_big(),
            LARGE_BIG_EXT => self.decode_large_big(),
            NEW_REFERENCE_EXT => self.decode_new_reference(),
            SMALL_ATOM_EXT | SMALL_ATOM_UTF8_EXT => self.decode_small_atom(),
            FUN_EXT => self.decode_fun(),
            NEW_FUN_EXT => self.decode_new_fun(),
            EXPORT_EXT => self.decode_export(),
            BIT_BINARY_EXT => self.decode_bit_binary(),
            NEW_FLOAT_EXT => self.decode_new_float(),
        }
    }
}

fn main() {
    use std::io::{File,BufferedReader};

    for i in range(70, 120) {
        let tag: Option<ErlTermTag> = FromPrimitive::from_int(i);
        println!("{} => {:?}", i, tag);
    }
    println!("==============================");
    let map = vec!( (Atom("my_map_key".to_string()), Nil) );

    let term: Eterm = NewFun {
        arity: 3,
        uniq: vec!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16),
        index: 10,
        module: "my_mod".to_string(),
        old_index: 1212,
        old_uniq: 1234,
        pid: Pid {node: "wasd".to_string(), id: 1, serial: 123, creation: 2},
        free_vars: vec!(//Float(3.14),
                        Nil,
                        Binary(vec!(1, 2, 3, 4)),
                        Export {
                            module: "my_mod".to_string(),
                            function: "my_func".to_string(),
                            arity: 4},
                        List(vec!(SmallInteger(1), Integer(1000000), Nil)),
                        Tuple(vec!(Atom("record".to_string()), Map(map))),
                        ),
    };
    println!("{}", term);
    println!("==============================");
    let f = BufferedReader::new(File::open(&Path::new("test/test_terms.bin")));
    let mut builder = Decoder::new(f);
    match builder.read_prelude() {
        Ok(true) =>
            println!("Valid eterm"),
        Ok(false) =>
            println!("Invalid eterm!"),
        Err(io::IoError{desc: d, ..}) => {
            println!("IoError: {}", d);
            return
        }
    }
    println!("{}", builder.decode_term());
}
