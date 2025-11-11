use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{buffer::BufferExtension, list_files_with_extension, path::PathExtension};
use xml::ParserConfig;
use xml::writer::EmitterConfig;

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

    /// disable escape characters in attributes
    #[argh(switch, short = 'e')]
    disable_escape_characters: bool,

    /// path to input file/folder
    #[argh(positional)]
    input: PathBuf,

    /// path to output file/folder (optional)
    #[argh(positional)]
    output: Option<PathBuf>,
}

fn create_indent_string(indent_type: &Indent, indent_size: u8) -> String {
    let indent_char = match indent_type {
        Indent::Space => ' ',
        Indent::Tab => '\t',
    };

    indent_char.to_string().repeat(indent_size as usize)
}

fn format_xml_file<P: AsRef<Path>>(
    file: P,
    output_file: P,
    indent_char: &Indent,
    indent_size: u8,
    escape_characters: bool,
) -> Result<()> {
    let buffer: Vec<u8> = Vec::read_from_file(&file)?;
    let input: &[u8] = &buffer;

    let mut reader = ParserConfig::default()
        .ignore_root_level_whitespace(true)
        .ignore_comments(false)
        .cdata_to_characters(false)
        .coalesce_characters(false)
        .create_reader(input);

    let output = File::create(output_file)?;
    let writer = BufWriter::new(output);

    let mut config = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(create_indent_string(&indent_char, indent_size))
        .write_document_declaration(true);

    config.perform_escaping = escape_characters;

    let mut emitter = config.create_writer(writer);

    loop {
        let reader_event = reader.next()?;

        match reader_event {
            xml::reader::XmlEvent::EndDocument => break,
            xml::reader::XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                let event = xml::writer::XmlEvent::StartElement {
                    name: name.borrow(),
                    namespace: namespace.borrow(),
                    attributes: attributes.iter().map(|attr| attr.borrow()).collect(),
                };
                emitter.write(event)?;
            }
            xml::reader::XmlEvent::Characters(text) => {
                let event = xml::writer::XmlEvent::Characters(&text);
                emitter.write(event)?;
            }
            xml::reader::XmlEvent::Comment(text) => {
                let event = xml::writer::XmlEvent::Comment(&text);
                emitter.write(event)?;
            }
            other => {
                if let Some(writer_event) = other.as_writer_event() {
                    emitter.write(writer_event)?;
                }
            }
        }
    }

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

            format_xml_file(
                &file,
                &&output_file,
                &cli.indent_char,
                cli.indent_size,
                !cli.disable_escape_characters,
            )?;
        }
    } else {
        let output: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        format_xml_file(
            &cli.input,
            &output,
            &cli.indent_char,
            cli.indent_size,
            !cli.disable_escape_characters,
        )?;

        if !cli.silent {
            println!("{}", output.display());
        }
    }

    Ok(())
}
