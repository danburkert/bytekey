#![feature(phase)]

extern crate serialize;

#[cfg(test)]
extern crate quickcheck;

pub use encoder::Encoder;
pub use encoder::EncodeResult;
pub use encoder::encode;

pub use decoder::Decoder;
pub use decoder::DecodeResult;
pub use decoder::decode;

mod encoder;
mod decoder;
