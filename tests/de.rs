extern crate bytekey;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use bytekey::{deserialize, serialize};
use rand::distributions::{Distribution, Standard};
use rand::{random, Rng};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{f32, f64, i16, i64, i8, isize, u16, u64, u8, usize};

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
struct TestStruct {
    u8_: u8,
    u16_: u16,
    u32_: u32,
    u64_: u64,
    i8_: i8,
    i16_: i16,
    i32_: i32,
    i64_: i64,
    f32_: f32,
    f64_: f64,
    bool_: bool,
    char_: char,
}

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
enum TestEnum {
    A(u32, String),
    B,
    C(isize),
}

// `rand` implemetations.

impl Distribution<TestStruct> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestStruct {
        TestStruct {
            u8_: rng.gen(),
            u16_: rng.gen(),
            u32_: rng.gen(),
            u64_: rng.gen(),
            i8_: rng.gen(),
            i16_: rng.gen(),
            i32_: rng.gen(),
            i64_: rng.gen(),
            f32_: rng.gen(),
            f64_: rng.gen(),
            bool_: rng.gen(),
            char_: rng.gen(),
        }
    }
}

impl Distribution<TestEnum> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestEnum {
        match rng.gen_range(0, 3) {
            0 => {
                let len = (rng.gen::<f32>() * 10.0) as usize;
                let string = (0..len).map(|_| random::<char>()).collect();
                TestEnum::A(rng.gen(), string)
            }
            1 => TestEnum::B,
            2 => TestEnum::C(rng.gen()),
            _ => unreachable!(),
        }
    }
}

fn assert_serde_equality_n_times<T>(n: usize)
where
    T: Debug + PartialEq + for<'de> Deserialize<'de> + Serialize,
    Standard: Distribution<T>,
{
    for _ in 0..n {
        let val: T = random();
        let bytes = serialize(&val).unwrap();
        assert_eq!(val, deserialize(&bytes).unwrap());
    }
}

#[test]
fn test_u8() {
    assert_serde_equality_n_times::<u8>(256);
}

#[test]
fn test_u16() {
    assert_serde_equality_n_times::<u16>(1024);
}

#[test]
fn test_u32() {
    assert_serde_equality_n_times::<u32>(1024);
}

#[test]
fn test_u64() {
    assert_serde_equality_n_times::<u64>(1024);
}

#[test]
fn test_usize() {
    assert_serde_equality_n_times::<usize>(1024);
}

#[test]
fn test_i8() {
    assert_serde_equality_n_times::<i8>(256);
}

#[test]
fn test_i16() {
    assert_serde_equality_n_times::<i16>(1024);
}

#[test]
fn test_i32() {
    assert_serde_equality_n_times::<i32>(1024);
}

#[test]
fn test_i64() {
    assert_serde_equality_n_times::<i64>(1024);
}

#[test]
fn test_isize() {
    assert_serde_equality_n_times::<isize>(1024);
}

#[test]
fn test_f32() {
    assert_serde_equality_n_times::<f32>(1024);
}

#[test]
fn test_f64() {
    assert_serde_equality_n_times::<f64>(1024);
}

#[test]
fn test_char() {
    assert_serde_equality_n_times::<char>(1024);
}

#[test]
fn test_bool() {
    assert_serde_equality_n_times::<bool>(16);
}

#[test]
fn test_option() {
    assert_serde_equality_n_times::<Option<u8>>(1024);
    assert_serde_equality_n_times::<Option<f32>>(1024);
    assert_serde_equality_n_times::<Option<usize>>(1024);
}

#[test]
fn test_tuples() {
    assert_serde_equality_n_times::<(u8, u8)>(1024);
    assert_serde_equality_n_times::<(i8, u8, u32)>(1024);
    assert_serde_equality_n_times::<(u8, f32, i32, char)>(1024);
}

