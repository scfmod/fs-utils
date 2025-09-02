use nom::{
    IResult,
    number::complete::{le_f32, le_f64, le_u8, le_u32},
};

use nom_leb128::leb128_usize;

use super::util::parse_list;

const CONSTANT_NIL: u8 = 0;
const CONSTANT_BOOLEAN: u8 = 1;
const CONSTANT_NUMBER: u8 = 2;
const CONSTANT_STRING: u8 = 3;
const CONSTANT_IMPORT: u8 = 4;
const CONSTANT_TABLE: u8 = 5;
const CONSTANT_CLOSURE: u8 = 6;
const CONSTANT_VECTOR: u8 = 7;

#[allow(dead_code)]
pub enum Constant {
    Nil,
    Boolean(bool),
    Number(f64),
    String(usize),
    Import(usize),
    Table(Vec<usize>),
    Closure(usize),
    Vector(f32, f32, f32, f32),
}

impl Constant {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, tag) = le_u8(input)?;
        match tag {
            CONSTANT_NIL => Ok((input, Constant::Nil)),
            CONSTANT_BOOLEAN => {
                let (input, value) = le_u8(input)?;
                Ok((input, Constant::Boolean(value != 0u8)))
            }
            CONSTANT_NUMBER => {
                let (input, value) = le_f64(input)?;
                Ok((input, Constant::Number(value)))
            }
            CONSTANT_STRING => {
                let (input, string_index) = leb128_usize(input)?;
                Ok((input, Constant::String(string_index)))
            }
            CONSTANT_IMPORT => {
                let (input, import_index) = le_u32(input)?;
                Ok((input, Constant::Import(import_index as usize)))
            }
            CONSTANT_TABLE => {
                let (input, keys) = parse_list(input, leb128_usize)?;
                Ok((input, Constant::Table(keys)))
            }
            CONSTANT_CLOSURE => {
                let (input, f_id) = leb128_usize(input)?;
                Ok((input, Constant::Closure(f_id)))
            }
            CONSTANT_VECTOR => {
                let (input, x) = le_f32(input)?;
                let (input, y) = le_f32(input)?;
                let (input, z) = le_f32(input)?;
                let (input, w) = le_f32(input)?;
                Ok((input, Constant::Vector(x, y, z, w)))
            }
            _ => panic!("Unknown constant tag: {}", tag),
        }
    }
}
