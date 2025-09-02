use nom::{
    IResult,
    number::complete::{le_u8, le_u32},
};

use nom_leb128::leb128_usize;

use super::constant::Constant;

use super::util::{parse_list, parse_list_len};

#[derive(Debug)]
pub struct Prototype {
    pub name: Option<String>,
    pub locals: Vec<Local>,
    pub upvalues: Vec<String>,
    pub file_scope: Option<(usize, usize)>,
}

impl Prototype {
    pub fn get_locals(&self) -> Vec<String> {
        let mut list: Vec<&Local> = self
            .locals
            .iter()
            .filter(|local| local.scope_start > 0)
            .collect();

        list.sort_by(|a, b| a.scope_start.cmp(&b.scope_start));

        list.iter().map(|local| local.name.clone()).collect()
    }

    pub fn get_parameters(&self) -> Vec<String> {
        let mut list: Vec<&Local> = self
            .locals
            .iter()
            .filter(|local| local.scope_start == 0)
            .collect();

        list.sort_by(|a, b| a.register.cmp(&b.register));

        list.iter().map(|local| local.name.clone()).collect()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Local {
    pub name: String,
    pub scope_start: usize,
    pub scope_end: usize,
    pub register: u8,
}

impl Prototype {
    pub fn parse_bytecode(
        input: &[u8],
        version: u8,
        symbol_table: Vec<Vec<u8>>,
    ) -> IResult<&[u8], Self> {
        let (input, _max_stack_size) = le_u8(input)?;
        let (input, _num_params) = le_u8(input)?;
        let (input, _num_upvalues) = le_u8(input)?;
        let (input, _is_vararg) = le_u8(input)?;

        let input = if version >= 4 {
            let (input, _flags) = le_u8(input)?;
            let (input, _) = parse_list(input, le_u8)?;

            input
        } else {
            input
        };

        let (input, instructions) = parse_list(input, le_u32)?;
        let (input, _constants) = parse_list(input, Constant::parse)?;
        let (input, _functions) = parse_list(input, leb128_usize)?;
        let (input, _line_defined) = leb128_usize(input)?;
        let (input, prototype_name_index) = leb128_usize(input)?;

        let prototype_name = if prototype_name_index > 0 {
            match std::str::from_utf8(&symbol_table[prototype_name_index - 1]) {
                Ok(str) => Some(String::from(str)),
                _ => None,
            }
        } else {
            None
        };

        let mut file_scope: Option<(usize, usize)> = None;
        let mut file_scope_start: usize = 0;
        let /*mut*/ file_scope_lines: usize = 0;

        let (input, has_line_info) = le_u8(input)?;

        let (input, line_gap_log2) = match has_line_info {
            0 => (input, None),
            _ => {
                let (input, line_gap_log2) = le_u8(input)?;
                (input, Some(line_gap_log2))
            }
        };

        let (input, _line_info_delta) = match has_line_info {
            0 => (input, None),
            _ => {
                let (input, line_info_delta) = parse_list_len(input, le_u8, instructions.len())?;

                // TODO
                // There's something missing, the sum is sometimes way too high.
                // Also needs to take child prototypes into consideration I guess?

                // for value in line_info_delta.iter() {
                //     file_scope_lines = file_scope_lines + (*value as usize);
                // }

                (input, Some(line_info_delta))
            }
        };

        let (input, _abs_line_info_delta) = match has_line_info {
            0 => (input, None),
            _ => {
                let intervals = ((instructions.len() - 1) >> line_gap_log2.unwrap()) + 1;
                let (input, abs_line_info_delta) = parse_list_len(input, le_u32, intervals)?;

                if abs_line_info_delta.len() > 0 {
                    file_scope_start = *abs_line_info_delta.last().unwrap() as usize;
                }

                (input, Some(abs_line_info_delta))
            }
        };

        let (input, debug_info) = le_u8(input)?;

        let (mut input, num_locals) = if debug_info == 0 {
            (input, 0)
        } else {
            leb128_usize(input)?
        };

        let mut index: usize;
        let mut locals: Vec<Local> = Vec::with_capacity(num_locals);

        let mut scope_start: usize;
        let mut scope_end: usize;
        let mut register: u8;

        if num_locals > 0 {
            for _ in 0..num_locals {
                (input, index) = leb128_usize(input)?;

                let name = match index {
                    0 => None,
                    _ => Some(std::str::from_utf8(&symbol_table[index - 1]).unwrap_or("NOT_FOUND")),
                };

                (input, scope_start) = leb128_usize(input)?; // scope start
                (input, scope_end) = leb128_usize(input)?; // scope end
                (input, register) = le_u8(input)?; // register

                if name.is_some() {
                    locals.push(Local {
                        name: name.unwrap().to_string(),
                        scope_start,
                        scope_end,
                        register,
                    });
                }
            }
        }

        let (mut input, num_upvalues) = if debug_info == 0 {
            (input, 0)
        } else {
            leb128_usize(input)?
        };

        let mut upvalues: Vec<String> = Vec::with_capacity(num_upvalues);

        if num_upvalues > 0 {
            for _ in 0..num_upvalues {
                (input, index) = leb128_usize(input)?;

                if index > 0 {
                    let name = std::str::from_utf8(&symbol_table[index - 1]).unwrap_or("NOT_FOUND");

                    upvalues.push(name.to_string());
                }
            }
        }

        if prototype_name.is_some() {
            file_scope = Some((file_scope_start, file_scope_start + file_scope_lines + 1));
        }

        Ok((input, Prototype {
            name: prototype_name,
            locals,
            upvalues,
            file_scope,
        }))
    }
}
