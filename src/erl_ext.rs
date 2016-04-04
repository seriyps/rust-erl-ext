// See erts-6.1/doc/html/erl_ext_dist.html for binary format description.

#![crate_type = "lib"]

#![allow(non_camel_case_types)] // this is for enum ErlTermTag

extern crate num;
extern crate byteorder;

use std::string::String;
use std::vec::Vec;
use std::io;
use std::io::Read;
use std::{error, fmt};
use std::mem::transmute;

use num::FromPrimitive;
use num::bigint;
use std::num::ParseFloatError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};


#[derive(Debug, PartialEq)]
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

// https://www.reddit.com/r/rust/comments/36pgn9/integer_to_enum_after_removal_of_fromprimitive/
impl ErlTermTag {
    fn from_u8(t: u8) -> Option<ErlTermTag> {
        if (t <= 119 && t >= 94) || (t == 77) || (t == 70) {
            Some(unsafe { transmute(t) })
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Eterm {
    SmallInteger(u8),           // small_integer
    Integer(i32),               // integer
    Float(f64),                 // float, new_float
    Atom(Atom),                 // atom, small_atom, atom_utf8, small_atom_utf8
    Reference(Reference),       // reference, new_reference TODO
    Port(Port),                 // poort TODO
    Pid(Pid),                   // pid
    Tuple(Tuple),               // small_tuple, large_tuple
    Map(Map),                   // map
    Nil,                        // nil
    String(Vec<u8>),            // string; it's not String, because not guaranteed to be valid UTF-8
    List(List),                 // list
    Binary(Vec<u8>),            // binary
    BigNum(bigint::BigInt),     // small_big, large_big
    Fun(Fun),                   // fun TODO
    NewFun(NewFun),             // new_fun TODO
    Export(Export),             // export TODO
    BitBinary(BitBinary),       // bit_binary; maybe implement .to_bitv() -> Bitv for it? TODO
}
pub type Atom = String;
pub type Tuple = Vec<Eterm>;
pub type Map = Vec<(Eterm, Eterm)>; // k-v pairs
pub type List = Vec<Eterm>;

#[derive(Debug, PartialEq, Clone)]
pub struct Reference {
    node: Atom,
    id: Vec<u8>,
    creation: u8
}
#[derive(Debug, PartialEq, Clone)]
pub struct Port {
    node: Atom,
    id: u32,
    creation: u8,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Pid {
    node: Atom,
    id: u32,
    serial: u32,                // maybe [u8, ..4]?
    creation: u8,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Fun {
    pid: Pid,
    module: Atom,
    index: u32,
    uniq: u32,
    free_vars: Vec<Eterm>
}
#[derive(Debug, PartialEq, Clone)]
pub struct NewFun {
    arity: u8,
    uniq: Vec<u8>, //[u8, ..16],
    index: u32,
    module: Atom,
    old_index: u32,
    old_uniq: u32,
    pid: Pid,
    free_vars: Vec<Eterm>
}
#[derive(Debug, PartialEq, Clone)]
pub struct Export {
    module: Atom,
    function: Atom,
    arity: u8,
}
#[derive(Debug, PartialEq, Clone)]
pub struct BitBinary {                 // bit_binary; maybe implement .to_bitv() -> Bitv for it? TODO
    bits: u8,
    data: Vec<u8>,
}


#[derive(Debug)]
pub enum Error {
    UnexpectedTerm(ErlTermTag),     // expected other term inside container
    UnknownTag(u8),                 // invalid term ID
    ByteorderUnexpectedEOF,         // byteorder error
    BadFloat(ParseFloatError), // invalid float, encoded as string
    Io(io::Error),                  // io error
}

impl From<io::Error> for Error {
    fn from (err: io::Error) -> Error { Error::Io(err) }
}
impl From<ParseFloatError> for Error {
    fn from (err: ParseFloatError) -> Error { Error::BadFloat(err) }
}
impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::BadFloat(_) => "Can't parse float, encoded as string",
            Error::UnexpectedTerm(_) => "Expected other term as a part of other complex term",
            Error::UnknownTag(_) => "Unknown term tag ID",
            Error::ByteorderUnexpectedEOF => "Not enough bytes to parse multibyte value",
            Error::Io(ref err) => error::Error::description(err),
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => err.cause(),
            _ => None
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadFloat(ref val) => write!(f, "Bad float '{}'.", val),
            Error::UnexpectedTerm(ref val) => write!(f, "Got '{:?}', but expected other term", val),
            Error::UnknownTag(ref val) => write!(f, "Unknown term tag ID: '{}'", val),
            Error::ByteorderUnexpectedEOF => write!(f, "Not enough bytes to parse multibyte value"),
            Error::Io(ref err) => err.fmt(f),
        }
    }
}

pub type DecodeResult = Result<Eterm, Error>;

pub struct Decoder<'a, T: ?Sized + io::Read + 'a> {
    rdr: &'a mut T,
}

macro_rules! decode_some(
    ($e:expr, $($t:path),+ ) => (
        {
            match try!($e._decode_tag()) {
                $(
                    $t =>
                        try!($e.decode_concrete_term($t)),
                    )+
                    bad =>
                    return Err(Error::UnexpectedTerm(bad))
            }
        }
        )
);

impl<'a, T> Decoder<'a, T> where T: io::Read + 'a {
    pub fn new(rdr: &'a mut T) -> Decoder<'a, T> {
        Decoder{rdr: rdr}
    }
    pub fn read_prelude(&mut self) -> Result<bool, Error> {
        Ok(131 == try!(self.rdr.read_u8()))
    }
    fn decode_small_integer(&mut self) -> DecodeResult {
        Ok(Eterm::SmallInteger(try!(self.rdr.read_u8())))
    }
    fn decode_integer(&mut self) -> DecodeResult {
        Ok(Eterm::Integer(try!(self.rdr.read_i32::<BigEndian>())))
    }
    fn _read_exact(&mut self, len: u64) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(len as usize);
        try!(io::copy(&mut self.rdr.take(len), &mut buf));
        Ok(buf)
    }
    fn _read_str(&mut self, len: usize) -> io::Result<String> {
        let mut str_buf = String::with_capacity(len);
        try!(self.rdr.take(len as u64).read_to_string(&mut str_buf));
        Ok(str_buf)
    }
    fn decode_float(&mut self) -> DecodeResult {
        let float_str = try!(self._read_str(31));
        let num = try!(float_str.parse::<f32>());
        Ok(Eterm::Float(num as f64))
    }
    fn _decode_any_atom(&mut self) -> DecodeResult {
        match try!(self._decode_tag()) {
            ErlTermTag::ATOM_EXT | ErlTermTag::ATOM_UTF8_EXT => self.decode_atom(),
            ErlTermTag::SMALL_ATOM_EXT | ErlTermTag::SMALL_ATOM_UTF8_EXT => self.decode_small_atom(),
            tag =>
                Err(Error::UnexpectedTerm(tag))
        }
    }
    fn decode_atom(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u16::<BigEndian>());
        let atom_str = try!(self._read_str(len as usize));
        // XXX: data is in latin1 in case of ATOM_EXT
        Ok(Eterm::Atom(atom_str))
    }
    fn decode_reference(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Eterm::Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self._read_exact(4));
        let creation = try!(self.rdr.read_u8());
        Ok(Eterm::Reference(Reference {
            node: node,
            id: id,
            creation: creation
        }))
    }
    fn decode_port(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Eterm::Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self.rdr.read_u32::<BigEndian>());
        let creation = try!(self.rdr.read_u8());
        Ok(Eterm::Port(Port {
            node: node,
            id: id,
            creation: creation
        }))
    }
    fn decode_pid(&mut self) -> DecodeResult {
        let node = match try!(self._decode_any_atom()) {
            Eterm::Atom(a) => a,
            _ => unreachable!()
        };
        let id = try!(self.rdr.read_u32::<BigEndian>());
        let serial = try!(self.rdr.read_u32::<BigEndian>());
        let creation = try!(self.rdr.read_u8());
        Ok(Eterm::Pid(Pid {
            node: node,
            id: id,
            serial: serial,
            creation: creation
        }))
    }

    fn _decode_small_tuple_arity(&mut self) -> io::Result<u8> {
        self.rdr.read_u8()
    }
    fn decode_small_tuple(&mut self) -> DecodeResult {
        let arity = try!(self._decode_small_tuple_arity());
        let mut tuple: Tuple = Vec::with_capacity(arity as usize);
        for _ in 0..arity {
            let term = try!(self.decode_term());
            tuple.push(term)
        }
        Ok(Eterm::Tuple(tuple))
    }

    fn _decode_large_tuple_arity(&mut self) -> io::Result<u32> {
        self.rdr.read_u32::<BigEndian>()
    }
    fn decode_large_tuple(&mut self) -> DecodeResult {
        let arity = try!(self._decode_large_tuple_arity());
        let mut tuple: Tuple = Vec::with_capacity(arity as usize);
        for _ in 0..arity {
            let term = try!(self.decode_term());
            tuple.push(term)
        }
        Ok(Eterm::Tuple(tuple))
    }

    fn _decode_map_arity(&mut self) -> io::Result<u32> {
        self.rdr.read_u32::<BigEndian>()
    }
    fn decode_map(&mut self) -> DecodeResult {
        let arity: u32 = try!(self._decode_map_arity());
        let mut map: Map = Vec::with_capacity(arity as usize);
        for _ in 0..arity {
            let key = try!(self.decode_term());
            let val = try!(self.decode_term());
            map.push((key, val))
        }
        Ok(Eterm::Map(map))
    }
    fn decode_nil(&mut self) -> DecodeResult {
        Ok(Eterm::Nil)
    }
    fn decode_string(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u16::<BigEndian>());
        Ok(Eterm::String(try!(self._read_exact(len as u64))))
    }

    fn _decode_list_len(&mut self) -> io::Result<u32> {
        self.rdr.read_u32::<BigEndian>()
    }
    fn decode_list(&mut self) -> DecodeResult {
        // XXX: should we push Nil as last element or may ignore it?
        let len = try!(self._decode_list_len()) + 1;
        let mut list = Vec::with_capacity(len as usize);
        for _ in 0..len {
            let term = try!(self.decode_term());
            list.push(term)
        }
        Ok(Eterm::List(list))
    }
    fn decode_binary(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u32::<BigEndian>());
        Ok(Eterm::Binary(try!(self._read_exact(len as u64))))
    }
    fn _decode_big(&mut self, n: usize) -> DecodeResult {
        let sign_int = try!(self.rdr.read_u8());
        let sign = if sign_int == 0 {
            bigint::Sign::Plus
        } else {
            bigint::Sign::Minus
        };
        let bytes = try!(self._read_exact(n as u64));
        Ok(Eterm::BigNum(bigint::BigInt::from_bytes_le(sign, bytes.as_ref())))
    }
    fn decode_small_big(&mut self) -> DecodeResult {
        let n = try!(self.rdr.read_u8());
        self._decode_big(n as usize)
    }
    fn decode_large_big(&mut self) -> DecodeResult {
        let n = try!(self.rdr.read_u32::<BigEndian>());
        self._decode_big(n as usize)
    }
    fn decode_new_reference(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u16::<BigEndian>()) as u64;
        let node = match try!(self._decode_any_atom()) {
            Eterm::Atom(a) => a,
            _ => unreachable!()
        };
        let creation = try!(self.rdr.read_u8());
        let id = try!(self._read_exact(4 * len));
        Ok(Eterm::Reference(Reference {
            node: node,
            id: id, // here id should be Vec<u32>, but since it's not interpreted, leave it as is
            creation: creation
        }))
    }
    fn decode_small_atom(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u8());
        let atom_str = try!(self._read_str(len as usize));
        // XXX: data is in latin1 in case of SMALL_ATOM_EXT
        Ok(Eterm::Atom(atom_str))
    }
    fn decode_fun(&mut self) -> DecodeResult {
        let num_free = try!(self.rdr.read_u32::<BigEndian>());
        let pid = match decode_some!(self, ErlTermTag::PID_EXT) {
            Eterm::Pid(pid) => pid,
            _ => unreachable!()
        };
        let module = match try!(self._decode_any_atom()) {
            Eterm::Atom(atom) => atom,
            _ => unreachable!()
        };
        let index = match decode_some!(self, ErlTermTag::SMALL_INTEGER_EXT, ErlTermTag::INTEGER_EXT) {
            Eterm::SmallInteger(idx) => idx as u32,
            Eterm::Integer(idx) => idx as u32,
            _ => unreachable!()
        };
        let uniq = match decode_some!(self, ErlTermTag::SMALL_INTEGER_EXT, ErlTermTag::INTEGER_EXT) {
            Eterm::SmallInteger(uq) => uq as u32,
            Eterm::Integer(uq) => uq as u32,
            _ => unreachable!()
        };
        let mut free_vars = Vec::<Eterm>::with_capacity(num_free as usize);
        for _ in 0..num_free {
            free_vars.push(try!(self.decode_term()));
        }
        Ok(Eterm::Fun(Fun {
            pid: pid,
            module: module,
            index: index,
            uniq: uniq,
            free_vars: free_vars,
        }))
    }
    fn decode_new_fun(&mut self) -> DecodeResult {
        let _size = try!(self.rdr.read_u32::<BigEndian>());
        let arity = try!(self.rdr.read_u8());
        let uniq = try!(self._read_exact(16));
        let index = try!(self.rdr.read_u32::<BigEndian>());
        let num_free = try!(self.rdr.read_u32::<BigEndian>());

        let module = match try!(self._decode_any_atom()) {
            Eterm::Atom(atom) => atom,
            _ => unreachable!()
        };
        let old_index = match decode_some!(self, ErlTermTag::SMALL_INTEGER_EXT, ErlTermTag::INTEGER_EXT) {
            Eterm::SmallInteger(idx) => idx as u32,
            Eterm::Integer(idx) => idx as u32,
            _ => unreachable!()
        };
        let old_uniq = match decode_some!(self, ErlTermTag::SMALL_INTEGER_EXT, ErlTermTag::INTEGER_EXT) {
            Eterm::SmallInteger(uq) => uq as u32,
            Eterm::Integer(uq) => uq as u32,
            _ => unreachable!()
        };
        let pid = match decode_some!(self, ErlTermTag::PID_EXT) {
            Eterm::Pid(pid) => pid,
            _ => unreachable!()
        };
        let mut free_vars = Vec::<Eterm>::with_capacity(num_free as usize);
        for _ in 0..num_free {
            free_vars.push(try!(self.decode_term()));
        }
        Ok(Eterm::NewFun(NewFun {
            arity: arity,
            uniq: uniq,
            index: index,
            module: module,
            old_index: old_index,
            old_uniq: old_uniq,
            pid: pid,
            free_vars: free_vars,
        }))
    }
    fn decode_export(&mut self) -> DecodeResult {
        let module = match try!(self._decode_any_atom()) {
            Eterm::Atom(atom) => atom,
            _ => unreachable!()
        };
        let function = match try!(self._decode_any_atom()) {
            Eterm::Atom(atom) => atom,
            _ => unreachable!()
        };
        let arity = match decode_some!(self, ErlTermTag::SMALL_INTEGER_EXT) {
            Eterm::SmallInteger(uq) => uq,
            _ => unreachable!()
        };
        Ok(Eterm::Export(Export {
            module: module,
            function: function,
            arity: arity, // arity > u8 possible in practice
        }))
    }
    fn decode_bit_binary(&mut self) -> DecodeResult {
        let len = try!(self.rdr.read_u32::<BigEndian>());
        let bits = try!(self.rdr.read_u8());
        Ok(Eterm::BitBinary(BitBinary {
            bits: bits,
            data: try!(self._read_exact(len as u64)),
        }))
    }
    fn decode_new_float(&mut self) -> DecodeResult {
        Ok(Eterm::Float(try!(self.rdr.read_f64::<BigEndian>())))
    }


    fn _decode_tag(&mut self) -> Result<ErlTermTag, Error> {
        let int_tag = try!(self.rdr.read_u8());
        let tag: Option<ErlTermTag> = ErlTermTag::from_u8(int_tag);
        match tag {
            Some(t) => Ok(t),
            None =>
                Err(Error::UnknownTag(int_tag))
        }
    }
    pub fn decode_term(&mut self) -> DecodeResult {
        let tag = try!(self._decode_tag());
        self.decode_concrete_term(tag)
    }
    fn decode_concrete_term(&mut self, tag: ErlTermTag) -> DecodeResult {
        match tag {
            ErlTermTag::SMALL_INTEGER_EXT => self.decode_small_integer(),
            ErlTermTag::INTEGER_EXT => self.decode_integer(),
            ErlTermTag::FLOAT_EXT => self.decode_float(),
            ErlTermTag::ATOM_EXT | ErlTermTag::ATOM_UTF8_EXT => self.decode_atom(),
            ErlTermTag::REFERENCE_EXT => self.decode_reference(),
            ErlTermTag::PORT_EXT => self.decode_port(),
            ErlTermTag::PID_EXT => self.decode_pid(),
            ErlTermTag::SMALL_TUPLE_EXT => self.decode_small_tuple(),
            ErlTermTag::LARGE_TUPLE_EXT => self.decode_large_tuple(),
            ErlTermTag::MAP_EXT => self.decode_map(),
            ErlTermTag::NIL_EXT => self.decode_nil(),
            ErlTermTag::STRING_EXT => self.decode_string(),
            ErlTermTag::LIST_EXT => self.decode_list(),
            ErlTermTag::BINARY_EXT => self.decode_binary(),
            ErlTermTag::SMALL_BIG_EXT => self.decode_small_big(),
            ErlTermTag::LARGE_BIG_EXT => self.decode_large_big(),
            ErlTermTag::NEW_REFERENCE_EXT => self.decode_new_reference(),
            ErlTermTag::SMALL_ATOM_EXT | ErlTermTag::SMALL_ATOM_UTF8_EXT => self.decode_small_atom(),
            ErlTermTag::FUN_EXT => self.decode_fun(),
            ErlTermTag::NEW_FUN_EXT => self.decode_new_fun(),
            ErlTermTag::EXPORT_EXT => self.decode_export(),
            ErlTermTag::BIT_BINARY_EXT => self.decode_bit_binary(),
            ErlTermTag::NEW_FLOAT_EXT => self.decode_new_float(),
        }
    }
}

