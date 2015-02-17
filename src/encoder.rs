use rustc_serialize::{self, Encodable};
use std::{i8, i16, i32, i64};
use std::old_io as io;
use std::mem::transmute;
use std::num::SignedInt;

/// An encoder for serializing data to a byte format that preserves lexicographic sort order.
///
/// The byte format is designed with a few goals:
///
/// * Order must be preserved
/// * Serialized representations should be as compact as possible
/// * Type information is *not* serialized with values
///
/// #### Supported Data Types
///
/// ##### Unsigned isizeegers
///
/// `u8`, `u16`, `u32`, and `u64` are encoded isizeo 1, 2, 4, and 8 bytes of output, respectively.
/// Order is preserved by encoding the bytes in big-endian (most-significant bytes first) format.
///
/// `usize` is variable-length encoded isizeo between 1 and 9 bytes.  Smaller magnitude values (closer
/// to 0) will encode isizeo fewer bytes. See `emit_var_u64` for details on serialization
/// size and format.
///
/// ##### Signed isizeegers
///
/// `i8`, `i16`, `i32`, and `i64` are encoded isizeo 1, 2, 4, and 8 bytes of output, respectively.
/// Order is preserved by taking the bitwise complement of the value, and encoding the resulting
/// bytes in big-endian format.
///
/// `isize` is variable-length encoded isizeo between 1 and 9 bytes. Smaller magnitude values (closer
/// to 0) will encode isizeo fewer bytes. See `emit_var_i64` for details on serialization
/// size and format.
///
/// ##### Floating Poisize Numbers
///
/// `f32` and `f64` are encoded isizeo 4 and 8 bytes of output, respectively. Order is preserved
/// by encoding the value, or the bitwise complement of the value if negative, isizeo bytes in
/// big-endian format. `NAN` values will sort after all other values. In general, it is
/// unwise to use IEEE 754 floating poisize values in keys, because rounding errors are pervasive.
/// It is typically hard or impossible to use an approximate 'epsilon' approach when using keys for
/// lookup.
///
/// ##### Characters
///
/// Characters are serialized isizeo between 1 and 4 bytes of output.
///
/// ##### Booleans
///
/// Booleans are serialized isizeo a single byte of output. `false` values will sort before `true`
/// values.
///
/// ##### Strings
///
/// Strings are encoded isizeo their natural UTF8 representation plus a single null byte suffix.
/// In general, strings should not contain null bytes. The encoder will not check for null bytes,
/// however their presence will break lexicographic sorting. The only exception to this rule is
/// the case where the string is the final (or only) component of the key. If the string field
/// is the final component of a tuple, enum-struct, or struct, then it may contain null bytes
/// without breaking sort order.
///
/// ##### Options
///
/// An optional wrapper type adds a 1 byte overhead to the wrapped data type. `None` values will
/// sort before `Some` values.
///
/// ##### Structs & Tuples
///
/// Structs and tuples are encoded by serializing their consituent fields in order with no prefix,
/// suffix, or padding bytes.
///
/// ##### Enums
///
/// Enums are encoded with a variable-length unsigned-isizeeger variant tag, plus the consituent
/// fields in the case of an enum-struct. The tag adds an overhead of between 1 and 9 bytes (it
/// will be a single byte for up to 16 variants). This encoding allows more enum variants to be
/// added in a backwards-compatible manner, as long as variants are not removed and the variant
/// order does not change.
///
/// #### Unsupported Data Types
///
/// Sequences and maps are unsupported at this time. Sequences and maps could probably be
/// implemented with a single byte overhead per item, key, and value, but these types are not
/// typically useful in keys.
///
/// Raw byte arrays are unsupported. The Rust `Encoder`/`Decoder` mechanism makes no distinction
/// between byte arrays and sequences, and thus the overhead for encoding a raw byte array would be
/// 1 byte per input byte. The theoretical best-case overhead for serializing a raw (null
/// containing) byte array in order-preserving format is 1 bit per byte, or 9 bytes of output for
/// every 8 bytes of input.
pub struct Encoder<'a> {
    writer: &'a mut (io::Writer+'a),
}

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
pub fn encode<'a, T : Encodable>(object: &T) -> Result<Vec<u8>, io::IoError>  {
    let mut writer = Vec::new();
    {
        let mut encoder = Encoder::new(&mut writer);
        try!(object.encode(&mut encoder));
    }
    Ok(writer)
}

