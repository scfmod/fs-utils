use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Result, bail};
use argh::FromArgs;
use biodivine_xml_doc::{Document, WriteOptions};
use fs_lib::{list_files_with_extension, path::PathExtension};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Indent {
    Space = 0x20,
    Tab = 0x09,
}

impl FromStr for Indent {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "space" => Ok(Indent::Space),
            "tab" => Ok(Indent::Tab),
            _ => Err(format!("Unknown indent character enum: {}", s)),
        }
    }
}

#[derive(FromArgs, PartialEq, Debug)]
/// Parse XML and output sane formatted XML.
pub struct Cmd {
    /// recursive mode if folder input
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// suppress output
    #[argh(switch, short = 's')]
    silent: bool,

    /// indent character (space,tab)
    #[argh(option, short = 'c', default = "Indent::Space")]
    indent_char: Indent,

    /// indent size
    #[argh(option, short = 'i', default = "4")]
    indent_size: u8,

    /// path to input file/folder
    #[argh(positional)]
    input: PathBuf,

    /// path to output file/folder (optional)
    #[argh(positional)]
    output: Option<PathBuf>,
}

fn format_xml_file<P: AsRef<Path>>(
    file: P,
    output_file: P,
    indent_char: &Indent,
    indent_size: u8,
) -> Result<()> {
    let doc = Document::parse_file(file)?;
    let mut path = output_file.as_ref().to_path_buf();

    path.pop();

    if !path.exists() {
        create_dir_all(path)?;
    }

    let opts = WriteOptions {
        indent_char: *indent_char as u8,
        indent_size: indent_size as usize,
        write_decl: true,
    };

    doc.write_file_with_opts(output_file, opts)?;

    Ok(())
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.input.is_dir() {
        let output_path = cli.output.unwrap_or_else(|| cli.input.clone());

        if output_path.is_file() {
            bail!("Output path is a file")
        }

        let files = list_files_with_extension(&cli.input, r"xml", cli.recursive)?;

        for file in files.iter() {
            let output_file: PathBuf = file
                .convert_relative_path(&cli.input, &output_path)?
                .components()
                .collect();

            if !cli.silent {
                if output_file != *file {
                    println!("{} -> {}", file.display(), output_file.display());
                } else {
                    println!("{}", file.display());
                }
            }

            format_xml_file(&file, &&output_file, &cli.indent_char, cli.indent_size)?;
        }
    } else {
        let output: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        format_xml_file(&cli.input, &output, &cli.indent_char, cli.indent_size)?;

        if !cli.silent {
            println!("{}", output.display());
        }
    }

    Ok(())
}