#[test]
fn test_string() {
    fn random_string() -> String {
        let len = (random::<f32>() * 10.0) as usize;
        (0..len).map(|_| random::<char>()).collect::<String>()
    }

    for _ in 0..1024 {
        let string = random_string();
        let bytes = serialize(&string).unwrap();
        let result: String = deserialize(&bytes).unwrap();
        assert_eq!(string, result);
    }
}

#[test]
fn test_vec() {
    for _ in 0..1024 {
        let len = (random::<f32>() * 10.0) as usize;
        let vec = (0..len).map(|_| random::<u8>()).collect::<Vec<_>>();
        let serialized = serialize(&vec).unwrap();
        let result: Vec<u8> = deserialize(&serialized).unwrap();
        assert_eq!(vec, result);
    }
}

#[test]
fn test_array() {
    assert_serde_equality_n_times::<[u8; 2]>(1024);
    assert_serde_equality_n_times::<[f32; 16]>(1024);
    assert_serde_equality_n_times::<[usize; 32]>(1024);
    assert_serde_equality_n_times::<[char; 1]>(1024);
}

#[test]
fn test_struct() {
    assert_serde_equality_n_times::<TestStruct>(1024);
}

#[test]
fn test_enum() {
    assert_serde_equality_n_times::<TestEnum>(1024);
}

#[test]
fn test_usize_2() {
    let values = vec![
        0usize,
        2usize.pow(0),
        2usize.pow(4) - 1,
        2usize.pow(4),
        2usize.pow(12) - 1,
        2usize.pow(12),
        2usize.pow(20) - 1,
        2usize.pow(20),
        2usize.pow(28) - 1,
        2usize.pow(28),
        2usize.pow(36) - 1,
        2usize.pow(36),
        2usize.pow(44) - 1,
        2usize.pow(44),
        2usize.pow(52) - 1,
        2usize.pow(52),
        2usize.pow(60) - 1,
        2usize.pow(60),
        usize::MAX,
    ];
    for val in values {
        assert_eq!(val, deserialize(&serialize(&val).unwrap()).unwrap());
    }
}

#[test]
fn test_isize_2() {
    let values = vec![
        -2isize.pow(0),
        0isize,
        2isize.pow(0),
        -2isize.pow(3) - 1,
        -2isize.pow(3),
        2isize.pow(3) - 1,
        2isize.pow(3),
        -2isize.pow(11) - 1,
        -2isize.pow(11),
        2isize.pow(11) - 1,
        2isize.pow(11),
        -2isize.pow(19) - 1,
        -2isize.pow(19),
        2isize.pow(19) - 1,
        2isize.pow(19),
        -2isize.pow(27) - 1,
        -2isize.pow(27),
        2isize.pow(27) - 1,
        2isize.pow(27),
        -2isize.pow(35) - 1,
        -2isize.pow(35),
        2isize.pow(35) - 1,
        2isize.pow(35),
        -2isize.pow(43) - 1,
        -2isize.pow(43),
        2isize.pow(43) - 1,
        2isize.pow(43),
        -2isize.pow(51) - 1,
        -2isize.pow(51),
        2isize.pow(51) - 1,
        2isize.pow(51),
        -2isize.pow(59) - 1,
        -2isize.pow(59),
        2isize.pow(59) - 1,
        2isize.pow(59),
        isize::MIN,
        isize::MAX,
    ];
    for val in values {
        assert_eq!(val, deserialize(&serialize(&val).unwrap()).unwrap());
    }
}

#[test]
fn test_f32_2() {
    let values = vec![
        f32::NEG_INFINITY,
        f32::MIN,
        -0.0,
        0.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
    ];
    for val in values {
        assert_eq!(val, deserialize(&serialize(&val).unwrap()).unwrap());
    }
}

#[test]
fn test_f64_2() {
    let values = vec![
        f64::NEG_INFINITY,
        f64::MIN,
        -0.0,
        0.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::INFINITY,
    ];
    for val in values {
        assert_eq!(val, deserialize(&serialize(&val).unwrap()).unwrap());
    }
}