impl<'a> Encoder<'a> {

    /// Creates a new ordered bytes encoder whose output will be written to the provided writer.
    pub fn new(writer: &'a mut io::Writer) -> Encoder<'a> {
        Encoder { writer: writer }
    }

    /// Encode a `u64` isizeo a variable number of bytes.
    ///
    /// The variable-length encoding scheme uses between 1 and 9 bytes depending on the value.
    /// Smaller magnitude (closer to 0) `u64`s will encode to fewer bytes.
    ///
    /// ##### Encoding
    ///
    /// The encoding uses the first 4 bits to store the number of trailing bytes, between 0 and 8.
    /// Subsequent bits are the input value in big-endian format with leading 0 bytes removed.
    ///
    /// ##### Encoded Size
    ///
    /// <table>
    ///     <tr>
    ///         <th>range</th>
    ///         <th>size (bytes)</th>
    ///     </tr>
    ///     <tr>
    ///         <td>[0, 2<sup>4</sup>)</td>
    ///         <td>1</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>4</sup>, 2<sup>12</sup>)</td>
    ///         <td>2</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>12</sup>, 2<sup>20</sup>)</td>
    ///         <td>3</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>20</sup>, 2<sup>28</sup>)</td>
    ///         <td>4</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>28</sup>, 2<sup>36</sup>)</td>
    ///         <td>5</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>36</sup>, 2<sup>44</sup>)</td>
    ///         <td>6</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>44</sup>, 2<sup>52</sup>)</td>
    ///         <td>7</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>52</sup>, 2<sup>60</sup>)</td>
    ///         <td>8</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[2<sup>60</sup>, 2<sup>64</sup>)</td>
    ///         <td>9</td>
    ///     </tr>
    /// </table>
    pub fn emit_var_u64(&mut self, val: u64) -> EncodeResult {
        if val < 1 << 4 {
            self.writer.write_u8(val as u8)
        } else if val < 1 << 12 {
            self.writer.write_be_u16((val as u16) | 1 << 12)
        } else if val < 1 << 20 {
            try!(self.writer.write_u8(((val >> 16) as u8) | 2 << 4));
            self.writer.write_be_u16((val as u16))
        } else if val < 1 << 28 {
            self.writer.write_be_u32((val as u32) | 3 << 28)
        } else if val < 1 << 36 {
            try!(self.writer.write_u8(((val >> 32) as u8) | 4 << 4));
            self.writer.write_be_u32((val as u32))
        } else if val < 1 << 44 {
            try!(self.writer.write_be_u16(((val >> 32) as u16) | 5 << 12));
            self.writer.write_be_u32((val as u32))
        } else if val < 1 << 52 {
            try!(self.writer.write_u8(((val >> 48) as u8) | 6 << 4));
            try!(self.writer.write_be_u16((val >> 32) as u16));
            self.writer.write_be_u32((val as u32))
        } else if val < 1 << 60 {
            self.writer.write_be_u64((val as u64) | 7 << 60)
        } else {
            try!(self.writer.write_u8(8 << 4));
            self.writer.write_be_u64(val)
        }
    }

    /// Encode an `i64` isizeo a variable number of bytes.
    ///
    /// The variable-length encoding scheme uses between 1 and 9 bytes depending on the value.
    /// Smaller magnitude (closer to 0) `i64`s will encode to fewer bytes.
    ///
    /// ##### Encoding
    ///
    /// The encoding uses the first bit to encode the sign: `0` for negative values and `1` for
    /// positive values. The following 4 bits store the number of trailing bytes, between 0 and 8.
    /// Subsequent bits are the absolute value of the input value in big-endian format with leading
    /// 0 bytes removed. If the original value was negative, than 1 is subtracted from the absolute
    /// value before encoding. Finally, if the value is negative, all bits except the sign bit are
    /// flipped (1s become 0s and 0s become 1s).
    ///
    /// ##### Encoded Size
    ///
    /// <table>
    ///     <tr>
    ///         <th>negative range</th>
    ///         <th>positive range</th>
    ///         <th>size (bytes)</th>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>3</sup>, 0)</td>
    ///         <td>[0, 2<sup>3</sup>)</td>
    ///         <td>1</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>11</sup>, -2<sup>3</sup>)</td>
    ///         <td>[2<sup>3</sup>, 2<sup>11</sup>)</td>
    ///         <td>2</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>19</sup>, -2<sup>11</sup>)</td>
    ///         <td>[2<sup>11</sup>, 2<sup>19</sup>)</td>
    ///         <td>3</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>27</sup>, -2<sup>19</sup>)</td>
    ///         <td>[2<sup>19</sup>, 2<sup>27</sup>)</td>
    ///         <td>4</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>35</sup>, -2<sup>27</sup>)</td>
    ///         <td>[2<sup>27</sup>, 2<sup>35</sup>)</td>
    ///         <td>5</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>43</sup>, -2<sup>35</sup>)</td>
    ///         <td>[2<sup>35</sup>, 2<sup>43</sup>)</td>
    ///         <td>6</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>51</sup>, -2<sup>43</sup>)</td>
    ///         <td>[2<sup>43</sup>, 2<sup>51</sup>)</td>
    ///         <td>7</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>59</sup>, -2<sup>51</sup>)</td>
    ///         <td>[2<sup>51</sup>, 2<sup>59</sup>)</td>
    ///         <td>8</td>
    ///     </tr>
    ///     <tr>
    ///         <td>[-2<sup>63</sup>, -2<sup>59</sup>)</td>
    ///         <td>[2<sup>59</sup>, 2<sup>63</sup>)</td>
    ///         <td>9</td>
    ///     </tr>
    /// </table>
    pub fn emit_var_i64(&mut self, v: i64) -> EncodeResult {
        // The mask is 0 for positive input and u64::MAX for negative input
        let mask = (v >> 63) as u64;
        let val = v.abs() as u64 - (1 & mask);
        println!("v: {}, val: {}", v, val);
        if val < 1 << 3 {
            let masked = (val | (0x10 << 3)) ^ mask;
            self.writer.write_u8(masked as u8)
        } else if val < 1 << 11 {
            let masked = (val | (0x11 << 11)) ^ mask;
            self.writer.write_be_u16(masked as u16)
        } else if val < 1 << 19 {
            let masked = (val | (0x12 << 19)) ^ mask;
            try!(self.writer.write_u8((masked >> 16) as u8));
            self.writer.write_be_u16(masked as u16)
        } else if val < 1 << 27 {
            let masked = (val | (0x13 << 27)) ^ mask;
            self.writer.write_be_u32(masked as u32)
        } else if val < 1 << 35 {
            let masked = (val | (0x14 << 35)) ^ mask;
            try!(self.writer.write_u8((masked >> 32) as u8));
            self.writer.write_be_u32(masked as u32)
        } else if val < 1 << 43 {
            let masked = (val | (0x15 << 43)) ^ mask;
            try!(self.writer.write_be_u16((masked >> 32) as u16));
            self.writer.write_be_u32(masked as u32)
        } else if val < 1 << 51 {
            let masked = (val | (0x16 << 51)) ^ mask;
            try!(self.writer.write_u8((masked >> 48) as u8));
            try!(self.writer.write_be_u16((masked >> 32) as u16));
            self.writer.write_be_u32(masked as u32)
        } else if val < 1 << 59 {
            let masked = (val | (0x17 << 59)) ^ mask;
            self.writer.write_be_u64(masked as u64)
        } else {
            try!(self.writer.write_u8((0x18 << 3) ^ mask as u8));
            self.writer.write_be_u64(val ^ mask)
        }
    }
}

/// The error type returned by all encoding operations supported by `Encoder`.
pub type EncodeResult = io::IoResult<()>;

impl<'a> rustc_serialize::Encoder for Encoder<'a> {

