//! [![Build Status](https://travis-ci.org/danburkert/bytekey.svg?branch=master)](https://travis-ci.org/danburkert/bytekey)
//!
//! [GitHub](https://github.com/danburkert/bytekey)
//!
//! Binary encoding for Rust values which preserves lexicographic sort order. Order-preserving
//! encoding is useful for creating keys for sorted key-value stores with byte string typed keys,
//! such as [leveldb](https://github.com/google/leveldb). `bytekey` attempts to encode values into
//! the fewest number of bytes possible while preserving ordering. Type information is *not*
//! serialized alongside values, and thus the type of serialized data must be known in order to
//! perform decoding (`bytekey` does not implement a self-describing format).
//!
//! #### Supported Data Types
//!
//! `bytekey` encoding currently supports all Rust primitives, strings, options, structs, enums, and
//! tuples. `isize` and `usize` types are variable-length encoded. Sequence (`Vec`) and map types are
//! not currently supported (but could be in the future). See `Encoder` for details on the
//! serialization format.
//!
//! #### Usage
//!
//! ```
//! extern crate "rustc-serialize" as rustc_serialize;
//! extern crate bytekey;
//! use bytekey::{encode, decode};
//!
//! #[derive(RustcEncodable, RustcDecodable, Show, PartialEq)]
//! struct MyKey { a: u32, b: String }
//!
//! # fn main() {
//! let a = MyKey { a: 1, b: "foo".to_string() };
//! let b = MyKey { a: 2, b: "foo".to_string() };
//! let c = MyKey { a: 2, b: "fooz".to_string() };
//!
//! assert!(encode(&a).unwrap() < encode(&b).unwrap());
//! assert!(encode(&b).unwrap() < encode(&c).unwrap());
//! assert_eq!(a, decode(encode(&a).unwrap()).unwrap());
//! # }
//! ```
//!
//! #### Type Evolution
//!
//! In general, the exact type of a serialized value must be known in order to correctly deserialize
//! it. For structs and enums, the type is effectively frozen once any values of the type have been
//! serialized: changes to the struct or enum will cause deserialization of already encoded values
//! to fail or return incorrect values. The only exception is adding adding new variants to the end
//! of an existing enum. Enum variants may *not* change type, be removed, or be reordered. All
//! changes to structs, including adding, removing, reordering, or changing the type of a field are
//! forbidden.
//!
//! These restrictions lead to a few best-practices when using `bytekey` encoding:
//!
//! * Don't use `bytekey` unless you need lexicographic ordering of encoded values! A more
//! general encoding library such as [Cap'n Proto](https://github.com/dwrensha/capnproto-rust) or
//! [binary-encode](https://github.com/TyOverby/binary-encode) will serve you better if this
//! feature is not necessary.
//! * If you persist encoded values for longer than the life of a process (i.e. you write the
//! encoded values to a file or a database), consider using an enum as a top-level wrapper type.
//! This will allow you to seamlessly add a new variant when you need to change the key format in a
//! backwards-compatible manner (the different key types will sort seperately). If your enum has
//! less than 16 variants, then the overhead is just a single byte in encoded output.

#![feature(plugin, old_io, core)]
#![cfg_attr(test, feature(std_misc))]
#![cfg_attr(test, plugin(quickcheck_macros))]

extern crate "rustc-serialize" as rustc_serialize;

#[cfg(test)] extern crate quickcheck;
#[cfg(test)] extern crate rand;

pub use encoder::Encoder;
pub use encoder::EncodeResult;
pub use encoder::encode;

pub use decoder::Decoder;
pub use decoder::DecodeResult;
pub use decoder::decode;

mod encoder;
mod decoder;
