use std::{error, i8, i16, i32, i64};
use std::io::{self, Read};
use std::iter::range_inclusive;
use std::mem::transmute;

use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use rustc_serialize;

use Error;
use Result;

/// A decoder for deserializing bytes in an order preserving format to a value.
pub struct Decoder<R> {
    reader: io::BufReader<R>
}

impl<R: io::Read> Decoder<R> {

    /// Creates a new ordered bytes encoder whose output will be written to the provided writer.
    pub fn new(reader: R) -> Decoder<R> {
        Decoder { reader: io::BufReader::new(reader) }
    }

    pub fn read_var_u64(&mut self) -> Result<u64> {
        let header = try!(self.reader.read_u8());
        let n = header >> 4;
        let mut val = ((header & 0x0F) as u64) << (n as usize * 8);
        for i in range_inclusive(1, n) {
            let byte = try!(self.reader.read_u8());
            val += (byte as u64) << ((n - i) as usize * 8);
        }
        Ok(val)
    }

    pub fn read_var_i64(&mut self) -> Result<i64> {
        let header = try!(self.reader.read_u8());
        let mask = ((header ^ 0x80) as i8 >> 7) as u8;
        let n = ((header >> 3) ^ mask) & 0x0F;
        let mut val = (((header ^ mask) & 0x07) as u64) << (n as usize * 8);
        for i in range_inclusive(1, n) {
            let byte = try!(self.reader.read_u8());
            val += ((byte ^ mask) as u64) << ((n - i) as usize * 8);
        }
        let final_mask = (((mask as i64) << 63) >> 63) as u64;
        val ^= final_mask;
        Ok(val as i64)
    }
}

impl<R> rustc_serialize::Decoder for Decoder<R>
where R: io::Read {

    type Error = Error;

    fn read_nil(&mut self) -> Result<()> { Ok(()) }

    fn read_u8(&mut self) -> Result<u8> {
        self.reader.read_u8().map_err(error::FromError::from_error)
    }
    fn read_u16(&mut self) -> Result<u16> {
        self.reader.read_u16::<BigEndian>().map_err(error::FromError::from_error)
    }
    fn read_u32(&mut self) -> Result<u32> {
        self.reader.read_u32::<BigEndian>().map_err(error::FromError::from_error)
    }
    fn read_u64(&mut self) -> Result<u64> {
        self.reader.read_u64::<BigEndian>().map_err(error::FromError::from_error)
    }
    fn read_usize(&mut self) -> Result<usize> {
        let val = try!(self.read_var_u64());
        Ok(val as usize)
    }

    fn read_i8(&mut self) -> Result<i8> {
        let val = try!(self.reader.read_i8());
        Ok(val ^ i8::MIN)
    }
    fn read_i16(&mut self) -> Result<i16> {
        let val = try!(self.reader.read_i16::<BigEndian>());
        Ok(val ^ i16::MIN)
    }
    fn read_i32(&mut self) -> Result<i32> {
        let val = try!(self.reader.read_i32::<BigEndian>());
        Ok(val ^ i32::MIN)
    }
    fn read_i64(&mut self) -> Result<i64> {
        let val = try!(self.reader.read_i64::<BigEndian>());
        Ok(val ^ i64::MIN)
    }
    fn read_isize(&mut self) -> Result<isize> {
        let val = try!(self.read_var_i64());
        Ok(val as isize)
    }

    fn read_bool(&mut self) -> Result<bool> {
        match try!(self.reader.read_u8()) {
            0 => Ok(false),
            _ => Ok(true)
        }
    }

    fn read_f32(&mut self) -> Result<f32> {
        let val = try!(self.reader.read_i32::<BigEndian>());
        let t = ((val ^ i32::MIN) >> 31) | i32::MIN;
        let f = unsafe { transmute(val ^ t) };
        Ok(f)
    }
    fn read_f64(&mut self) -> Result<f64> {
        let val = try!(self.reader.read_i64::<BigEndian>());
        let t = ((val ^ i64::MIN) >> 63) | i64::MIN;
        let f = unsafe { transmute(val ^ t) };
        Ok(f)
    }

    fn read_char(&mut self) -> Result<char> {
        match (&mut self.reader).chars().next() {
            Some(Ok(c)) => Ok(c),
            Some(Err(io::CharsError::NotUtf8)) => Err(Error::NotUtf8),
            Some(Err(io::CharsError::Other(error))) => Err(Error::Io(error)),
            None => Err(Error::UnexpectedEof),
        }
    }

    fn read_str(&mut self) -> Result<String> {
        let mut string = String::new();

        loop {
            let c = try!(self.read_char());
            if c == '\0' { break; }
            string.push(c);
        }

        Ok(string)
    }

    fn read_enum<T, F>(&mut self, _name: &str, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }
    fn read_enum_variant<T, F>(&mut self, _names: &[&str], mut f: F) -> Result<T>
            where F: FnMut(&mut Self, usize) -> Result<T> {
        let id = try!(self.read_usize());
        f(self, id)
    }
    fn read_enum_variant_arg<T, F>(&mut self, _idx: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }
    fn read_enum_struct_variant<T, F>(&mut self, names: &[&str], f: F) -> Result<T>
            where F: FnMut(&mut Self, usize) -> Result<T> {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T, F>(&mut self,
                                            _name: &str,
                                            idx: usize,
                                            f: F)
                                            -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        self.read_enum_variant_arg(idx, f)
    }

    fn read_struct<T, F>(&mut self, _name: &str, _len: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }
    fn read_struct_field<T, F>(&mut self, _name: &str, _idx: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }

    fn read_tuple<T, F>(&mut self, _len: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }
    fn read_tuple_arg<T, F>(&mut self, _idx: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        f(self)
    }

    fn read_tuple_struct<T, F>(&mut self, _name: &str, len: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        self.read_tuple(len, f)
    }
    fn read_tuple_struct_arg<T, F>(&mut self, idx: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        self.read_tuple_arg(idx, f)
    }

    fn read_option<T, F>(&mut self, mut f: F) -> Result<T>
            where F: FnMut(&mut Self, bool) -> Result<T> {
        let is_some = try!(self.read_bool());
        f(self, is_some)
    }

    fn read_seq<T, F>(&mut self, _f: F) -> Result<T>
            where F: FnOnce(&mut Self, usize) -> Result<T> {
        unimplemented!()
    }
    fn read_seq_elt<T, F>(&mut self, _idx: usize, _f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        unimplemented!()
    }

    fn read_map<T, F>(&mut self, _f: F) -> Result<T>
            where F: FnOnce(&mut Self, usize) -> Result<T> {
        unimplemented!()
    }
    fn read_map_elt_key<T, F>(&mut self, _idx: usize, _f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        unimplemented!()
    }
    fn read_map_elt_val<T, F>(&mut self, _idx: usize, _f: F) -> Result<T>
            where F: FnOnce(&mut Self) -> Result<T> {
        unimplemented!()
    }

    fn error(&mut self, err: &str) -> Error {
        Error::Io(io::Error::new(io::ErrorKind::Other, "", Some(err.to_string())))
    }
}

