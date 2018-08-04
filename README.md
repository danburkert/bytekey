[![Build Status](https://travis-ci.org/danburkert/bytekey.svg?branch=master)](https://travis-ci.org/danburkert/bytekey)

[rustdoc](https://danburkert.github.io/bytekey/bytekey/index.html)

# bytekey

Binary encoding for Rust values which preserves lexicographic sort order.
Order-preserving encoding is useful for creating keys for sorted key-value
stores with byte string typed keys, such as
[leveldb](https://github.com/google/leveldb) and
[sled](https://github.com/spacejam/sled). `bytekey` attempts to encode values
into the fewest number of bytes possible while preserving order guarantees. Type
information is *not* serialized alongside values, and thus the type of
serialized data must be known in order to perform decoding (`bytekey` does not
implement a self-describing format).

## Supported Data Types

`bytekey` currently supports all Rust primitives, strings, options, structs,
enums, vecs, and tuples. See **Serializer** for details on the serialization
format.

## Usage

```rust
#[macro_use]
extern crate serde_derive;
extern crate bytekey;

use bytekey::{deserialize, serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct MyKey { a: u32, b: String }

fn main() {
    let a = MyKey { a: 1, b: "foo".to_string() };
    let b = MyKey { a: 2, b: "foo".to_string() };
    let c = MyKey { a: 2, b: "fooz".to_string() };
    assert!(serialize(&a).unwrap() < serialize(&b).unwrap());
    assert!(serialize(&b).unwrap() < serialize(&c).unwrap());
    assert_eq!(a, deserialize(&serialize(&a).unwrap()).unwrap());
}
```

## Type Evolution

In general, the exact type of a serialized value must be known in order to
correctly deserialize it. For structs and enums, the type is effectively frozen
once any values of the type have been serialized: changes to the struct or enum
will cause deserialization of already serialized values to fail or return
incorrect values. The only exception is adding new variants to the end of an
existing enum. Enum variants may *not* change type, be removed, or be reordered.
All changes to structs, including adding, removing, reordering, or changing the
type of a field are forbidden.

These restrictions lead to a few best-practices when using `bytekey`
serialization:

* Don't use `bytekey` unless you need lexicographic ordering of serialized
  values! A more general encoding library such as [Cap'n
  Proto](https://github.com/dwrensha/capnproto-rust) or
  [bincode](https://github.com/TyOverby/binary-encode) will serve you better if
  this feature is
  not necessary.
* If you persist serialized values for longer than the life of a process (i.e.
  you write the serialized values to a file or a database), consider using an
  enum as a top-level wrapper type. This will allow you to seamlessly add a new
  variant when you need to change the key format in a backwards-compatible
  manner (the different key types will sort seperately). If your enum has less
  than 16 variants, then the overhead is just a single byte in serialized
  output.

## License

`bytekey` is licensed under the Apache License, Version 2.0. See LICENSE for
full license text.