    type Error = io::IoError;

    fn emit_nil(&mut self) -> EncodeResult { self.writer.write_all([].as_slice()) }

    fn emit_u8(&mut self, v: u8) -> EncodeResult  { self.writer.write_u8(v) }
    fn emit_u16(&mut self, v: u16) -> EncodeResult { self.writer.write_be_u16(v) }
    fn emit_u32(&mut self, v: u32) -> EncodeResult { self.writer.write_be_u32(v) }
    fn emit_u64(&mut self, v: u64) -> EncodeResult { self.writer.write_be_u64(v) }
    fn emit_usize(&mut self, v: usize) -> EncodeResult { self.emit_var_u64(v as u64) }

    fn emit_i8(&mut self, v: i8) -> EncodeResult  { self.writer.write_i8(v ^ i8::MIN) }
    fn emit_i16(&mut self, v: i16) -> EncodeResult { self.writer.write_be_i16(v ^ i16::MIN) }
    fn emit_i32(&mut self, v: i32) -> EncodeResult { self.writer.write_be_i32(v ^ i32::MIN) }
    fn emit_i64(&mut self, v: i64) -> EncodeResult { self.writer.write_be_i64(v ^ i64::MIN) }
    fn emit_isize(&mut self, v: isize) -> EncodeResult { self.emit_var_i64(v as i64) }

