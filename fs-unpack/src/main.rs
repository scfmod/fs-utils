use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::buffer::BufferExtension;
use gar_lib::GarArchive;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(FromArgs, PartialEq, Debug)]
/// Extract .gar/.dlc archive
struct Cmd {
    /// silent mode
    #[argh(switch, short = 's')]
    silent: bool,

    /// path to .gar/.dlc archive
    #[argh(positional)]
    input: PathBuf,

    /// output path
    #[argh(positional)]
    output_path: PathBuf,
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    let archive = GarArchive::open(&cli.input)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let file_count = archive.len();

    if file_count == 0 {
        bail!("No files found in archive");
    }

    if !cli.silent {
        println!("Extracting {} files from {}", file_count, cli.input.display());
    }

    let files: Vec<_> = archive.files().collect();
    let extracted = AtomicUsize::new(0);

    files.into_par_iter().try_for_each(|file_name| -> Result<()> {
        let data = archive.read_file(file_name)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let file_path: PathBuf = cli.output_path.join(file_name).components().collect();

        data.write_to_file(&file_path)?;

        let count = extracted.fetch_add(1, Ordering::Relaxed) + 1;
        if !cli.silent && count % 500 == 0 {
            println!("Extracted {}/{} files...", count, file_count);
        }

        Ok(())
    })?;

    if !cli.silent {
        println!("Done. Extracted {} files.", file_count);
    }

    Ok(())
}