pub type EncodeResult = Result<(), Error>; // TODO: maybe return num bytes written?

pub struct Encoder<'a> {
    wrtr: &'a mut (io::Write + 'a),
    use_utf8_atoms: bool,
    use_small_atoms: bool,
    fair_new_fun: bool,
    //use_new_float: bool, (>=R11B)
}


impl<'a> Encoder<'a> {
    // TODO: asserts for overflows

    pub fn new(writer: &'a mut io::Write, utf8_atoms: bool, small_atoms: bool, fair_new_fun: bool) -> Encoder<'a> {
        Encoder{wrtr: writer,
                use_utf8_atoms: utf8_atoms,
                use_small_atoms: small_atoms,
                fair_new_fun: fair_new_fun}
    }

    pub fn write_prelude(&mut self) -> EncodeResult {
        self.wrtr.write_u8(131).map_err(From::from)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.wrtr.flush()
    }

    fn encode_small_integer(&mut self, num: u8) -> EncodeResult {
        self.wrtr.write_u8(num).map_err(From::from)
    }
    fn encode_integer(&mut self, num: i32) -> EncodeResult {
        self.wrtr.write_i32::<BigEndian>(num).map_err(From::from)
    }
    fn encode_new_float(&mut self, num: f64) -> EncodeResult {
        self.wrtr.write_f64::<BigEndian>(num).map_err(From::from)
    }

    fn _encode_str(&mut self, s: String) -> EncodeResult {
        self.wrtr.write_all(s.as_bytes()).map_err(From::from)
    }
    fn encode_atom(&mut self, atom: Atom) -> EncodeResult {
        try!(self.wrtr.write_u16::<BigEndian>(atom.len() as u16));
        self._encode_str(atom)
    }
    fn encode_small_atom(&mut self, atom: Atom) -> EncodeResult {
        try!(self.wrtr.write_u8(atom.len() as u8));
        self._encode_str(atom)
    }
    fn encode_new_reference(&mut self, reference: Reference) -> EncodeResult {
        let len = reference.id.len() / 4; // todo: ensure proper rounding, maybe (id.len() / 4) + if (id.len() % 4) == 0 {0} else {1}
        try!(self.wrtr.write_u16::<BigEndian>(len as u16));
        try!(self.encode_term(Eterm::Atom(reference.node)));
        try!(self.wrtr.write_u8(reference.creation));
        self.wrtr.write_all(reference.id.as_ref()).map_err(From::from)
    }
    fn encode_port(&mut self, port: Port) -> EncodeResult {
        try!(self.encode_term(Eterm::Atom(port.node)));
        try!(self.wrtr.write_u32::<BigEndian>(port.id));
        self.wrtr.write_u8(port.creation).map_err(From::from)
    }
    fn encode_pid(&mut self, pid: Pid) -> EncodeResult {
        try!(self.encode_term(Eterm::Atom(pid.node)));
        try!(self.wrtr.write_u32::<BigEndian>(pid.id));
        try!(self.wrtr.write_u32::<BigEndian>(pid.serial));
        self.wrtr.write_u8(pid.creation).map_err(From::from)
    }

    fn encode_small_tuple(&mut self, tuple: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_u8(tuple.len() as u8));
        for term in tuple.into_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_large_tuple(&mut self, tuple: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(tuple.len() as u32));
        for term in tuple.into_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_map(&mut self, map: Map) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(map.len() as u32));
        for (key, val) in map.into_iter() {
            try!(self.encode_term(key));
            try!(self.encode_term(val));
        }
        Ok(())
    }
    fn encode_string(&mut self, s: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u16::<BigEndian>(s.len() as u16));
        self.wrtr.write_all(s.as_ref()).map_err(From::from)
    }
    fn encode_list(&mut self, list: Vec<Eterm>) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>((list.len() - 1) as u32));
        for term in list.into_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }

    fn encode_binary(&mut self, bin: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(bin.len() as u32));
        self.wrtr.write_all(bin.as_ref()).map_err(From::from)
    }

    fn _encode_big(&mut self, sign: bigint::Sign, bytes: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u8(match sign {
            bigint::Sign::Plus => 0,
            bigint::Sign::Minus => 1,
            _ => panic!("Invalid bignum sign")
        }));
        self.wrtr.write_all(bytes.as_ref()).map_err(From::from)
    }
    fn encode_small_big(&mut self, sign: bigint::Sign, bytes: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u8(bytes.len() as u8));
        self._encode_big(sign, bytes)
    }
    fn encode_large_big(&mut self, sign: bigint::Sign, bytes: Vec<u8>) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(bytes.len() as u32));
        self._encode_big(sign, bytes)
    }

    fn encode_fun(&mut self, fun: Fun) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(fun.free_vars.len() as u32));
        try!(self.encode_term(Eterm::Pid(fun.pid)));
        try!(self.encode_term(Eterm::Atom(fun.module)));
        try!(self.encode_term(
            if fun.index <= 255 { Eterm::SmallInteger(fun.index as u8) }
            else { Eterm::Integer(fun.index as i32) }));
        try!(self.encode_term(
            if fun.uniq <= 255 { Eterm::SmallInteger(fun.uniq as u8) }
            else { Eterm::Integer(fun.uniq as i32) }));
        for term in fun.free_vars.into_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn _encode_new_fun(&mut self, fun: NewFun) -> EncodeResult {
        try!(self.wrtr.write_u8(fun.arity));
        assert!(fun.uniq.len() == 16);
        try!(self.wrtr.write_all(fun.uniq.as_ref()));
        try!(self.wrtr.write_u32::<BigEndian>(fun.index));
        try!(self.wrtr.write_u32::<BigEndian>(fun.free_vars.len() as u32));
        try!(self.encode_term(Eterm::Atom(fun.module)));

        let old_index_term = if fun.old_index <= 255 {
            Eterm::SmallInteger(fun.old_index as u8)
        } else {
            Eterm::Integer(fun.old_index as i32)
        };
        try!(self.encode_term(old_index_term));

        let old_uniq_term = if fun.old_uniq <= 255 {
            Eterm::SmallInteger(fun.old_uniq as u8)
        } else {
            Eterm::Integer(fun.old_uniq as i32)
        };
        try!(self.encode_term(old_uniq_term));

        try!(self.encode_term(Eterm::Pid(fun.pid)));

        for term in fun.free_vars.into_iter() {
            try!(self.encode_term(term));
        }
        Ok(())
    }
    fn encode_new_fun(&mut self, fun: NewFun) -> EncodeResult {
        // We serialize to temporary memory buffer to calculate encoded term size.
        // Erlang itself in 'term_to_binary' does back-patching (see
        // erts/emulator/beam/external.c#enc_term_int 'ENC_PATCH_FUN_SIZE'), but
        // at the same time, in 'binary_to_term' this size u32 is just skipped!
        // So, we make this configurable: do fair encoding or cheating with
        // fake zero size.
        if self.fair_new_fun {
            let mut temp = Vec::new();
            {
                let mut encoder = Encoder::new(&mut temp, self.use_utf8_atoms, self.use_small_atoms, self.fair_new_fun);
                try!(encoder._encode_new_fun(fun));
            }
            let size = temp.len();
            // +4 is size itself
            try!(self.wrtr.write_u32::<BigEndian>(4 + size as u32));
            self.wrtr.write_all(temp.as_ref()).map_err(From::from)
        } else {
            // cheating - write 0, since binary_to_term don't use this (at least now, in 17.0)
            try!(self.wrtr.write_u32::<BigEndian>(0));
            self._encode_new_fun(fun)
        }
    }
    fn encode_export(&mut self, export: Export) -> EncodeResult {
        try!(self.encode_term(Eterm::Atom(export.module)));
        try!(self.encode_term(Eterm::Atom(export.function)));
        self.encode_term(Eterm::SmallInteger(export.arity))
    }
    fn encode_bit_binary(&mut self, bit_bin: BitBinary) -> EncodeResult {
        try!(self.wrtr.write_u32::<BigEndian>(bit_bin.data.len() as u32));
        try!(self.wrtr.write_u8(bit_bin.bits));
        self.wrtr.write_all(bit_bin.data.as_ref()).map_err(From::from)
    }

    fn _encode_tag(&mut self, tag: ErlTermTag) -> EncodeResult {
        let int_tag = tag as u8;
        self.wrtr.write_u8(int_tag).map_err(From::from)
    }
    pub fn encode_term(&mut self, term: Eterm) -> EncodeResult {
        // XXX: maybe use &Eterm, not just Eterm?
        match term {
            Eterm::SmallInteger(num) => {
                try!(self._encode_tag(ErlTermTag::SMALL_INTEGER_EXT));
                self.encode_small_integer(num)
            },
            Eterm::Integer(num) => {
                try!(self._encode_tag(ErlTermTag::INTEGER_EXT));
                self.encode_integer(num)
            },
            Eterm::Float(num) => {
                try!(self._encode_tag(ErlTermTag::NEW_FLOAT_EXT));
                self.encode_new_float(num)
            },
            Eterm::Atom(atom) => {
                let use_utf8 = self.use_utf8_atoms;
                let use_small = self.use_small_atoms;
                if (atom.len() <= 255) && use_small {
                    try!(self._encode_tag(if use_utf8 {ErlTermTag::SMALL_ATOM_UTF8_EXT} else {ErlTermTag::SMALL_ATOM_EXT}));
                    self.encode_small_atom(atom)
                } else {
                    try!(self._encode_tag(if use_utf8 {ErlTermTag::ATOM_UTF8_EXT} else {ErlTermTag::ATOM_EXT}));
                    self.encode_atom(atom)
                }
            },
            Eterm::Reference(reference) => {
                try!(self._encode_tag(ErlTermTag::NEW_REFERENCE_EXT));
                self.encode_new_reference(reference)
            },
            Eterm::Port(port) => {
                try!(self._encode_tag(ErlTermTag::PORT_EXT));
                self.encode_port(port)
            },
            Eterm::Pid(pid) => {
                try!(self._encode_tag(ErlTermTag::PID_EXT));
                self.encode_pid(pid)
            },
            Eterm::Tuple(tuple) => {
                if tuple.len() <= 255 {
                    try!(self._encode_tag(ErlTermTag::SMALL_TUPLE_EXT));
                    self.encode_small_tuple(tuple)
                } else {
                    try!(self._encode_tag(ErlTermTag::LARGE_TUPLE_EXT));
                    self.encode_large_tuple(tuple)
                }
            },
            Eterm::Map(map) => {
                try!(self._encode_tag(ErlTermTag::MAP_EXT));
                self.encode_map(map)
            },
            Eterm::Nil =>
                self._encode_tag(ErlTermTag::NIL_EXT),
            Eterm::String(s) => {
                try!(self._encode_tag(ErlTermTag::STRING_EXT));
                self.encode_string(s)
            },
            Eterm::List(list) => {
                try!(self._encode_tag(ErlTermTag::LIST_EXT));
                self.encode_list(list)
            },
            Eterm::Binary(bin) => {
                try!(self._encode_tag(ErlTermTag::BINARY_EXT));
                self.encode_binary(bin)
            },
            Eterm::BigNum(num) => {
                let (sign, bytes) = num.to_bytes_le();
                if bytes.len() < 255 {
                    try!(self._encode_tag(ErlTermTag::SMALL_BIG_EXT));
                    self.encode_small_big(sign, bytes)
                } else {
                    try!(self._encode_tag(ErlTermTag::LARGE_BIG_EXT));
                    self.encode_large_big(sign, bytes)
                }
            },
            Eterm::Fun(fun) => {
                try!(self._encode_tag(ErlTermTag::FUN_EXT));
                self.encode_fun(fun)
            },
            Eterm::NewFun(new_fun) => {
                try!(self._encode_tag(ErlTermTag::NEW_FUN_EXT));
                self.encode_new_fun(new_fun)
            },
            Eterm::Export(export) => {
                try!(self._encode_tag(ErlTermTag::EXPORT_EXT));
                self.encode_export(export)
            },
            Eterm::BitBinary(bit_binary) => {
                try!(self._encode_tag(ErlTermTag::BIT_BINARY_EXT));
                self.encode_bit_binary(bit_binary)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Eterm,Encoder,Decoder,DecodeResult,Error};
    use std::io;
    use std::iter::FromIterator;
    use num::bigint;
    use num::traits::FromPrimitive;

    fn term_to_binary(term: Eterm) -> Result<Vec<u8>, Error> {
        let mut writer = Vec::new();
        {
            let mut encoder = Encoder::new(&mut writer, false, false, true);
            try!(encoder.write_prelude());
            try!(encoder.encode_term(term));
        }
        Ok(writer)
    }
    fn binary_to_term(binary: Vec<u8>) -> DecodeResult {
        let mut reader = io::Cursor::new(binary);
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
    );


    #[test]
    fn codec_small_integer() {
        codec_eq!(Eterm::SmallInteger(0));
        codec_eq!(Eterm::SmallInteger(255));
    }

    #[test]
    fn codec_integer() {
        codec_eq!(Eterm::Integer(-2147483647));
        codec_eq!(Eterm::Integer(-1));
        codec_eq!(Eterm::Integer(256));
        codec_eq!(Eterm::Integer(2147483647));
    }

    #[test]
    fn codec_float() {
        codec_eq!(Eterm::Float(-111111.11));
        codec_eq!(Eterm::Float(0.0));
        codec_eq!(Eterm::Float(111111.11));
    }

    #[test]
    fn codec_atom() {
        codec_eq!(Eterm::Atom(String::from("hello_world")));
    }

    #[test]
    fn codec_reference() {
        let node = String::from("my_node");
        let reference = Eterm::Reference(super::Reference {
            node: node,
            id: vec!(0, 1, 2, 3),
            creation: 0
        });
        codec_eq!(reference);
    }

    #[test]
    fn codec_port() {
        codec_eq!(Eterm::Port(super::Port {
            node: String::from("my_node"),
            id: 4294967295,
            creation: 0
        }));
    }

    #[test]
    fn codec_pid() {
        codec_eq!(Eterm::Pid(super::Pid {
            node: String::from("my_node"),
            id: 4294967295,
            serial: 1,
            creation: 0
        }));
    }

    #[test]
    fn codec_tuple() {
        codec_eq!(Eterm::Tuple(vec!(
            Eterm::SmallInteger(0),
            Eterm::Nil
                )));
    }

    #[test]
    fn codec_map() {
        // #{0 => {}, 0.0 => -1}
        let mut map: super::Map = Vec::new();
        map.push((Eterm::SmallInteger(0), Eterm::Tuple(vec!())));
        map.push((Eterm::Float(0.0), Eterm::Integer(-1)));
        let emap = Eterm::Map(map);
        codec_eq!(emap);
    }

    #[test]
    fn codec_nil() {
        codec_eq!(Eterm::Nil);
    }

    #[test]
    fn codec_string() {
        // Vec::from_fn(255, |i| i as u8);
        let vec: Vec<u8> = FromIterator::from_iter(0..(255 as u8));
        codec_eq!(Eterm::String(vec));
    }

    #[test]
    fn codec_list() {
        codec_eq!(Eterm::List(vec!(
            Eterm::Tuple(vec!()),
            Eterm::SmallInteger(1),
            Eterm::Nil,
            )));
    }

    #[test]
    fn codec_binary() {
        // Vec::from_fn(1024, |i| (i % 255) as u8)
        let mut vec: Vec<u8> = Vec::with_capacity(1024);
        for i in 0..1024 {
            vec.push((i % 255) as u8);
        }
        codec_eq!(Eterm::Binary(vec));
    }

    #[test]
    fn codec_big_num() {
        codec_eq!(Eterm::BigNum(bigint::BigInt::new(bigint::Sign::Plus, vec!(1, 1, 1, 1, 1, 1))));
        codec_eq!(Eterm::BigNum(bigint::BigInt::new(bigint::Sign::Minus, vec!(1, 1, 1, 1, 1, 1))));
        codec_eq!(Eterm::BigNum(FromPrimitive::from_i64(i64::max_value()).unwrap()));
        let vec: Vec<u32> = FromIterator::from_iter(0..(256 as u32));
        codec_eq!(Eterm::BigNum(bigint::BigInt::new(bigint::Sign::Plus, vec)));
    }

    #[test]
    fn codec_fun() {
        let pid = super::Pid {
            node: String::from("my_node"),
            id: 4294967295,
            serial: 1,
            creation: 0
        };
        codec_eq!(Eterm::Fun(super::Fun {
            pid: pid,
            module: String::from("my_mod"),
            index: 1,
            uniq: u32::max_value(),
            free_vars: vec!(Eterm::Nil)
        }));
    }

    #[test]
    fn codec_new_fun() {
        let pid = super::Pid {
            node: String::from("my_node"),
            id: u32::max_value(),
            serial: 1,
            creation: 0
        };
        let vec: Vec<u8> = FromIterator::from_iter(0..(16 as u8));
        codec_eq!(Eterm::NewFun(super::NewFun {
            arity: 128,         // :-)
            uniq: vec, //Vec::from_fn(16, |i| i as u8),
            index: u32::max_value(),
            module: String::from("my_mod"),
            old_index: u32::max_value(),
            old_uniq: u32::max_value(),
            pid: pid,
            free_vars: vec!(Eterm::Nil)
        }));
    }

    #[test]
    fn codec_export() {
        codec_eq!(Eterm::Export(super::Export {
            module: String::from("my_mod"),
            function: String::from("my_fun"),
            arity: u8::max_value()
        }));
    }

    #[test]
    fn codec_bit_binary() {
        codec_eq!(Eterm::BitBinary(super::BitBinary {
            bits: 1,
            data: vec!(255, 255)
        }));
    }
}
