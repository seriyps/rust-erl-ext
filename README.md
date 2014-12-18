Rust Erl Ext
============

[Erlang external term format](http://erlang.org/doc/apps/erts/erl_ext_dist.html)
parser/serializer for Rust.

[![Build Status](https://travis-ci.org/seriyps/rust-erl-ext.png?branch=master)](https://travis-ci.org/seriyps/rust-erl-ext)

Examples
-------

Decoding

```rust
extern crate erl_ext;
use erl_ext::Decoder;

fn main() {
    let mut decoder = Decoder::new(&mut io::stdin());
    assert!(true == decoder.read_prelude().unwrap());
    println!("{}", decoder.decode_term().unwrap());
}
```

Encoding

```rust
extern crate erl_ext;
use erl_ext::{Eterm, Encoder};

fn main() {
    let term = Eterm::List(vec!(Eterm::SmallInteger(1),
                                Eterm::Integer(1000000),
                                Eterm::Nil));
    // this combination of options make it compatible with erlang:term_to_binary/1
    let utf8_atoms = false;
    let small_atoms = false;
    let fair_new_fun = true;
    let mut encoder = Encoder::new(&mut io::stdout(),
                                   utf8_atoms, small_atoms, fair_new_fun);
    encoder.write_prelude();
    encoder.encode_term(term);
}
```

More examples are in `examples` directory.

Types (all Erlang 17.1 types are supported):

* SmallInteger (u8)     : `0..255`
* Integer (i32)         : `integer()`
* Float (f64)           : `float()`
* Atom (String)         : `atom()`
* Reference             : `reference()` `erlang:make_ref/0`
* Port                  : `port()` eg, socket or raw file or `erlang:open_port/2`
* Pid                   : `pid()`
* Tuple (`Vec<Eterm>`)  : `{ any() }`
* Map (`Vec<(Eterm, Eterm)>`) : `#{any() := any()}`
* Nil                   : `[]`
* String (`Vec<u8>`)    : `[0..255]`
* List (`Vec<Eterm>`)   : `[ any() ]`
* Binary (`Vec<u8>`)    : `binary()`
* BigNum (`BigInt`)     : `integer() > i32`
* Fun                   : `fun(..) -> ... end.` - deprecated variant
* NewFun                : `fun(..) -> ... end.`
* Export                : `fun my_mod:my_fun/1`
* BitBinary             : `<<128, 128:4>>`


TODO
----

* `serialize::Decoder` and `serialize::Encoder` implementations (not so easy for containers)
* Quick-Check - like tests (feed pseudo-random bytes to decoder, feed random Eterm's to encoder)

Keywords
--------

* Rust
* Erlang
* BERT
* External term format
* term_to_binary, binary_to_term
* parser, serializer