    fn emit_bool(&mut self, v: bool) -> EncodeResult { self.writer.write_u8(if v { 1 } else { 0 }) }

    /// Encode an `f32` isizeo sortable bytes.
    ///
    /// `NaN`s will sort greater than positive infinity. -0.0 will sort directly before +0.0.
    ///
    /// See [Hacker's Delight 2nd Edition](http://www.hackersdelight.org/) Section 17-3.
    fn emit_f32(&mut self, v: f32) -> EncodeResult {
        let val = unsafe { transmute::<f32, i32>(v) };
        let t = (val >> 31) | i32::MIN;
        self.writer.write_be_i32(val ^ t)
    }

    /// Encode an `f64` isizeo sortable bytes.
    ///
    /// `NaN`s will sort greater than positive infinity. -0.0 will sort directly before +0.0.
    ///
    /// See [Hacker's Delight 2nd Edition](http://www.hackersdelight.org/) Section 17-3.
    fn emit_f64(&mut self, v: f64) -> EncodeResult {
        let val = unsafe { transmute::<f64, i64>(v) };
        let t = (val >> 63) | i64::MIN;
        self.writer.write_be_i64(val ^ t)
    }

    fn emit_char(&mut self, v: char) -> EncodeResult {
        self.writer.write_char(v)
    }

    fn emit_str(&mut self, v: &str) -> EncodeResult {
        try!(self.writer.write_all(v.as_bytes()));
        self.writer.write_u8(0u8)
    }

