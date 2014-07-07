// See erts-6.1/doc/html/erl_ext_dist.html

#![feature(struct_variant)]     // this is for enum Eterm
#![allow(non_camel_case_types)] // this is for enum ErlTermTag
#![feature(macro_rules)]        // for try_io!

extern crate num;
extern crate collections;
extern crate debug;

use std::string::String;
use std::vec::Vec;
use std::num::FromPrimitive;
use std::io;
use num::bigint::BigInt;
use collections::bitv::Bitv;


#[deriving(FromPrimitive, Show)]
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

//#[deriving(PartialOrd, PartialEq, Ord, Eq)]
#[deriving(Show)]
pub enum Eterm {
    SmallInteger(u8),           // small_integer
    Integer(int),               // integer
    Float(f64),                 // float, new_float
    Atom(Atom),                 // atom, small_atom, atom_utf8, small_atom_utf8
    Reference {                 // reference, new_reference
        node: Atom,
        id: Vec<u8>,
        creation: u8},
    Port {                      // poort
        node: Atom,
        id: u8,
        creation: u8},
    Pid(Pid),                   // pid
    Tuple(Tuple),               // small_tuple, large_tuple
    Map(EMap),                  // map
    Nil,                        // nil
    String(String),             // string
    List(List),                 // list
    Binary(Vec<u8>),            // binary
    BigNum(BigInt),             // small_big, large_big
    Fun {                       // fun
        pid: Pid,
        module: Atom,
        index: u32,
        uniq: u32,
        free_vars: Vec<Eterm>},
    NewFun {                    // new_fun
        arity: u8,
        uniq: u16,
        index: u32,
        module: Atom,
        old_index: u32,
        old_uniq: u16,
        pid: Pid,
        free_vars: Vec<Eterm>},
    Export {                    // export
        module: Atom,
        function: Atom,
        arity: u8,
    },
    BitBinary(Bitv),            // bit_binary; XXX: maybe choose some other representation?
}
pub type Atom = String;
pub type Tuple = Vec<Eterm>;
pub type EMap = Vec<(Eterm, Eterm)>; // k-v pairs //TreeMap<Eterm, Eterm>;
pub type List = Vec<Eterm>;

//#[deriving(PartialOrd, PartialEq, Ord, Eq)]
#[deriving(Show)]
pub struct Pid {                // moved out from enum because it used in Eterm::{Fun,NewFun}
    node: Atom,
    id: u8,
    serial: u32,
    creation: u8,
}


// enum BuildError {
//     IoError(io::IoError),
//     BErr,
// }
type BuildResult = io::IoResult<Eterm>;//, BuildError>;

struct Builder<T> {
    rdr: T,
}

// macro_rules! try_io(
//     ($e:expr) => (
//         match $e {
//             Ok(e) => e,
//             Err(e) => return IoError(e)
//         }
//         )
// )

impl<T: io::Reader> Builder<T> {
    fn new(rdr: T) -> Builder<T> {
        Builder{rdr: rdr}
    }
    fn read_prelude(&mut self) -> io::IoResult<bool> {
        Ok(131 == try!(self.rdr.read_u8()))
    }
    fn build(&mut self) -> BuildResult {
        let int_tag = try!(self.rdr.read_u8());
        let tag: Option<ErlTermTag> = FromPrimitive::from_u8(int_tag);
        match tag {
            Some(SMALL_INTEGER_EXT) =>
                Ok(SmallInteger(try!(self.rdr.read_u8()))),
            Some(INTEGER_EXT) =>
                Ok(Integer(try!(self.rdr.read_be_int()))),
            Some(ATOM_EXT) | Some(ATOM_UTF8_EXT) => {
                let len = try!(self.rdr.read_be_u16());
                let data = try!(self.rdr.read_exact(len as uint));
                // XXX: data is in latin1 in case of ATOM_EXT
                match String::from_utf8(data) {
                    Ok(atom) =>
                        Ok(Atom(atom)),
                    Err(_) =>
                        Err(io::IoError{
                            kind: io::OtherIoError,
                            desc: "Bad utf-8",
                            detail: None // format!("{}", data)
                            })
                }
            },
            Some(REFERENCE_EXT) => {
                let node = match try!(self.build()) {
                    Atom(a) =>
                        a,
                    _ =>
                        return Err(io::IoError{
                            kind: io::OtherIoError,
                            desc: "Assertion failed: Atom expected",
                            detail: None
                        })
                };
                let id = try!(self.rdr.read_exact(4));
                let creation = try!(self.rdr.read_u8());
                Ok(Reference {
                    node: node,
                    id: id,
                    creation: creation
                })
            },
            Some(SMALL_TUPLE_EXT) => {
                let mut tuple: Tuple = Vec::new();
                let arity = try!(self.rdr.read_u8());
                for _ in range(0, arity) {
                    let term = try!(self.build());
                    tuple.push(term)
                }
                Ok(Tuple(tuple))
            },
            Some(MAP_EXT) => {
                let mut map: EMap = Vec::new();
                let arity = try!(self.rdr.read_be_u32());
                for _ in range(0, arity) {
                    let key = try!(self.build());
                    let val = try!(self.build());
                    map.push((key, val))
                }
                Ok(Map(map))
            },
            Some(NIL_EXT) =>
                Ok(Nil),
            Some(BINARY_EXT) => {
                let len = try!(self.rdr.read_be_u32());
                Ok(Binary(try!(self.rdr.read_exact(len as uint))))
            },
            Some(NEW_REFERENCE_EXT) => {
                let len = try!(self.rdr.read_be_u16());
                let node = match try!(self.build()) {
                    Atom(a) =>
                        a,
                    _ =>
                        return Err(io::IoError{
                            kind: io::OtherIoError,
                            desc: "Assertion failed: Atom expected",
                            detail: None
                        })
                };
                let creation = try!(self.rdr.read_u8());
                let id = try!(self.rdr.read_exact(4 * len as uint));
                Ok(Reference{
                    node: node,
                    id: id, // here id should be Vec<u32>, but since it's not interpreted, leave it as is
                    creation: creation
                })
            },
            Some(SMALL_ATOM_EXT) | Some(SMALL_ATOM_UTF8_EXT) => {
                let len = try!(self.rdr.read_u8());
                let data = try!(self.rdr.read_exact(len as uint));
                // XXX: data is in latin1 in case of ATOM_EXT
                match String::from_utf8(data) {
                    Ok(atom) =>
                        Ok(Atom(atom)),
                    Err(_) =>
                        Err(io::IoError{
                            kind: io::OtherIoError,
                            desc: "Bad utf-8",
                            detail: None // format!("{}", data)
                            })
                }
            },
            Some(t) =>
                Err(io::IoError{
                    kind: io::OtherIoError,
                    desc: "Tag not implemented",
                    detail: Some(format!("Tag: #{} - {}", int_tag, t))
                }),
            None =>
                Err(io::IoError{
                    kind: io::OtherIoError,
                    desc: "Invalid data type",
                    detail: Some(format!("Tag: #{}", int_tag))
                })
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
        uniq: 1234,
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
    println!("{:?}", term);
    println!("==============================");
    match term {
        NewFun {free_vars: vars, ..} =>
            println!("{:?}", vars.as_slice()),
        _ => ()
    }
    println!("==============================");
    let f = BufferedReader::new(File::open(&Path::new("test/test_terms.bin")));
    let mut builder = Builder::new(f);
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
    println!("{}", builder.build());
}
