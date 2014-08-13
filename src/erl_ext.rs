// See erts-6.1/doc/html/erl_ext_dist.html for binary format description.

// #![crate_id = "erl_ext#0.0.1"]
#![comment = "Erlang external term format codec for Rust"]
#![license = "APL2.0"]
#![crate_type = "lib"]

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
use std::num::Zero;
use num::integer::Integer;


#[deriving(FromPrimitive, Show, PartialEq)]
pub enum ErlTermTag {
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

#[deriving(Show, PartialEq, Clone)]
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


#[deriving(Show, PartialEq, Clone)]
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

pub type DecodeResult = io::IoResult<Eterm>;//, DecodeError>;

pub struct Decoder<'a> {
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
    pub fn new(rdr: &'a mut io::Reader) -> Decoder<'a> {
        Decoder{rdr: rdr}
    }
    pub fn read_prelude(&mut self) -> io::IoResult<bool> {
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
    pub fn decode_term(&mut self) -> DecodeResult {
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

pub type EncodeResult = io::IoResult<()>; // TODO: maybe return num bytes written?

pub struct Encoder<'a> {
    wrtr: &'a mut io::Writer,
    use_utf8_atoms: bool,
    use_small_atoms: bool,
    fair_new_fun: bool,
    //use_new_float: bool, (>=R11B)
}


impl<'a> Encoder<'a> {
    // TODO: asserts for overflows

    pub fn new<'a>(writer: &'a mut io::Writer, utf8_atoms: bool, small_atoms: bool, fair_new_fun: bool) -> Encoder<'a> {
        Encoder{wrtr: writer,
                use_utf8_atoms: utf8_atoms,
                use_small_atoms: small_atoms,
                fair_new_fun: fair_new_fun}
    }

    pub fn write_prelude(&mut self) -> EncodeResult {
        self.wrtr.write_u8(131)
    }

    fn encode_small_integer(&mut self, num: u8) -> EncodeResult {
        self.wrtr.write_u8(num)
    }
    fn encode_integer(&mut self, num: i32) -> EncodeResult {
        self.wrtr.write_be_i32(num)
    }
    fn encode_new_float(&mut self, num: f64) -> EncodeResult {
        self.wrtr.write_be_f64(num)
    }

    fn _encode_str(&mut self, s: String) -> EncodeResult {
        self.wrtr.write_str(s.as_slice())
    }
    fn encode_atom(&mut self, atom: Atom) -> EncodeResult {
        try!(self.wrtr.write_be_u16(atom.len() as u16));
        self._encode_str(atom)
    }
    fn encode_small_atom(&mut self, atom: Atom) -> EncodeResult {
        try!(self.wrtr.write_u8(atom.len() as u8));
        self._encode_str(atom)
    }
    fn encode_new_reference(&mut self, node: Atom, id: Vec<u8>, creation: u8) -> EncodeResult {
        let len = id.len() / 4; // todo: ensure proper rounding, maybe (id.len() / 4) + if (id.len() % 4) == 0 {0} else {1}
        try!(self.wrtr.write_be_u16(len as u16));
        try!(self.encode_term(Atom(node)));
        try!(self.wrtr.write_u8(creation));
        self.wrtr.write(id.as_slice())
    }
    fn encode_port(&mut self, node: Atom, id: u32, creation: u8) -> EncodeResult {
        try!(self.encode_term(Atom(node)));
        try!(self.wrtr.write_be_u32(id));
        self.wrtr.write_u8(creation)
    }
    fn encode_pid(&mut self, node: Atom, id: u32, serial: u32, creation: u8) -> EncodeResult {
        try!(self.encode_term(Atom(node)));
        try!(self.wrtr.write_be_u32(id));
        try!(self.wrtr.write_be_u32(serial));
        self.wrtr.write_u8(creation)
    }

    fn encode_small_tuple(&mut self, tuple: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_u8(tuple.len() as u8));
        for term in tuple.move_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_large_tuple(&mut self, tuple: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_be_u32(tuple.len() as u32));
        for term in tuple.move_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_map(&mut self, map: Map) -> EncodeResult {
        try!(self.wrtr.write_be_u32(map.len() as u32));
        for (key, val) in map.move_iter() {
            try!(self.encode_term(key));
            try!(self.encode_term(val));
        }
        Ok(())
    }
    fn encode_string(&mut self, s: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_be_u16(s.len() as u16));
        self.wrtr.write(s.as_slice())
    }
    fn encode_list(&mut self, list: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_be_u32((list.len() - 1) as u32));
        for term in list.move_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }

    fn encode_binary(&mut self, bin: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_be_u32(bin.len() as u32));
        self.wrtr.write(bin.as_slice())
    }

    fn _bigint_to_bytes(&self, num: bigint::BigInt) -> Vec<u8> {
        // there is no num.as_slice(), so, the only way to extract bytes is
        // some arithmetic operations.
        let mut bytes = Vec::new();
        let mut n = num.abs();
        let quantor: bigint::BigInt = FromPrimitive::from_u16(256).unwrap();
        while !n.is_zero() {
            let (rest, byte) = n.div_rem(&quantor);
            let byte_u8 = byte.to_u8().unwrap();
            bytes.push(byte_u8);
            n = rest;
        }
        bytes
    }
    fn _encode_big(&mut self, num: bigint::BigInt, bytes: Vec<u8>) -> EncodeResult {
        let sign = if num.is_positive() {
            0
        } else {
            1
        };
        try!(self.wrtr.write_u8(sign));
        self.wrtr.write(bytes.as_slice())
    }
    fn encode_small_big(&mut self, num: bigint::BigInt, bytes: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u8(bytes.len() as u8));
        self._encode_big(num, bytes)
    }
    fn encode_large_big(&mut self, num: bigint::BigInt, bytes: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_be_u32(bytes.len() as u32));
        self._encode_big(num, bytes)
    }

    fn encode_fun(&mut self, pid: Pid, module: Atom, index: u32, uniq: u32, free_vars: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_be_u32(free_vars.len() as u32));
        try!(self.encode_term(Pid(pid)));
        try!(self.encode_term(Atom(module)));
        try!(self.encode_term(
            if index <= 255 { SmallInteger(index as u8) }
            else { Integer(index as i32) }));
        try!(self.encode_term(
            if uniq <= 255 { SmallInteger(uniq as u8) }
            else { Integer(uniq as i32) }));
        for term in free_vars.move_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn _encode_new_fun(&mut self, arity: u8, uniq: Vec<u8>, index: u32, module: Atom, old_index: u32, old_uniq: u32, pid: Pid, free_vars: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_u8(arity));
        assert!(uniq.len() == 16);
        try!(self.wrtr.write(uniq.as_slice()));
        try!(self.wrtr.write_be_u32(index));
        try!(self.wrtr.write_be_u32(free_vars.len() as u32));
        try!(self.encode_term(Atom(module)));

        let old_index_term = if old_index <= 255 {
            SmallInteger(old_index as u8)
        } else {
            Integer(old_index as i32)
        };
        try!(self.encode_term(old_index_term));

        let old_uniq_term = if old_uniq <= 255 {
            SmallInteger(old_uniq as u8)
        } else {
            Integer(old_uniq as i32)
        };
        try!(self.encode_term(old_uniq_term));

        try!(self.encode_term(Pid(pid)));

        for term in free_vars.move_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_new_fun(&mut self, arity: u8, uniq: Vec<u8>, index: u32, module: Atom, old_index: u32, old_uniq: u32, pid: Pid, free_vars: Vec<Eterm>) -> EncodeResult {
        // We serialize to temporary memory buffer to calculate encoded term size.
        // Erlang itself in 'term_to_binary' does back-patching (see
        // erts/emulator/beam/external.c#enc_term_int 'ENC_PATCH_FUN_SIZE'), but
        // at the same time, in 'binary_to_term' this size u32 is just skipped!
        // So, we make this configurable: do fair encoding or cheating with
        // fake zero size.
        if self.fair_new_fun {
            let mut temp = io::MemWriter::new();
            {
                let mut encoder = Encoder::new(&mut temp, self.use_utf8_atoms, self.use_small_atoms, self.fair_new_fun);
                try!(encoder._encode_new_fun(arity, uniq, index, module, old_index, old_uniq, pid, free_vars));
            }
            let size = temp.get_ref().len();
            // +4 is size itself
            try!(self.wrtr.write_be_u32(4 + size as u32));
            self.wrtr.write(temp.get_ref())
        } else {
            // cheating - write 0, since binary_to_term don't use this (at least now, in 17.0)
            try!(self.wrtr.write_be_u32(0));
            self._encode_new_fun(arity, uniq, index, module, old_index, old_uniq, pid, free_vars)
        }
    }
    fn encode_export(&mut self, module: Atom, function: Atom, arity: u8) -> EncodeResult {
        try!(self.encode_term(Atom(module)));
        try!(self.encode_term(Atom(function)));
        self.encode_term(SmallInteger(arity))
    }
    fn encode_bit_binary(&mut self, bits: u8, data: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_be_u32(data.len() as u32));
        try!(self.wrtr.write_u8(bits));
        self.wrtr.write(data.as_slice())
    }

    fn _encode_tag(&mut self, tag: ErlTermTag) -> EncodeResult {
        let int_tag = tag as u8;
        self.wrtr.write_u8(int_tag)
    }
    pub fn encode_term(&mut self, term: Eterm) -> EncodeResult {
        // XXX: maybe use &Eterm, not just Eterm?
        match term {
            SmallInteger(num) => {
                try!(self._encode_tag(SMALL_INTEGER_EXT));
                self.encode_small_integer(num)
            },
            Integer(num) => {
                try!(self._encode_tag(INTEGER_EXT));
                self.encode_integer(num)
            },
            Float(num) => {
                try!(self._encode_tag(NEW_FLOAT_EXT));
                self.encode_new_float(num)
            },
            Atom(atom) => {
                let use_utf8 = self.use_utf8_atoms;
                let use_small = self.use_small_atoms;
                if (atom.len() <= 255) && use_small {
                    try!(self._encode_tag(if use_utf8 {SMALL_ATOM_UTF8_EXT} else {SMALL_ATOM_EXT}));
                    self.encode_small_atom(atom)
                } else {
                    try!(self._encode_tag(if use_utf8 {ATOM_UTF8_EXT} else {ATOM_EXT}));
                    self.encode_atom(atom)
                }
            },
            Reference{node: node, id: id, creation: creation} => {
                try!(self._encode_tag(NEW_REFERENCE_EXT));
                self.encode_new_reference(node, id, creation)
            },
            Port{node: node, id: id, creation: creation} => {
                try!(self._encode_tag(PORT_EXT));
                self.encode_port(node, id, creation)
            },
            Pid(Pid{node: node, id: id, serial: serial, creation: creation}) => {
                try!(self._encode_tag(PID_EXT));
                self.encode_pid(node, id, serial, creation)
            },
            Tuple(tuple) => {
                if tuple.len() <= 255 {
                    try!(self._encode_tag(SMALL_TUPLE_EXT));
                    self.encode_small_tuple(tuple)
                } else {
                    try!(self._encode_tag(LARGE_TUPLE_EXT));
                    self.encode_large_tuple(tuple)
                }
            },
            Map(map) => {
                try!(self._encode_tag(MAP_EXT));
                self.encode_map(map)
            },
            Nil => 
                self._encode_tag(NIL_EXT),
            String(s) => {
                try!(self._encode_tag(STRING_EXT));
                self.encode_string(s)
            },
            List(list) => {
                try!(self._encode_tag(LIST_EXT));
                self.encode_list(list)
            },
            Binary(bin) => {
                try!(self._encode_tag(BINARY_EXT));
                self.encode_binary(bin)
            },
            BigNum(num) => {
                let num_bytes = self._bigint_to_bytes(num.clone());
                if num_bytes.len() < 255 {
                    try!(self._encode_tag(SMALL_BIG_EXT));
                    self.encode_small_big(num, num_bytes)
                } else {
                    try!(self._encode_tag(LARGE_BIG_EXT))
                    self.encode_large_big(num, num_bytes)
                }
            },
            Fun{pid: pid, module: module, index: index, uniq: uniq, free_vars: free_vars} => {
                try!(self._encode_tag(FUN_EXT));
                self.encode_fun(pid, module, index, uniq, free_vars)
            },
            NewFun{arity: arity, uniq: uniq, index: index, module: module, old_index: old_index, old_uniq: old_uniq, pid: pid, free_vars: free_vars} => {
                try!(self._encode_tag(NEW_FUN_EXT));
                self.encode_new_fun(arity, uniq, index, module, old_index, old_uniq, pid, free_vars)
            },
            Export{module: module, function: function, arity: arity} => {
                try!(self._encode_tag(EXPORT_EXT));
                self.encode_export(module, function, arity)
            },
            BitBinary{bits: bits, data: data} => {
                try!(self._encode_tag(BIT_BINARY_EXT));
                self.encode_bit_binary(bits, data)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Eterm,Encoder,Decoder,DecodeResult};
    use std::io;
    use num::bigint;
    use std::num::Bounded;

    fn term_to_binary(term: Eterm) -> io::IoResult<Vec<u8>> {
        let mut writer = io::MemWriter::new();
        {
            let mut encoder = Encoder::new(&mut writer, false, false, true);
            try!(encoder.write_prelude());
            try!(encoder.encode_term(term));
        }
        Ok(writer.unwrap())
    }
    fn binary_to_term(binary: Vec<u8>) -> DecodeResult {
        let mut reader = io::MemReader::new(binary);
        let mut decoder = Decoder::new(&mut reader);
        assert!(true == try!(decoder.read_prelude()));
        decoder.decode_term()
    }

    macro_rules! codec_eq (
        ($inp:expr) => {
            {
                let orig = $inp;
                let teleported = binary_to_term(term_to_binary(orig.clone()).unwrap()).unwrap();
                assert_eq!(orig, teleported);
            }
        };
    )


    #[test]
    fn codec_small_integer() {
        codec_eq!(super::SmallInteger(0));
        codec_eq!(super::SmallInteger(255));
    }

    #[test]
    fn codec_integer() {
        codec_eq!(super::Integer(-2147483647));
        codec_eq!(super::Integer(-1));
        codec_eq!(super::Integer(256));
        codec_eq!(super::Integer(2147483647));
    }

    #[test]
    fn codec_float() {
        codec_eq!(super::Float(-111111.11));
        codec_eq!(super::Float(0.0));
        codec_eq!(super::Float(111111.11));
    }

    #[test]
    fn codec_atom() {
        codec_eq!(super::Atom(String::from_str("hello_world")));
    }

    #[test]
    fn codec_reference() {
        let node = String::from_str("my_node");
        let reference = super::Reference {
            node: node,
            id: vec!(0, 1, 2, 3),
            creation: 0
        };
        codec_eq!(reference);
    }

    #[test]
    fn codec_port() {
        codec_eq!(super::Port {
            node: String::from_str("my_node"),
            id: 4294967295,
            creation: 0
        });
    }

    #[test]
    fn codec_pid() {
        codec_eq!(super::Pid(super::Pid {
            node: String::from_str("my_node"),
            id: 4294967295,
            serial: 1,
            creation: 0
        }));
    }

    #[test]
    fn codec_tuple() {
        codec_eq!(super::Tuple(vec!(
            super::SmallInteger(0),
            super::Nil
                )));
    }

    #[test]
    fn codec_map() {
        // #{0 => {}, 0.0 => -1}
        let mut map: super::Map = Vec::new();
        map.push((super::SmallInteger(0), super::Tuple(vec!())));
        map.push((super::Float(0.0), super::Integer(-1)));
        let emap = super::Map(map);
        codec_eq!(emap);
    }

    #[test]
    fn codec_nil() {
        codec_eq!(super::Nil);
    }

    #[test]
    fn codec_string() {
        codec_eq!(super::String(Vec::from_fn(255, |i| i as u8)));
    }

    #[test]
    fn codec_list() {
        codec_eq!(super::List(vec!(
            super::Tuple(vec!()),
            super::SmallInteger(1),
            super::Nil,
            )));
    }

    #[test]
    fn codec_binary() {
        codec_eq!(super::Binary(Vec::from_fn(1024, |i| (i % 255) as u8)));
    }

    #[test]
    fn codec_big_num() {
        codec_eq!(super::BigNum(bigint::BigInt::new(bigint::Plus, vec!(1, 1, 1, 1, 1, 1))));
        codec_eq!(super::BigNum(bigint::BigInt::new(bigint::Minus, vec!(1, 1, 1, 1, 1, 1))));
        codec_eq!(super::BigNum(FromPrimitive::from_i64(Bounded::max_value()).unwrap()));
        codec_eq!(super::BigNum(bigint::BigInt::new(bigint::Plus, Vec::from_fn(256, |i| i as u32))));
    }

    #[test]
    fn codec_fun() {
        let pid = super::Pid {
            node: String::from_str("my_node"),
            id: 4294967295,
            serial: 1,
            creation: 0
        };
        codec_eq!(super::Fun {
            pid: pid,
            module: String::from_str("my_mod"),
            index: 1,
            uniq: Bounded::max_value(),
            free_vars: vec!(super::Nil)
        });
    }

    #[test]
    fn codec_new_fun() {
        let pid = super::Pid {
            node: String::from_str("my_node"),
            id: Bounded::max_value(),
            serial: 1,
            creation: 0
        };
        codec_eq!(super::NewFun {
            arity: 128,         // :-)
            uniq: Vec::from_fn(16, |i| i as u8),
            index: Bounded::max_value(),
            module: String::from_str("my_mod"),
            old_index: Bounded::max_value(),
            old_uniq: Bounded::max_value(),
            pid: pid,
            free_vars: vec!(super::Nil)
        });
    }

    #[test]
    fn codec_export() {
        codec_eq!(super::Export {
            module: String::from_str("my_mod"),
            function: String::from_str("my_fun"),
            arity: Bounded::max_value()
        });
    }

    #[test]
    fn codec_bit_binary() {
        codec_eq!(super::BitBinary {
            bits: 1,
            data: vec!(255, 255)
        });
    }
}