    fn emit_enum<F>(&mut self, _name: &str, f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_enum_variant<F>(&mut self,
                            _name: &str,
                            id: usize,
                            _len: usize,
                            f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        try!(self.emit_usize(id));
        f(self)
    }
    fn emit_enum_variant_arg<F>(&mut self,
                                _idx: usize,
                                f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_enum_struct_variant<F>(&mut self,
                                   _name: &str,
                                   id: usize,
                                   _len: usize,
                                   f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        try!(self.emit_usize(id));
        f(self)
    }
    fn emit_enum_struct_variant_field<F>(&mut self,
                                         _name: &str,
                                         _idx: usize,
                                         f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }

    fn emit_struct<F>(&mut self, _name: &str, _len: usize, f: F)
                      -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_struct_field<F>(&mut self, _name: &str, _idx: usize, f: F)
                            -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }

    fn emit_tuple<F>(&mut self, _len: usize, f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_tuple_arg<F>(&mut self, _idx: usize, f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_tuple_struct<F>(&mut self,
                            name: &str,
                            len: usize,
                            f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        self.emit_struct(name, len, f)
    }
    fn emit_tuple_struct_arg<F>(&mut self,
                                idx: usize,
                                f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        self.emit_struct_field("", idx, f)
    }

    fn emit_option<F>(&mut self, f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        f(self)
    }
    fn emit_option_none(&mut self) -> Result<(), io::IoError> {
        self.emit_bool(false)
    }
    fn emit_option_some<F>(&mut self, f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        try!(self.emit_bool(true));
        f(self)
    }

    fn emit_seq<F>(&mut self, _len: usize, _f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
         unimplemented!()
    }
    fn emit_seq_elt<F>(&mut self, _idx: usize, _f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        unimplemented!()
    }

    fn emit_map<F>(&mut self, _len: usize, _f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        unimplemented!()
    }
    fn emit_map_elt_key<F>(&mut self, _idx: usize, _f: F) -> Result<(), io::IoError> {
        unimplemented!()
    }
    fn emit_map_elt_val<F>(&mut self, _idx: usize, _f: F) -> Result<(), io::IoError>
            where F: FnOnce(&mut Self) -> Result<(), io::IoError> {
        unimplemented!()
    }
}

#[cfg(test)]
pub mod test {

    use rand::Rng;
    use std::{u8, u16, i8, i16, f32, f64};
    use std::iter::range_inclusive;
    use std::num::{Int, Float};

    use quickcheck::{Arbitrary, Gen};

    use encoder::encode;

    #[test]
    fn test_u8() {
        let mut previous = encode(&u8::MIN).unwrap();
        for i in range_inclusive(u8::MIN + 1, u8::MAX) {
            let current = encode(&i).unwrap();
            assert!(current > previous);
            previous = current;
        }
    }

    #[test]
    fn test_u16() {
        let mut previous = encode(&u16::MIN).unwrap();
        for i in range_inclusive(u16::MIN + 1, u16::MAX) {
            let current = encode(&i).unwrap();
            assert!(current > previous);
            previous = current;
        }
    }

    #[quickcheck]
    fn check_u32(a: u32, b: u32) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_u64(a: u64, b: u64) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[test]
    fn test_var_u64() {
        assert_eq!(vec!(0x00), encode(&0us).unwrap());
        assert_eq!(vec!(0x01), encode(&2us.pow(0)).unwrap());

        assert_eq!(vec!(0x0F), encode(&(2us.pow(4) - 1)).unwrap());
        assert_eq!(vec!(0x10, 0x10), encode(&2us.pow(4)).unwrap());

        assert_eq!(vec!(0x1F, 0xFF), encode(&(2us.pow(12) - 1)).unwrap());
        assert_eq!(vec!(0x20, 0x10, 0x00), encode(&2us.pow(12)).unwrap());

        assert_eq!(vec!(0x2F, 0xFF, 0xFF), encode(&(2us.pow(20) - 1)).unwrap());
        assert_eq!(vec!(0x30, 0x10, 0x00, 0x00), encode(&2us.pow(20)).unwrap());

        assert_eq!(vec!(0x3F, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(28) - 1)).unwrap());
        assert_eq!(vec!(0x40, 0x10, 0x00, 0x00, 0x00), encode(&2us.pow(28)).unwrap());

        assert_eq!(vec!(0x4F, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(36) - 1)).unwrap());
        assert_eq!(vec!(0x50, 0x10, 0x00, 0x00, 0x00, 0x00), encode(&2us.pow(36)).unwrap());

        assert_eq!(vec!(0x5F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(44) - 1)).unwrap());
        assert_eq!(vec!(0x60, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2us.pow(44)).unwrap());

        assert_eq!(vec!(0x6F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(52) - 1)).unwrap());
        assert_eq!(vec!(0x70, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2us.pow(52)).unwrap());

        assert_eq!(vec!(0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(60) - 1)).unwrap());
        assert_eq!(vec!(0x80, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2us.pow(60)).unwrap());

