// See erts-6.1/doc/html/erl_ext_dist.html

#![feature(struct_variant)]     // this is for enum Eterm
#![allow(non_camel_case_types)] // this is for enum ErlTermTag

extern crate num;
extern crate collections;
extern crate debug;

use std::string::String;
use std::vec::Vec;
use std::num::FromPrimitive;
use num::bigint::BigInt;
use collections::bitv::Bitv;
use std::collections::TreeMap;


#[deriving(FromPrimitive)]
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
pub type EMap = TreeMap<Eterm, Eterm>;
pub type List = Vec<Eterm>;
pub struct Pid {
    node: Atom,
    id: u8,
    serial: u32,
    creation: u8,
}

fn main() {
    for i in range(70, 120) {
        let tag: Option<ErlTermTag> = FromPrimitive::from_int(i);
        println!("{} => {:?}", i, tag);
    }
}
