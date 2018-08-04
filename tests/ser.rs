extern crate bytekey;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use bytekey::serialize;
use rand::distributions::{Distribution, Standard};
use rand::{random, Rng};
use serde::Serialize;
use std::{f32, f64, i16, i64, i8, isize, u16, u64, u8, usize};

#[derive(PartialEq, PartialOrd, Serialize)]
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

#[derive(PartialEq, PartialOrd, Serialize)]
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

// 1. Generate some random values of type `T`.
// 2. Collect them into two `Vec`s - one with values, one with the serialized bytes.
// 3. Sort both `Vec`s.
// 4. Assert that the order of both `Vec`s is identical.
fn assert_order_preservation_of_n_elems<T>(n: usize)
where
    T: PartialOrd + Serialize,
    Standard: Distribution<T>,
{
    let mut values = Vec::with_capacity(n);
    let mut bytes = Vec::with_capacity(n);
    for i in 0..n {
        let val: T = rand::random();
        let serialized = serialize(&val).unwrap();
        values.push((val, i));
        bytes.push((serialized, i));
    }
    values.sort_by(|(a, _), (b, _)| a.partial_cmp(&b).unwrap());
    bytes.sort_by(|(a, _), (b, _)| a.partial_cmp(&b).unwrap());
    for ((_, value_i), (_, bytes_i)) in values.into_iter().zip(bytes) {
        assert_eq!(value_i, bytes_i);
    }
}

#[test]
fn test_u8() {
    assert_order_preservation_of_n_elems::<u8>(256);
}

#[test]
fn test_u16() {
    assert_order_preservation_of_n_elems::<u16>(1024);
}

#[test]
fn test_u32() {
    assert_order_preservation_of_n_elems::<u32>(1024);
}

#[test]
fn test_u64() {
    assert_order_preservation_of_n_elems::<u64>(1024);
}

#[test]
fn test_usize() {
    assert_order_preservation_of_n_elems::<usize>(1024);
}

#[test]
fn test_i8() {
    assert_order_preservation_of_n_elems::<i8>(256);
}

#[test]
fn test_i16() {
    assert_order_preservation_of_n_elems::<i16>(1024);
}

#[test]
fn test_i32() {
    assert_order_preservation_of_n_elems::<i32>(1024);
}

#[test]
fn test_i64() {
    assert_order_preservation_of_n_elems::<i64>(1024);
}

#[test]
fn test_isize() {
    assert_order_preservation_of_n_elems::<isize>(1024);
}

#[test]
fn test_f32() {
    assert_order_preservation_of_n_elems::<i32>(1024);
}

#[test]
fn test_f64() {
    assert_order_preservation_of_n_elems::<i64>(1024);
}

#[test]
fn test_bool() {
    assert_order_preservation_of_n_elems::<bool>(16);
}

#[test]
fn test_struct() {
    assert_order_preservation_of_n_elems::<TestStruct>(1024);
}

#[test]
fn test_enum() {
    assert_order_preservation_of_n_elems::<TestEnum>(1024);
}

#[test]
fn test_usize_as_u64() {
    assert_eq!(
        serialize(&usize::MAX).unwrap(),
        serialize(&u64::MAX).unwrap()
    );
    assert!(serialize(&usize::MIN).unwrap() < serialize(&usize::MAX).unwrap());
}

#[test]
fn test_isize_as_i64() {
    assert_eq!(
        serialize(&isize::MAX).unwrap(),
        serialize(&i64::MAX).unwrap()
    );
    assert!(serialize(&isize::MIN).unwrap() < serialize(&isize::MAX).unwrap());
}

#[test]
fn test_f32_special_values() {
    assert!(serialize(&f32::MAX).unwrap() < serialize(&f32::NAN).unwrap());
    assert!(serialize(&-1.0f32).unwrap() < serialize(&-0.9f32).unwrap());
    assert!(serialize(&-10_000.0f32).unwrap() < serialize(&10_000.0f32).unwrap());
    assert!(serialize(&9_000.0f32).unwrap() < serialize(&10_000.0f32).unwrap());
    assert!(serialize(&9_000.0f32).unwrap() < serialize(&10_000.0f32).unwrap());
    assert!(serialize(&0.0f32).unwrap() < serialize(&1.0f32).unwrap());
}

#[test]
fn test_f64_special_values() {
    assert!(serialize(&f64::MAX).unwrap() < serialize(&f64::NAN).unwrap());
    assert!(serialize(&-1.0f64).unwrap() < serialize(&-0.9f64).unwrap());
    assert!(serialize(&-10_000.0f64).unwrap() < serialize(&10_000.0f64).unwrap());
    assert!(serialize(&9_000.0f64).unwrap() < serialize(&10_000.0f64).unwrap());
    assert!(serialize(&9_000.0f64).unwrap() < serialize(&10_000.0f64).unwrap());
    assert!(serialize(&0.0f64).unwrap() < serialize(&1.0f64).unwrap());
}
