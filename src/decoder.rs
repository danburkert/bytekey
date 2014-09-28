use serialize;
use serialize::Decodable;
use std::{i8, i16, i32, i64};
use std::io::MemReader;
use std::io;
use std::iter::range_inclusive;
use std::io::IoError;
use std::mem::transmute;

/// A decoder for deserializing bytes in an order preserving format to a value.
pub struct Decoder<R> {
    reader: io::BufferedReader<R>
}

/// Decode data from a byte vector.
///
/// #### Usage
///
/// ```
/// # use bytekey::{encode, decode};
/// assert_eq!(Ok(42u), decode::<uint>(encode(&42u)));
/// ```
pub fn decode<T: Decodable<Decoder<MemReader>, io::IoError>>(bytes: Vec<u8>) -> Result<T, io::IoError> {
    Decodable::decode(&mut Decoder::new(MemReader::new(bytes)))
}

/// The error type returned by all decoding operations supported by `Decoder`.
pub type DecodeResult<T> = Result<T, IoError>;

impl<R: io::Reader> Decoder<R> {

    /// Creates a new ordered bytes encoder whose output will be written to the provided writer.
    pub fn new(reader: R) -> Decoder<R> {
        Decoder { reader: io::BufferedReader::new(reader) }
    }

    pub fn read_var_u64(&mut self) -> DecodeResult<u64> {
        let header = try!(self.reader.read_u8());
        let n = header >> 4;
        let mut val = ((header & 0x0F) as u64) << (n as uint * 8);
        for i in range_inclusive(1, n) {
            let byte = try!(self.reader.read_u8());
            val += (byte as u64) << ((n - i) as uint * 8);
        }
        Ok(val)
    }

    pub fn read_var_i64(&mut self) -> DecodeResult<i64> {
        let header = try!(self.reader.read_u8());
        let mask = ((header ^ 0x80) as i8 >> 7) as u8;
        let n = ((header >> 3) ^ mask) & 0x0F;
        let mut val = (((header ^ mask) & 0x07) as u64) << (n as uint * 8);
        for i in range_inclusive(1, n) {
            let byte = try!(self.reader.read_u8());
            val += ((byte ^ mask) as u64) << ((n - i) as uint * 8);
        }
        let final_mask = (((mask as i64) << 63) >> 63) as u64;
        val ^= final_mask;
        Ok(val as i64)
    }
}


impl<R: Reader> serialize::Decoder<IoError> for Decoder<R> {

    fn read_nil(&mut self) -> DecodeResult<()> { Ok(()) }

    fn read_u8(&mut self) -> DecodeResult<u8> { self.reader.read_u8() }
    fn read_u16(&mut self) -> DecodeResult<u16> { self.reader.read_be_u16() }
    fn read_u32(&mut self) -> DecodeResult<u32> { self.reader.read_be_u32() }
    fn read_u64(&mut self) -> DecodeResult<u64> { self.reader.read_be_u64() }
    fn read_uint(&mut self) -> DecodeResult<uint> {
        let val = try!(self.read_var_u64());
        Ok(val as uint)
    }

    fn read_i8(&mut self) -> DecodeResult<i8> {
        let val = try!(self.reader.read_i8());
        Ok(val ^ i8::MIN)
    }
    fn read_i16(&mut self) -> DecodeResult<i16> {
        let val = try!(self.reader.read_be_i16());
        Ok(val ^ i16::MIN)
    }
    fn read_i32(&mut self) -> DecodeResult<i32> {
        let val = try!(self.reader.read_be_i32());
        Ok(val ^ i32::MIN)
    }
    fn read_i64(&mut self) -> DecodeResult<i64> {
        let val = try!(self.reader.read_be_i64());
        Ok(val ^ i64::MIN)
    }
    fn read_int(&mut self) -> DecodeResult<int> {
        let val = try!(self.read_var_i64());
        Ok(val as int)
    }

    fn read_bool(&mut self) -> DecodeResult<bool> {
        match try!(self.reader.read_u8()) {
            0 => Ok(false),
            _ => Ok(true)
        }
    }

