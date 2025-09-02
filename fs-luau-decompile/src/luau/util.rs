use fs_lib::buffer::BufferExtension;
use nom::{
    IResult, bytes::complete::take, character::complete::char, multi::count, multi::many_till,
    number::complete::le_u8,
};

use nom_leb128::leb128_usize;

use regex::Regex;

use crate::luau::parser::FunctionParser;

use super::prototype::Prototype;

use anyhow::{Result, anyhow};

use super::DecompileOptions;

pub(crate) fn parse_list<'a, T>(
    input: &'a [u8],
    parser: impl Fn(&'a [u8]) -> IResult<&'a [u8], T>,
) -> IResult<&'a [u8], Vec<T>> {
    let (input, length) = leb128_usize(input)?;
    let (input, items) = count(parser, length)(input)?;
    Ok((input, items))
}

pub(crate) fn parse_list_len<'a, T>(
    input: &'a [u8],
    parser: impl Fn(&'a [u8]) -> IResult<&'a [u8], T>,
    length: usize,
) -> IResult<&'a [u8], Vec<T>> {
    let (input, items) = count(parser, length)(input)?;
    Ok((input, items))
}

pub(crate) fn parse_string(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (input, length) = leb128_usize(input)?;
    let (input, bytes) = take(length)(input)?;
    Ok((input, bytes.to_owned()))
}

fn parse_input(input: &[u8]) -> IResult<&[u8], (usize, Vec<Prototype>, Vec<String>)> {
    let (input, version) = le_u8(input)?;

    if version < 3 && version > 6 {
        panic!("Unsupported bytecode version {}", version);
    }

    let (input, types_version) = if version >= 4 {
        le_u8(input)?
    } else {
        (input, 0)
    };

    if types_version > 3 {
        panic!("Unsupported types version");
    }

    let (input, symbol_table) = parse_list(input, parse_string)?;

    let input = if types_version == 3 {
        many_till(leb128_usize, char('\0'))(input)?.0
    } else {
        input
    };

    let (input, prototypes) = parse_list(input, |i| {
        Prototype::parse_bytecode(i, version, symbol_table.clone())
    })?;
    let (input, main) = leb128_usize(input)?;

    Ok((
        input,
        (main, prototypes, u8_vec_to_string_vec(&symbol_table)),
    ))
}

fn u8_vec_to_string_vec(input: &Vec<Vec<u8>>) -> Vec<String> {
    input
        .iter()
        .map(|entry| match std::str::from_utf8(&entry) {
            Ok(v) => String::from(v),
            _ => String::from("INVALID_UTF8"),
        })
        .collect()
}

fn gen_string_list(list: &Vec<String>, prefix: &str) -> String {
    format!("-- {}: {}\r\n", prefix, list.join(", "))
}

fn gen_symbol_list(list: &Vec<String>, prefix: &str) -> String {
    format!("--[[ {}:\r\n\t{}\r\n]]\r\n", prefix, list.join("\r\n\t"))
}

/// Parse Luau bytecode in order to get needed info.
pub fn parse_bytecode_info(
    bytecode_buffer: &Vec<u8>,
) -> Result<(usize, Vec<Prototype>, Vec<String>)> {
    let (_, (main, prototypes, symbol_table)) =
        parse_input(&bytecode_buffer).map_err(|e| anyhow!(e.to_string()))?;

    Ok((main, prototypes, symbol_table))
}

/// Since we don't implement a complete Lua(u) parser in this project we just
/// want to search for unknown local variables from 0 up to first prototype, and rename
/// using known names in main prototype (which are ordered).
fn rename_nearest_upvalues(
    buffer: &mut Vec<u8>,
    main: usize,
    functions: &Vec<Prototype>,
    nearest_proto_position: usize,
) -> Result<()> {
    if functions.len() == 1 {
        return Ok(());
    }
    let Some(main) = functions.get(main) else {
        return Ok(());
    };

    let buffer_slice = buffer[0..nearest_proto_position].to_vec();

    let re = Regex::new(r"\bv_u_\d+_\b").unwrap();
    let str = buffer_slice.to_vec().to_string()?;
    let locals = main.get_locals();
    let mut i: usize = 0;

    for cap in re.find_iter(&str) {
        let Some(name) = locals.get(i) else { break };

        buffer.find_and_replace_string(&cap.as_str(), &name, 0);

        i = i + 1;
    }

    return Ok(());
}

/// Use parsed Luau bytecode to apply known debug information after decompilation.
pub fn format_luau_buffer(
    buffer: &mut Vec<u8>,
    main: usize,
    prototypes: &Vec<Prototype>,
    symbol_table: &Vec<String>,
    opts: &DecompileOptions,
) -> Result<()> {
    let mut nearest_proto_position = 0;

    for i in 0..prototypes.len() {
        let proto = &prototypes[i];
        let mut parser = FunctionParser::from(buffer, proto)?;

        match &mut parser {
            Some(parser) => {
                parser.rename_parameters(buffer)?;
                parser.rename_self(buffer);

                if nearest_proto_position == 0 {
                    nearest_proto_position = parser.position;
                }
            }
            _ => {}
        };

        if opts.use_variables {
            let local_names = proto.get_locals();
            let locals = gen_string_list(&local_names, "Local values");
            let upvalues = gen_string_list(&proto.upvalues, "Upvalues");

            match parser {
                Some(ref parser) => {
                    if local_names.len() > 0 {
                        buffer.splice(
                            parser.position..parser.position,
                            locals.as_bytes().iter().cloned(),
                        );
                    }
                    if proto.upvalues.len() > 0 {
                        buffer.splice(
                            parser.position..parser.position,
                            upvalues.as_bytes().iter().cloned(),
                        );
                    }
                }
                _ => {
                    if i == main && proto.locals.len() > 0 {
                        buffer.splice(0..0, locals.as_bytes().iter().cloned());
                        nearest_proto_position = nearest_proto_position + locals.len();
                    }
                }
            }
        }

        if opts.use_line_numbers {
            match proto.file_scope {
                Some((line_start, _line_end)) => match parser {
                    Some(ref parser) => {
                        // let comment = format!("-- Line numbers: {} -> {}\n", line_start, line_end);
                        let comment = format!("-- Starts at line {}\n", line_start);

                        buffer.splice(
                            parser.position..parser.position,
                            comment.as_bytes().iter().cloned(),
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    rename_nearest_upvalues(buffer, main, prototypes, nearest_proto_position)?;

    if opts.use_symbol_table && symbol_table.len() > 0 {
        let symbols = gen_symbol_list(&symbol_table, "Symbol table");

        buffer.splice(0..0, symbols.as_bytes().iter().cloned());
    }

    Ok(())
}

pub fn find_position_of_function(buffer: &Vec<u8>, name: &str, offset: usize) -> Option<usize> {
    let str = format!("function \\s*([A-z0-9.]+)?{}\\(", name);

    let input = match std::str::from_utf8(&buffer) {
        Ok(v) => String::from(v),
        _ => {
            println!("utf8 error");
            return None;
        }
    };

    let Ok(re) = Regex::new(&str) else {
        return None;
    };

    let Some(captures) = re.captures_at(&input, offset) else {
        return None;
    };

    if captures.len() < 1 {
        return None;
    }

    Some(captures.get(0).unwrap().start())
}