#[cfg(test)]
mod test {

    use std::{f32, f64, isize, usize};
    use std::num::Int;

    use decode;
    use encode;
    use encoder::test::{TestStruct, TestEnum};

    #[quickcheck]
    fn check_u8(val: u8) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_u16(val: u16) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_u32(val: u32) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_u64(val: u64) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_usize(val: usize) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[test]
    fn test_usize() {
        let values = vec![
            0usize, 2usize.pow(0),
            2usize.pow(4)  - 1, 2usize.pow(4),
            2usize.pow(12) - 1, 2usize.pow(12),
            2usize.pow(20) - 1, 2usize.pow(20),
            2usize.pow(28) - 1, 2usize.pow(28),
            2usize.pow(36) - 1, 2usize.pow(36),
            2usize.pow(44) - 1, 2usize.pow(44),
            2usize.pow(52) - 1, 2usize.pow(52),
            2usize.pow(60) - 1, 2usize.pow(60),
            usize::MAX,
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val).unwrap()).unwrap());
        }
    }

    #[quickcheck]
    fn check_i8(val: i8) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_i16(val: i16) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_i32(val: i32) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_i64(val: i64) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[quickcheck]
    fn check_isize(val: isize) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[test]
    fn test_isize() {
        let values = vec![
            -2isize.pow(0), 0isize, 2isize.pow(0),
            -2isize.pow(3)  - 1, -2isize.pow(3),  2isize.pow(3)  - 1, 2isize.pow(3),
            -2isize.pow(11) - 1, -2isize.pow(11), 2isize.pow(11) - 1, 2isize.pow(11),
            -2isize.pow(19) - 1, -2isize.pow(19), 2isize.pow(19) - 1, 2isize.pow(19),
            -2isize.pow(27) - 1, -2isize.pow(27), 2isize.pow(27) - 1, 2isize.pow(27),
            -2isize.pow(35) - 1, -2isize.pow(35), 2isize.pow(35) - 1, 2isize.pow(35),
            -2isize.pow(43) - 1, -2isize.pow(43), 2isize.pow(43) - 1, 2isize.pow(43),
            -2isize.pow(51) - 1, -2isize.pow(51), 2isize.pow(51) - 1, 2isize.pow(51),
            -2isize.pow(59) - 1, -2isize.pow(59), 2isize.pow(59) - 1, 2isize.pow(59),
            isize::MIN, isize::MAX,
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val).unwrap()).unwrap());
        }
    }

    #[quickcheck]
    fn check_f32(val: f32) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[test]
    fn test_f32() {
        let values = vec![
            f32::NEG_INFINITY,
            f32::MIN,
            -0.0,
            0.0,
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::INFINITY
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val).unwrap()).unwrap());
        }
    }

    #[quickcheck]
    fn check_f64(val: f64) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
    #[test]
    fn test_f64() {
        let values = vec![
            f64::NEG_INFINITY,
            f64::MIN,
            -0.0,
            0.0,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::INFINITY
        ];
        for val in values.iter() {
            assert_eq!(*val, decode(encode(val).unwrap()).unwrap());
        }
    }

    #[quickcheck]
    fn check_char(val: char) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }

    #[quickcheck]
    fn check_string(val: String) -> bool {
        val == decode::<String>(encode(&val).unwrap()).unwrap()
    }

    #[quickcheck]
    fn check_option(val: Option<String>) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }

    #[quickcheck]
    fn check_struct(val: TestStruct) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }

     #[quickcheck]
    fn check_tuple(val: (usize, char, String)) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }

    #[quickcheck]
    fn check_enum(val: TestEnum) -> bool {
        val == decode(encode(&val).unwrap()).unwrap()
    }
}
