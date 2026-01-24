#[cfg(windows)]
mod process;

#[cfg(windows)]
mod windows {
    use std::{path::PathBuf, process::Command};

    use anyhow::{Result, bail};
    use argh::FromArgs;
    use fs_lib::{EXECUTABLE_PATTERNS, Platform, buffer::BufferExtension};

    use crate::process::{get_process_modules, open_process, resume_process, suspend_process};

    #[derive(FromArgs, PartialEq, Debug)]
    /// Launch executable as child process and patch memory
    pub struct Cmd {
        /// path to executable
        #[argh(positional)]
        input: PathBuf,

        /// platform: steam, giants (default: steam)
        #[argh(option, default = "Platform::Steam")]
        platform: Platform,

        /// use PID of already running process instead
        #[argh(option)]
        pid: Option<u32>,

        /// stop process instead of resuming
        #[argh(switch)]
        test: bool,

        /// don't wait for child process to exit
        #[argh(switch)]
        no_wait: bool,
    }

    pub fn run() -> Result<()> {
        let cli: Cmd = argh::from_env();

        let Some(items) = EXECUTABLE_PATTERNS.get(&cli.platform) else {
            bail!("No patch items found")
        };

        let (pid, child_process) = match cli.pid {
            Some(id) => (id, None),
            _ => {
                let child_process = Command::new(&cli.input).spawn()?;

                println!("Started new child ProcessID: {}", child_process.id());

                (child_process.id(), Some(child_process))
            }
        };

        if !suspend_process(pid) {
            bail!("Failed to suspend child process")
        }

        println!("Suspended process");

        let process_handle = open_process(pid)?;
        let modules = get_process_modules(&process_handle, pid)?;

        println!("Found {} process modules", modules.len());

        let file_name = cli
            .input
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let Some(module) = modules.iter().find(|m| m.name == file_name) else {
            bail!("Unable to locate main process module")
        };

        println!("Module name: {}", module.name);
        println!("Module base_addr: {}", module.base_addr);
        println!("Module base_size: {}", module.base_size);

        let (module_buffer, bytes_read) = module.to_buffer(&process_handle)?;

        println!("{} bytes read from module memory into buffer", bytes_read);

        for item in items {
            if let Some(offset) = module_buffer.find_bytes(&item.find) {
                println!("Applying {:?} at offset {}", item.patch_type, offset);

                let bytes_written = module.replace_bytes(&item.replace, offset, &process_handle)?;

                println!("Bytes written: {}", bytes_written)
            }
        }

        println!("Resuming process");

        if !resume_process(pid) {
            bail!("Failed to resume process!")
        }

        if let Some(mut child_process) = child_process {
            if cli.test {
                println!("Klling child process");
                let _ = child_process.kill();
            } else if !cli.no_wait {
                println!("Waiting for child process to exit");

                let _ = child_process.wait();
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    windows::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("fs-patch-process is only supported on Windows");
    std::process::exit(1);
}
