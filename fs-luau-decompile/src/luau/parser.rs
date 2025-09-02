use fs_lib::buffer::BufferExtension;
use regex::Regex;

use crate::luau::prototype::Prototype;
use crate::luau::util::find_position_of_function;

use anyhow::Result;

#[allow(dead_code)]
pub struct FunctionParser {
    pub name: String,
    pub position: usize,
    pub initial_size: usize,
    pub parameters: Vec<String>,
    pub definition: String,
    pub definition_parameters: Vec<String>,
    pub definition_parameters_str: String,
    pub has_self: bool,
}

impl FunctionParser {
    /// Rename function parameters in decompiled source file, using
    /// debug info from the original bytecode file.
    pub fn rename_parameters(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        let search_signature = format!("({})", &self.definition_parameters_str);
        let search_replace = format!("({})", &self.parameters.join(", "));

        // Replace source parameter string with our new string.
        buffer.find_and_replace_string(&search_signature, &search_replace, 0);

        // Create a vector with tuple source and target parameter names.
        let mapped_parameters: Vec<(&str, &String)> = self
            .definition_parameters
            .iter()
            .map(|v| v.trim())
            .enumerate()
            .map(|(i, v)| (v, self.parameters.get(i).clone().unwrap()))
            .collect();

        for (definition_name, name) in mapped_parameters.iter() {
            // If we find a "self" parameter name we want to register it for further use.
            if *definition_name == "self" || *name == "self" {
                self.has_self = true;
            }

            // We don't want to replace anything if the definition name is an underscore,
            // which means the variable is not referenced anywhere.
            if *definition_name != "_" {
                buffer.find_and_replace_string(&definition_name, &name, self.position);
            }
        }

        Ok(())
    }

    /// If function is a table (class) method with self parameter,
    /// remove self and convert to method signature with ":" notation.
    /// Example:
    ///   function MyClass.doSomething(self, x, y, z)
    /// ->
    ///   function MyClass:doSomething(x, y, z)
    pub fn rename_self(&mut self, buffer: &mut Vec<u8>) {
        if !self.has_self {
            return;
        }

        if let Some(function_path) = self.get_class_path() {
            let (search, replace) = if self.definition_parameters.len() == 1 {
                (
                    format!("function {}(self)", function_path),
                    format!("function {}()", function_path.replace(".", ":")),
                )
            } else {
                (
                    format!("function {}(self, ", function_path),
                    format!("function {}(", function_path.replace(".", ":")),
                )
            };

            buffer.find_and_replace_string(&search, &replace, self.position);
        }
    }

    /// Returns full name of function if it's a part of a table (class) method.
    pub fn get_class_path(&self) -> Option<&str> {
        let re = Regex::new(r"\b(\w+\.\w+)\(").unwrap();

        re.captures(&self.definition)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
    }

    /// Constructor function.
    pub fn from(buffer: &mut Vec<u8>, proto: &Prototype) -> Result<Option<Self>> {
        let parameters = proto.get_parameters();

        if proto.name.is_none() || parameters.len() == 0 {
            return Ok(None);
        }

        let name = proto.name.as_ref().unwrap().clone();

        let Some(mut position) = find_position_of_function(&buffer, &name, 0) else {
            return Ok(None);
        };

        let Some(initial_size) = buffer.find_bytes_from(&vec![0x0A], position) else {
            return Ok(None);
        };

        let slice_buffer = &buffer[position..(position + initial_size)];
        let definition = slice_buffer.to_vec().to_string()?;

        let re = Regex::new(r"\((.*?)\)").unwrap();

        let Some((definition_parameters_str, definition_parameters)): Option<(
            String,
            Vec<String>,
        )> = re
            .captures(&definition)
            .and_then(|cap| cap.get(1))
            .map(|matched| matched.as_str().to_string())
            .and_then(|value| {
                Some((
                    value.clone(),
                    value.split(",").map(|s| s.to_string()).collect(),
                ))
            })
        else {
            return Ok(None);
        };

        if definition_parameters_str.len() == 0 || definition_parameters.len() != parameters.len() {
            return Ok(None);
        }

        // Add a newline before function definition
        buffer.splice(position..position, [0x0d_u8, 0x0a_u8]);
        position = position + 2;

        Ok(Some(Self {
            name,
            parameters,
            position,
            initial_size,
            definition,
            definition_parameters,
            definition_parameters_str,
            has_self: false,
        }))
    }
}