    fn read_f32(&mut self) -> DecodeResult<f32> {
        let val = try!(self.reader.read_be_i32());
        let t = ((val ^ i32::MIN) >> 31) | i32::MIN;
        let f = unsafe { transmute(val ^ t) };
        Ok(f)
    }

    fn read_f64(&mut self) -> DecodeResult<f64> {
        let val = try!(self.reader.read_be_i64());
        let t = ((val ^ i64::MIN) >> 63) | i64::MIN;
        let f = unsafe { transmute(val ^ t) };
        Ok(f)
    }

    fn read_char(&mut self) -> DecodeResult<char> {
        let val = try!(self.reader.read_char());
        Ok(val)
    }

    fn read_str(&mut self) -> DecodeResult<String> {
        let mut string = String::new();

        loop {
            let c = try!(self.reader.read_char());
            if c == '\0' { break; }
            string.push(c);
        }

        Ok(string)
    }

    fn read_enum<T>(&mut self, _name: &str, f: |&mut Decoder<R>| -> DecodeResult<T>) -> DecodeResult<T> {
        f(self)
    }
    fn read_enum_variant<T>(&mut self,
                            _names: &[&str],
                            f: |&mut Decoder<R>, uint| -> DecodeResult<T>)
                            -> DecodeResult<T> {
        let id = try!(self.read_uint());
        f(self, id)
    }
    fn read_enum_variant_arg<T>(&mut self,
                                _idx: uint,
                                f: |&mut Decoder<R>| -> DecodeResult<T>)
                                -> DecodeResult<T> {
        f(self)
    }
    fn read_enum_struct_variant<T>(&mut self,
                                   names: &[&str],
                                   f: |&mut Decoder<R>, uint| -> DecodeResult<T>)
                                   -> DecodeResult<T> {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T>(&mut self,
                                         _name: &str,
                                         idx: uint,
                                         f: |&mut Decoder<R>| -> DecodeResult<T>)
                                         -> DecodeResult<T> {
        self.read_enum_variant_arg(idx, f)
    }

    fn read_struct<T>(&mut self,
                      _name: &str,
                      _len: uint,
                      f: |&mut Decoder<R>| -> DecodeResult<T>)
                      -> DecodeResult<T> {
        f(self)
    }
    fn read_struct_field<T>(&mut self,
                            _name: &str,
                            _idx: uint,
                            f: |&mut Decoder<R>| -> DecodeResult<T>) -> DecodeResult<T> {
        f(self)
    }

    fn read_tuple<T>(&mut self, _f: |&mut Decoder<R>, uint| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not implemented. Waiting on rust#17595")
    }
    fn read_tuple_arg<T>(&mut self,
                         _idx: uint,
                         f: |&mut Decoder<R>| -> DecodeResult<T>)
                         -> DecodeResult<T> {
        f(self)
    }
    fn read_tuple_struct<T>(&mut self,
                            _name: &str,
                            f: |&mut Decoder<R>, uint| -> DecodeResult<T>)
                            -> DecodeResult<T> {
        self.read_tuple(f)
    }
    fn read_tuple_struct_arg<T>(&mut self,
                                idx: uint,
                                f: |&mut Decoder<R>| -> DecodeResult<T>)
                                -> DecodeResult<T> {
        self.read_tuple_arg(idx, f)
    }

    fn read_option<T>(&mut self, f: |&mut Decoder<R>, bool| -> DecodeResult<T>) -> DecodeResult<T> {
        let is_some = try!(self.read_bool());
        f(self, is_some)
    }

    fn read_seq<T>(&mut self, _f: |&mut Decoder<R>, uint| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not yet implemented");
    }
    fn read_seq_elt<T>(&mut self, _idx: uint, _f: |&mut Decoder<R>| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not yet implemented");
    }

    fn read_map<T>(&mut self, _f: |&mut Decoder<R>, uint| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not yet implemented");
    }
    fn read_map_elt_key<T>(&mut self, _idx: uint, _f: |&mut Decoder<R>| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not yet implemented");
    }
    fn read_map_elt_val<T>(&mut self, _idx: uint, _f: |&mut Decoder<R>| -> DecodeResult<T>) -> DecodeResult<T> {
        fail!("not yet implemented");
    }

    fn error(&mut self, err: &str) -> IoError {
        IoError {
            kind: io::OtherIoError,
            desc: "Decoding error",
            detail: Some(err.to_string())
        }
    }
}

#[cfg(test)]
mod test {

