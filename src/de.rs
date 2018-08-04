use byteorder::{ReadBytesExt, BE};
use serde;
use serde::de::{Deserialize, Visitor};
use std;
use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Read};
use std::mem::transmute;
use std::{i16, i32, i64, i8};
use utf8;

/// A decoder for deserializing bytes from an order preserving format to a value.
///
/// Please see the **Serializer** documentation for a precise overview of the `bytekey` format.
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
}

/// Errors that may be occur when deserializing.
#[derive(Debug)]
pub enum Error {
    DeserializeAnyUnsupported,
    UnexpectedEof,
    InvalidUtf8,
    Io(io::Error),
    Message(String),
}

/// Shorthand for `Result<T, bytekey::de::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Deserialize data from the given slice of bytes.
///
/// #### Usage
///
/// ```
/// # use bytekey::{serialize, deserialize};
/// let bytes = serialize(&42usize).unwrap();
/// assert_eq!(42usize, deserialize::<usize>(&bytes).unwrap());
/// ```
pub fn deserialize<T>(bytes: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    deserialize_from(bytes)
}

/// Deserialize data from the given byte reader.
///
/// #### Usage
///
/// ```
/// # use bytekey::{serialize, deserialize_from};
/// let bytes = serialize(&42u64).unwrap();
/// let result: u64 = deserialize_from(&bytes[..]).unwrap();
/// assert_eq!(42u64, result);
/// ```
pub fn deserialize_from<R, T>(reader: R) -> Result<T>
where
    R: io::BufRead,
    T: for<'de> Deserialize<'de>,
{
    let mut deserializer = Deserializer::new(reader);
    T::deserialize(&mut deserializer)
}

impl<R: io::Read> Deserializer<R> {
    /// Creates a new ordered bytes encoder whose output will be written to the provided writer.
    pub fn new(reader: R) -> Deserializer<R> {
        Deserializer { reader: reader }
    }

    /// Deserialize a `u64` that has been serialized using the `serialize_var_u64` method.
    pub fn deserialize_var_u64(&mut self) -> Result<u64> {
        let header = try!(self.reader.read_u8());
        let n = header >> 4;
        let (mut val, _) = ((header & 0x0F) as u64).overflowing_shl(n as u32 * 8);
        for i in 1..n + 1 {
            let byte = try!(self.reader.read_u8());
            val += (byte as u64) << ((n - i) * 8);
        }
        Ok(val)
    }

    /// Deserialize an `i64` that has been serialized using the `serialize_var_i64` method.
    pub fn deserialize_var_i64(&mut self) -> Result<i64> {
        let header = try!(self.reader.read_u8());
        let mask = ((header ^ 0x80) as i8 >> 7) as u8;
        let n = ((header >> 3) ^ mask) & 0x0F;
        let (mut val, _) = (((header ^ mask) & 0x07) as u64).overflowing_shl(n as u32 * 8);
        for i in 1..n + 1 {
            let byte = try!(self.reader.read_u8());
            val += ((byte ^ mask) as u64) << ((n - i) * 8);
        }
        let final_mask = (((mask as i64) << 63) >> 63) as u64;
        val ^= final_mask;
        Ok(val as i64)
    }
}

