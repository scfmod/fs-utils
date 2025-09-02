use anyhow::Result;
use winapi::{
    ctypes::c_void,
    shared::{
        minwindef::FALSE,
        ntdef::{HANDLE, NULL},
    },
    um::{
        handleapi::CloseHandle,
        memoryapi::{ReadProcessMemory, WriteProcessMemory},
        processthreadsapi::{OpenProcess, OpenThread, ResumeThread, SuspendThread},
        psapi::EnumProcessModules,
        tlhelp32::{
            CreateToolhelp32Snapshot, MODULEENTRY32, Module32First, Module32Next,
            TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPTHREAD, THREADENTRY32,
            Thread32First, Thread32Next,
        },
        winnt::{PROCESS_ALL_ACCESS, THREAD_SUSPEND_RESUME},
    },
};

pub trait FromCString {
    fn from_cstring(arr: &[i8]) -> String;
}

impl FromCString for String {
    fn from_cstring(arr: &[i8]) -> String {
        convert_c_string_lossy(arr)
    }
}

pub struct Module {
    pub name: String,
    pub base_addr: usize,
    pub base_size: u32,
}

impl Module {
    pub fn from_entry(entry: &MODULEENTRY32) -> Self {
        let name = String::from_cstring(&entry.szModule);

        Self {
            name,
            base_addr: entry.modBaseAddr as usize,
            base_size: entry.modBaseSize,
        }
    }

    pub fn to_buffer(&self, handle: &Handle) -> Result<(Vec<u8>, usize)> {
        read_process_memory(handle, self.base_addr, self.base_size as usize)
    }

    pub fn replace_bytes(&self, bytes: &Vec<u8>, offset: usize, handle: &Handle) -> Result<usize> {
        write_process_memory(handle, self.base_addr + offset, &bytes)
    }
}
pub struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        close_handle(&self);
    }
}

pub fn convert_c_string_lossy(arr: &[i8]) -> String {
    // Find the null terminator (0)
    let len = arr.iter().position(|&c| c == 0).unwrap_or(arr.len());

    // Convert to &[u8] using a raw pointer cast
    let u8_slice = unsafe { std::slice::from_raw_parts(arr.as_ptr() as *const u8, len) };

    // Create a String, lossy conversion handles invalid UTF-8
    String::from_utf8_lossy(u8_slice).into_owned()
}

pub fn suspend_process(id: u32) -> bool {
    unsafe {
        let mut has_err = false;

        let te: &mut THREADENTRY32 = &mut std::mem::zeroed();
        (*te).dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

        let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);

        if Thread32First(snapshot, te) == 1 {
            loop {
                if id == (*te).th32OwnerProcessID {
                    let tid = (*te).th32ThreadID;

                    let thread: HANDLE = OpenThread(THREAD_SUSPEND_RESUME, FALSE, tid);
                    has_err |= SuspendThread(thread) as i32 == -1i32;

                    CloseHandle(thread);
                }

                if Thread32Next(snapshot, te) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        !has_err
    }
}

pub fn open_process(id: u32) -> Result<Handle> {
    let handle: HANDLE = unsafe { OpenProcess(PROCESS_ALL_ACCESS, FALSE, id) };

    if handle == NULL {
        anyhow::bail!(
            "OpenProcess failed, error: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(Handle(handle))
}

pub fn close_handle(handle: &Handle) -> bool {
    let result = unsafe { CloseHandle(handle.0) };

    if result == FALSE {
        println!("Warning: CloseHandle() returned FALSE");
        return false;
    }

    true
}

pub fn get_process_num_modules(handle: &HANDLE) -> Result<u32> {
    let mut module_list_size: u32 = 0;
    let result =
        unsafe { EnumProcessModules(*handle, std::ptr::null_mut(), 0, &mut module_list_size) };

    if result == 0 {
        anyhow::bail!(
            "EnumProcessModules failed, error: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(module_list_size / 8)
}

pub fn get_process_modules(handle: &Handle, pid: u32) -> Result<Vec<Module>> {
    let mut modules: Vec<Module> = Vec::new();

    let num_modules = get_process_num_modules(&handle.0)?;

    if num_modules > 0 {
        let h_module_snapshot: HANDLE =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };

        for i in 0..num_modules {
            match get_next_module(&h_module_snapshot, i) {
                Some(module) => {
                    modules.push(module);
                }
                None => break,
            }
        }

        let close_result = unsafe { CloseHandle(h_module_snapshot) };

        if close_result == 0 {
            println!(
                "CloseHandle() failed, error: {}",
                std::io::Error::last_os_error()
            );
        }
    }

    Ok(modules)
}

fn get_next_module(h_module_snapshot: &HANDLE, index: u32) -> Option<Module> {
    let mut entry: MODULEENTRY32 = MODULEENTRY32 {
        dwSize: 0,
        th32ModuleID: 0,
        th32ProcessID: 0,
        GlblcntUsage: 0,
        ProccntUsage: 0,
        modBaseAddr: std::ptr::null_mut(),
        modBaseSize: 0,
        hModule: std::ptr::null_mut(),
        szModule: [0; 256],
        szExePath: [0; 260],
    };
    entry.dwSize = std::mem::size_of::<MODULEENTRY32>() as u32;

    if index == 0 {
        let result = unsafe { Module32First(*h_module_snapshot, &mut entry) };

        if result != 0 {
            return Some(Module::from_entry(&entry));
        }
        println!("Warning: Module32First entry not found")
    } else {
        let result = unsafe { Module32Next(*h_module_snapshot, &mut entry) };

        if result != 0 {
            return Some(Module::from_entry(&entry));
        }
        println!("Warning: Module32Next entry not found")
    }

    None
}

pub fn resume_process(id: u32) -> bool {
    unsafe {
        let mut has_err = false;

        let te: &mut THREADENTRY32 = &mut std::mem::zeroed();
        (*te).dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

        let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);

        if Thread32First(snapshot, te) == 1 {
            loop {
                if id == (*te).th32OwnerProcessID {
                    let tid = (*te).th32ThreadID;

                    let thread: HANDLE = OpenThread(THREAD_SUSPEND_RESUME, FALSE, tid);
                    has_err |= ResumeThread(thread) as i32 == -1i32;

                    CloseHandle(thread);
                }

                if Thread32Next(snapshot, te) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        !has_err
    }
}

pub fn read_process_memory(
    handle: &Handle,
    position: usize,
    num_bytes: usize,
) -> Result<(Vec<u8>, usize)> {
    let mut bytes_read = 0;
    let mut buffer = vec![0_u8; num_bytes];

    let result = unsafe {
        ReadProcessMemory(
            handle.0,
            position as *const c_void,
            buffer.as_mut_ptr().cast(),
            num_bytes,
            &mut bytes_read,
        )
    };

    if result == FALSE {
        anyhow::bail!(
            "ReadProcessMemory() failed: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok((buffer, bytes_read))
}

pub fn write_process_memory(handle: &Handle, position: usize, bytes: &[u8]) -> Result<usize> {
    let mut bytes_written = 0;

    let result: i32 = unsafe {
        WriteProcessMemory(
            handle.0,
            position as *mut _,
            bytes.as_ptr() as *const _,
            bytes.len(),
            &mut bytes_written,
        )
    };

    if result == FALSE {
        anyhow::bail!(
            "WriteProcessMemory() failed: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(bytes_written)
}
