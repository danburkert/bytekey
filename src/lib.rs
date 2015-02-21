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

#![feature(core, plugin, io, unicode)]
#![cfg_attr(test, feature(std_misc))]
#![cfg_attr(test, plugin(quickcheck_macros))]

extern crate byteorder;
extern crate "rustc-serialize" as rustc_serialize;

#[cfg(test)] extern crate quickcheck;
#[cfg(test)] extern crate rand;

pub use encoder::Encoder;
pub use decoder::Decoder;

mod encoder;
mod decoder;

use rustc_serialize::{Encodable, Decodable};
use std::{error, fmt, io, result};

/// Encode data isizeo a byte vector.
///
/// #### Usage
///
/// ```
/// # use bytekey::encode;
/// assert_eq!(Ok(vec!(0x00, 0x00, 0x00, 0x2A)), encode(&42u32));
/// assert_eq!(Ok(vec!(0x66, 0x69, 0x7A, 0x7A, 0x62, 0x75, 0x7A, 0x7A, 0x00)), encode(&"fizzbuzz"));
/// assert_eq!(Ok(vec!(0x2A, 0x66, 0x69, 0x7A, 0x7A, 0x00)), encode(&(42u8, "fizz")));
/// ```
pub fn encode<T : Encodable>(object: &T) -> Result<Vec<u8>>  {
    let mut writer = Vec::new();
    {
        let mut encoder = Encoder::new(&mut writer);
        try!(object.encode(&mut encoder));
    }
    Ok(writer)
}

/// Decode data from a byte vector.
///
/// #### Usage
///
/// ```
/// # use bytekey::{encode, decode};
/// assert_eq!(Ok(42u), decode::<usize>(encode(&42u).unwrap()));
/// ```
pub fn decode<T: Decodable>(bytes: Vec<u8>) -> Result<T> {
    Decodable::decode(&mut Decoder::new(io::Cursor::new(bytes)))
}

/// A short-hand for `result::Result<T, bytekey::decoder::Error>`.
pub type Result<T> = result::Result<T, Error>;

/// An error type for bytekey decoding and encoding.
///
/// This is a thin wrapper over the standard `io::Error` type. Namely, it
/// adds two additional error cases: an unexpected EOF, and invalid utf8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {

    /// Variant representing that the underlying stream was read successfully but it did not contain
    /// valid utf8 data.
    NotUtf8,

    /// Variant representing that the underlying stream returns less bytes, than are required to
    /// decode a meaningful value.
    UnexpectedEof,

    /// Variant representing that an I/O error occurred.
    Io(io::Error),
}

impl error::FromError<io::Error> for Error {
    fn from_error(error: io::Error) -> Error { Error::Io(error) }
}

impl error::FromError<io::CharsError> for Error {
    fn from_error(error: io::CharsError) -> Error {
        match error {
            io::CharsError::NotUtf8 => Error::NotUtf8,
            io::CharsError::Other(error) => Error::Io(error),
        }
    }
}

impl error::FromError<byteorder::Error> for Error {
    fn from_error(error: byteorder::Error) -> Error {
        match error {
            byteorder::Error::UnexpectedEOF => Error::UnexpectedEof,
            byteorder::Error::Io(error) => Error::Io(error),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NotUtf8 => write!(f, "byte stream did not contain valid utf8"),
            Error::UnexpectedEof => write!(f, "unexpected end of file"),
            Error::Io(ref err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::NotUtf8 => "invalid utf8 encoding",
            Error::UnexpectedEof => "unexpected end of file",
            Error::Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::NotUtf8 => None,
            Error::UnexpectedEof => None,
            Error::Io(ref err) => err.cause(),
        }
    }
}