impl<'de, 'a, R> serde::de::Deserializer<'de> for &'a mut Deserializer<R>
where
    R: io::BufRead,
{
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::DeserializeAnyUnsupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let b = match self.reader.read_u8()? {
            0 => false,
            _ => true,
        };
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let i = self.reader.read_i8()?;
        visitor.visit_i8(i ^ i8::MIN)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let i = self.reader.read_i16::<BE>()?;
        visitor.visit_i16(i ^ i16::MIN)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let i = self.reader.read_i32::<BE>()?;
        visitor.visit_i32(i ^ i32::MIN)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let i = self.reader.read_i64::<BE>()?;
        visitor.visit_i64(i ^ i64::MIN)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let u = self.reader.read_u8()?;
        visitor.visit_u8(u)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let u = self.reader.read_u16::<BE>()?;
        visitor.visit_u16(u)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let u = self.reader.read_u32::<BE>()?;
        visitor.visit_u32(u)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let u = self.reader.read_u64::<BE>()?;
        visitor.visit_u64(u)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.reader.read_i32::<BE>()?;
        let t = ((val ^ i32::MIN) >> 31) | i32::MIN;
        let f: f32 = unsafe { transmute(val ^ t) };
        visitor.visit_f32(f)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = self.reader.read_i64::<BE>()?;
        let t = ((val ^ i64::MIN) >> 63) | i64::MIN;
        let f: f64 = unsafe { transmute(val ^ t) };
        visitor.visit_f64(f)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut utf8_decoder = utf8::BufReadDecoder::new(&mut self.reader);
        match utf8_decoder.next_strict() {
            Some(Ok(s)) => {
                let ch = s.chars().next().expect("expected at least one `char`");
                visitor.visit_char(ch)
            }
            Some(Err(err)) => return Err(err.into()),
            None => return Err(Error::UnexpectedEof.into()),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut string = String::new();
        let mut utf8_decoder = utf8::BufReadDecoder::new(&mut self.reader);
        while let Some(res) = utf8_decoder.next_strict() {
            match res {
                Ok(mut s) => {
                    // The only way for us to know whether `String` or `Vec` deserialization is
                    // complete is to check for a EOF character yielded by the reader.
                    const EOF: char = '\u{0}';
                    const EOF_STR: &'static str = "\u{0}";
                    if s.len() >= EOF.len_utf8() {
                        let eof_start = s.len() - EOF.len_utf8();
                        if &s[eof_start..] == EOF_STR {
                            s = &s[..eof_start];
                        }
                    }
                    string.push_str(s)
                }
                Err(utf8::BufReadDecoderError::Io(err)) => return Err(err.into()),
                Err(utf8::BufReadDecoderError::InvalidByteSequence(_)) => break,
            }
        }
        visitor.visit_string(string)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes = vec![];
        for byte in (&mut self.reader).bytes() {
            bytes.push(byte?);
        }
        visitor.visit_byte_buf(bytes)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.reader.read_u8()? {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(&mut *self),
            b => {
                let msg = format!("expected `0` or `1` for option tag - found {}", b);
                Err(Error::Message(msg))
            }
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct Access<'a, R>
        where
            R: 'a + io::BufRead,
        {
            deserializer: &'a mut Deserializer<R>,
        }

        impl<'de, 'a, R> serde::de::SeqAccess<'de> for Access<'a, R>
        where
            R: io::BufRead,
        {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                match serde::de::DeserializeSeed::deserialize(seed, &mut *self.deserializer) {
                    Ok(v) => Ok(Some(v)),
                    Err(Error::Io(ref err)) if err.kind() == io::ErrorKind::UnexpectedEof => {
                        Ok(None)
                    }
                    Err(err) => Err(err),
                }
            }
        }

        visitor.visit_seq(Access { deserializer: self })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct Access<'a, R>
        where
            R: 'a + io::BufRead,
        {
            deserializer: &'a mut Deserializer<R>,
            len: usize,
        }

        impl<'de, 'a, R> serde::de::SeqAccess<'de> for Access<'a, R>
        where
            R: io::BufRead,
        {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                if self.len == 0 {
                    return Ok(None);
                }
                self.len -= 1;
                let value = serde::de::DeserializeSeed::deserialize(seed, &mut *self.deserializer)?;
                Ok(Some(value))
            }

            fn size_hint(&self) -> Option<usize> {
                Some(self.len)
            }
        }

        visitor.visit_seq(Access {
            deserializer: self,
            len: len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct Access<'a, R>
        where
            R: 'a + io::BufRead,
        {
            deserializer: &'a mut Deserializer<R>,
        }

        impl<'de, 'a, R> serde::de::MapAccess<'de> for Access<'a, R>
        where
            R: io::BufRead,
        {
            type Error = Error;

            fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                match serde::de::DeserializeSeed::deserialize(seed, &mut *self.deserializer) {
                    Ok(v) => Ok(Some(v)),
                    Err(Error::Io(ref err)) if err.kind() == io::ErrorKind::UnexpectedEof => {
                        Ok(None)
                    }
                    Err(err) => Err(err),
                }
            }

            fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                serde::de::DeserializeSeed::deserialize(seed, &mut *self.deserializer)
            }
        }

        visitor.visit_map(Access { deserializer: self })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        impl<'de, 'a, R> serde::de::EnumAccess<'de> for &'a mut Deserializer<R>
        where
            R: io::BufRead,
        {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: serde::de::DeserializeSeed<'de>,
            {
                let idx: u32 = serde::de::Deserialize::deserialize(&mut *self)?;
                let val: Result<_> =
                    seed.deserialize(serde::de::IntoDeserializer::into_deserializer(idx));
                Ok((val?, self))
            }
        }

        impl<'de, 'a, R> serde::de::VariantAccess<'de> for &'a mut Deserializer<R>
        where
            R: io::BufRead,
        {
            type Error = Error;

            fn unit_variant(self) -> Result<()> {
                Ok(())
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                serde::de::DeserializeSeed::deserialize(seed, self)
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
            where
                V: serde::de::Visitor<'de>,
            {
                serde::de::Deserializer::deserialize_tuple(self, len, visitor)
            }

            fn struct_variant<V>(
                self,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value>
            where
                V: serde::de::Visitor<'de>,
            {
                serde::de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
            }
        }

        visitor.visit_enum(self)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Error::DeserializeAnyUnsupported)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Error::DeserializeAnyUnsupported)
    }
}

impl<'a> From<utf8::BufReadDecoderError<'a>> for Error {
    fn from(_err: utf8::BufReadDecoderError) -> Self {
        Error::InvalidUtf8
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::DeserializeAnyUnsupported => "`bytekey` is not a self-describing format",
            Error::UnexpectedEof => "encountered unexpected EOF when deserializing utf8",
            Error::InvalidUtf8 => "attempted to deserialize invalid utf8",
            Error::Io(ref err) => err.description(),
            Error::Message(ref msg) => msg,
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::DeserializeAnyUnsupported => None,
            Error::UnexpectedEof => None,
            Error::InvalidUtf8 => None,
            Error::Io(ref err) => Some(err),
            Error::Message(ref _msg) => None,
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