    #[phase(plugin)]
    extern crate quickcheck_macros;
    extern crate quickcheck;

    use encoder::encode;
    use encoder::test::{TestStruct, TestEnum};
    use decoder::decode;
    use std::{f32, f64};
    use std::num::pow;

    #[quickcheck]
    fn check_u8(val: u8) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_u16(val: u16) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_u32(val: u32) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_u64(val: u64) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_uint(val: uint) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[test]
    fn test_uint() {
        let values = vec![
            0u, pow(2u, 0),
            pow(2u, 4)  - 1, pow(2u, 4),
            pow(2u, 12) - 1, pow(2u, 12),
            pow(2u, 20) - 1, pow(2u, 20),
            pow(2u, 28) - 1, pow(2u, 28),
            pow(2u, 36) - 1, pow(2u, 36),
            pow(2u, 44) - 1, pow(2u, 44),
            pow(2u, 52) - 1, pow(2u, 52),
            pow(2u, 60) - 1, pow(2u, 60),
            pow(2u, 64) - 1,
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val)).unwrap());
        }
    }

    #[quickcheck]
    fn check_i8(val: i8) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_i16(val: i16) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_i32(val: i32) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_i64(val: i64) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[quickcheck]
    fn check_int(val: int) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[test]
    fn test_int() {
        let values = vec![
            -pow(2i, 0), 0i, pow(2i, 0),
            -pow(2i, 3)  - 1, -pow(2i, 3),  pow(2i, 3)  - 1, pow(2i, 3),
            -pow(2i, 11) - 1, -pow(2i, 11), pow(2i, 11) - 1, pow(2i, 11),
            -pow(2i, 19) - 1, -pow(2i, 19), pow(2i, 19) - 1, pow(2i, 19),
            -pow(2i, 27) - 1, -pow(2i, 27), pow(2i, 27) - 1, pow(2i, 27),
            -pow(2i, 35) - 1, -pow(2i, 35), pow(2i, 35) - 1, pow(2i, 35),
            -pow(2i, 43) - 1, -pow(2i, 43), pow(2i, 43) - 1, pow(2i, 43),
            -pow(2i, 51) - 1, -pow(2i, 51), pow(2i, 51) - 1, pow(2i, 51),
            -pow(2i, 59) - 1, -pow(2i, 59), pow(2i, 59) - 1, pow(2i, 59),
            -pow(2i, 63), pow(2i, 63) - 1
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val)).unwrap());
        }
    }

    #[quickcheck]
    fn check_f32(val: f32) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[test]
    fn test_f32() {
        let values = vec![
            f32::NEG_INFINITY,
            f32::MIN_VALUE,
            -0.0,
            0.0,
            f32::MIN_POS_VALUE,
            f32::MAX_VALUE,
            f32::INFINITY
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val)).unwrap());
        }
    }

    #[quickcheck]
    fn check_f64(val: f64) -> bool {
        val == decode(encode(&val)).unwrap()
    }
    #[test]
    fn test_f64() {
        let values = vec![
            f64::NEG_INFINITY,
            f64::MIN_VALUE,
            -0.0,
            0.0,
            f64::MIN_POS_VALUE,
            f64::MAX_VALUE,
            f64::INFINITY
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val)).unwrap());
        }
    }

    #[quickcheck]
    fn check_char(val: char) -> bool {
        val == decode(encode(&val)).unwrap()
    }

    #[quickcheck]
    fn check_string(val: String) -> bool {
        val == decode(encode(&val)).unwrap()
    }

    #[quickcheck]
    fn check_option(val: Option<String>) -> bool {
        val == decode(encode(&val)).unwrap()
    }

    #[quickcheck]
    fn check_struct(val: TestStruct) -> bool {
        val == decode(encode(&val)).unwrap()
    }

    // #[quickcheck]
    fn check_tuple(val: (uint, char, String)) -> bool {
        val == decode(encode(&val)).unwrap()
    }

    #[quickcheck]
    fn check_enum(val: TestEnum) -> bool {
        val == decode(encode(&val)).unwrap()
    }
}
