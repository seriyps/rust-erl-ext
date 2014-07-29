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

fn main() {
    let mut decoder = erl_ext::Decoder(&mut io::stdin());
    assert!(true == decoder.read_prelude().unwrap());
    println!("{}", decoder.decode_term().unwrap());
}
```

Encoding

```rust
extern crate erl_ext;

fn main() {
    let term = erl_ext::List(vec!(erl_ext::SmallInteger(1),
                                  erl_ext::Integer(1000000),
                                  erl_ext::Nil));
    // this combination of options make it compatible with erlang:term_to_binary/1
    let utf8_atoms = false;
    let small_atoms = false;
    let fair_new_fun = true;
    let mut encoder = erl_ext::Encoder(&mut io::stdout(),
                                       utf8_atoms, small_atoms, fair_new_fun);
    encoder.write_prelude();
    encoder.encode_term(term);
}
```

More examples are in `examples` directory.

TODO
----

* `serialize::Decoder` and `serialize::Encoder` implementations (not so easy for containers)
* Unit and functional tests
* Quick-Check - like tests (feed pseudo-random bytes to decoder, feed random Eterm's to encoder)

Keywords
--------

* Rust
* Erlang
* BERT
* External term format
* term_to_binary, binary_to_term
* parser, serializer