        assert_eq!(vec!(0x80, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2us.pow(64) - 1)).unwrap());
    }

    #[quickcheck]
    fn check_usize(a: usize, b: usize) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[test]
    fn test_i8() {
        let mut previous = encode(&i8::MIN).unwrap();
        for i in range_inclusive(i8::MIN + 1, i8::MAX) {
            let current = encode(&i).unwrap();
            assert!(current > previous);
            previous = current;
        }
    }

    #[test]
    fn test_i16() {
        let mut previous = encode(&i16::MIN).unwrap();
        for i in range_inclusive(i16::MIN + 1, i16::MAX) {
            let current = encode(&i).unwrap();
            assert!(current > previous);
            previous = current;
        }
    }

    #[quickcheck]
    fn check_i32(a: i32, b: i32) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_i64(a: i64, b: i64) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[test]
    fn test_pos_var_i64() {
        assert_eq!(vec!(0x80), encode(&0is).unwrap());
        assert_eq!(vec!(0x81), encode(&2is.pow(0)).unwrap());

        assert_eq!(vec!(0x87), encode(&(2is.pow(3) - 1)).unwrap());
        assert_eq!(vec!(0x88, 0x08), encode(&2is.pow(3)).unwrap());

        assert_eq!(vec!(0x8F, 0xFF), encode(&(2is.pow(11) - 1)).unwrap());
        assert_eq!(vec!(0x90, 0x08, 0x00), encode(&2is.pow(11)).unwrap());

        assert_eq!(vec!(0x97, 0xFF, 0xFF), encode(&(2is.pow(19) - 1)).unwrap());
        assert_eq!(vec!(0x98, 0x08, 0x00, 0x00), encode(&2is.pow(19)).unwrap());

        assert_eq!(vec!(0x9F, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(27) - 1)).unwrap());
        assert_eq!(vec!(0xA0, 0x08, 0x00, 0x00, 0x00), encode(&2is.pow(27)).unwrap());

        assert_eq!(vec!(0xA7, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(35) - 1)).unwrap());
        assert_eq!(vec!(0xA8, 0x08, 0x00, 0x00, 0x00, 0x00), encode(&2is.pow(35)).unwrap());

        assert_eq!(vec!(0xAF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(43) - 1)).unwrap());
        assert_eq!(vec!(0xB0, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2is.pow(43)).unwrap());

        assert_eq!(vec!(0xB7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(51) - 1)).unwrap());
        assert_eq!(vec!(0xB8, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2is.pow(51)).unwrap());

        assert_eq!(vec!(0xBF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(59) - 1)).unwrap());
        assert_eq!(vec!(0xC0, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&2is.pow(59)).unwrap());

        assert_eq!(vec!(0xC0, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(2is.pow(63) - 1)).unwrap());
    }

    #[test]
    fn test_neg_var_i64() {
        assert_eq!(vec!(0x7F), encode(&(0is - 1)).unwrap());

        assert_eq!(vec!(0x78), encode(&-2is.pow(3)).unwrap());
        assert_eq!(vec!(0x77, 0xF7), encode(&(-2is.pow(3) - 1)).unwrap());

        assert_eq!(vec!(0x70, 0x00), encode(&-2is.pow(11)).unwrap());
        assert_eq!(vec!(0x6F, 0xF7, 0xFF), encode(&(-2is.pow(11) - 1)).unwrap());

        assert_eq!(vec!(0x68, 0x00, 0x00), encode(&-2is.pow(19)).unwrap());
        assert_eq!(vec!(0x67, 0xF7, 0xFF, 0xFF), encode(&(-2is.pow(19) - 1)).unwrap());

        assert_eq!(vec!(0x60, 0x00, 0x00, 0x00), encode(&-2is.pow(27)).unwrap());
        assert_eq!(vec!(0x5F, 0xF7, 0xFF, 0xFF, 0xFF), encode(&(-2is.pow(27) - 1)).unwrap());

        assert_eq!(vec!(0x58, 0x00, 0x00, 0x00, 0x00), encode(&-2is.pow(35)).unwrap());
        assert_eq!(vec!(0x57, 0xF7, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(-2is.pow(35) - 1)).unwrap());

        assert_eq!(vec!(0x50, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&-2is.pow(43)).unwrap());
        assert_eq!(vec!(0x4F, 0xF7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(-2is.pow(43) - 1)).unwrap());

        assert_eq!(vec!(0x48, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&-2is.pow(51)).unwrap());
        assert_eq!(vec!(0x47, 0xF7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(-2is.pow(51) - 1)).unwrap());

        assert_eq!(vec!(0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&-2is.pow(59)).unwrap());
        assert_eq!(vec!(0x3F, 0xF7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF), encode(&(-2is.pow(59) - 1)).unwrap());

        assert_eq!(vec!(0x3F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00), encode(&-2is.pow(63)).unwrap());
    }

    #[quickcheck]
    fn check_isize(a: isize, b: isize) -> bool {
        a.cmp(&b) == encode(&a).unwrap().cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_f32(a: f32, b: f32) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
            && a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&(b.next_after(a))).unwrap())
            && b.partial_cmp(&a) == encode(&b).unwrap().partial_cmp(&encode(&(a.next_after(b))).unwrap())
    }

    #[test]
    fn test_f32() {
        assert!(encode(&f32::NEG_INFINITY).unwrap() < encode(&f32::MIN_VALUE).unwrap());
        assert!(encode(&f32::MIN_VALUE).unwrap() < encode(&(f32::MIN_VALUE.next_after(f32::INFINITY))).unwrap());

        assert!(encode(&(-0.0f32).next_after(f32::NEG_INFINITY)).unwrap() < encode(&-0.0f32).unwrap());
        assert!(encode(&-0f32).unwrap() < encode(&0f32).unwrap());
        assert!(encode(&0f32).unwrap() < encode(&f32::MIN_POS_VALUE).unwrap());

        assert!(encode(&(f32::MAX_VALUE.next_after(f32::NEG_INFINITY))).unwrap() < encode(&f32::MAX_VALUE).unwrap());
        assert!(encode(&f32::MAX_VALUE).unwrap() < encode(&f32::INFINITY).unwrap());
        assert!(encode(&f32::INFINITY).unwrap() < encode(&f32::NAN).unwrap());
    }

    #[quickcheck]
    fn check_f64(a: f64, b: f64) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
            && a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&(b.next_after(a))).unwrap())
            && b.partial_cmp(&a) == encode(&b).unwrap().partial_cmp(&encode(&(a.next_after(b))).unwrap())
    }

    #[test]
    fn test_f64() {
        assert!(encode(&f64::NEG_INFINITY).unwrap() < encode(&f64::MIN_VALUE).unwrap());
        assert!(encode(&f64::MIN_VALUE).unwrap() < encode(&(f64::MIN_VALUE.next_after(f64::INFINITY))).unwrap());

        assert!(encode(&(-0.0f64).next_after(f64::NEG_INFINITY)).unwrap() < encode(&-0.0f64).unwrap());
        assert!(encode(&-0f64).unwrap() < encode(&0f64).unwrap());
        assert!(encode(&0f64).unwrap() < encode(&f64::MIN_POS_VALUE).unwrap());

        assert!(encode(&(f64::MAX_VALUE.next_after(f64::NEG_INFINITY))).unwrap() < encode(&f64::MAX_VALUE).unwrap());
        assert!(encode(&f64::MAX_VALUE).unwrap() < encode(&f64::INFINITY).unwrap());
        assert!(encode(&f64::INFINITY).unwrap() < encode(&f64::NAN).unwrap());
    }

    #[test]
    fn test_bool() {
        for &(a, b) in vec!((true, true), (true, false), (false, true), (false, false)).iter() {
            assert_eq!(a.partial_cmp(&b), encode(&a).unwrap().partial_cmp(&encode(&b).unwrap()))
        }
    }

    #[quickcheck]
    fn check_char(a: char, b: char) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_string(a: String, b: String) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_option(a: Option<String>, b: Option<String>) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_struct(a: TestStruct, b: TestStruct) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_tuple(a: (u32, char, String), b: (u32, char, String)) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[quickcheck]
    fn check_enum(a: TestEnum, b: TestEnum) -> bool {
        a.partial_cmp(&b) == encode(&a).unwrap().partial_cmp(&encode(&b).unwrap())
    }

    #[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq, PartialOrd)]
    pub struct TestStruct {
        u8_: u8,
        u16_: u16,
        u32_: u32,
        u64_: u64,
        usize_: usize,

        i8_: i8,
        i16_: i16,
        i32_: i32,
        i64_: i64,
        isize_: isize,

        f32_: f32,
        f64_: f64,

        bool_: bool,
        char_: char,

        string: String,
    }

    impl Arbitrary for TestStruct {
        fn arbitrary<G: Gen>(g: &mut G) -> TestStruct {
            TestStruct {
                u8_: Arbitrary::arbitrary(g),
                u16_: Arbitrary::arbitrary(g),
                u32_: Arbitrary::arbitrary(g),
                u64_: Arbitrary::arbitrary(g),
                usize_: Arbitrary::arbitrary(g),

                i8_: Arbitrary::arbitrary(g),
                i16_: Arbitrary::arbitrary(g),
                i32_: Arbitrary::arbitrary(g),
                i64_: Arbitrary::arbitrary(g),
                isize_: Arbitrary::arbitrary(g),

                f32_: Arbitrary::arbitrary(g),
                f64_: Arbitrary::arbitrary(g),

                bool_: Arbitrary::arbitrary(g),
                char_: Arbitrary::arbitrary(g),

                string: Arbitrary::arbitrary(g)
            }
        }
    }

    #[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq, PartialOrd)]
    pub enum TestEnum {
        A(u32, String),
        B,
        C(isize)
    }

    impl Arbitrary for TestEnum {
        fn arbitrary<G: Gen>(g: &mut G) -> TestEnum {
            let mut variants = vec![
                TestEnum::A(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)),
                TestEnum::B,
                TestEnum::C(Arbitrary::arbitrary(g))
            ];

            g.shuffle(variants.as_mut_slice());
            variants.pop().unwrap()
        }
    }
}
