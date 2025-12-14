use std::ffi::CStr;

use anyhow::anyhow;
use windows::Win32::{
    Foundation::{CloseHandle, ERROR_NO_MORE_FILES},
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPPROCESS,
    },
};

use crate::ProcessInfo;

pub(crate) fn get_processes() -> anyhow::Result<Vec<ProcessInfo>> {
    let mut processes = Vec::new();

    // we can't return early so we set the error and return it later.
    let mut error = None;
    unsafe {
        let snapshot_handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        {
            // do NOT return early from this block; `CloseHandle` must be run after we are done.
            let mut process_entry: PROCESSENTRY32 = std::mem::zeroed();
            let process_entry_size = u32::try_from(std::mem::size_of::<PROCESSENTRY32>());
            match process_entry_size {
                Ok(process_entry_size) => {
                    process_entry.dwSize = process_entry_size;
                    match Process32First(snapshot_handle, &raw mut process_entry) {
                        Ok(()) => loop {
                            processes.push(ProcessInfo {
                                name: CStr::from_ptr(process_entry.szExeFile.as_ptr())
                                    .to_string_lossy()
                                    .into_owned(),
                                pid: process_entry.th32ProcessID,
                                parent_pid: process_entry.th32ParentProcessID,
                            });
                            match Process32Next(snapshot_handle, &raw mut process_entry) {
                                Ok(()) => {}
                                Err(e) => {
                                    if e.code() != ERROR_NO_MORE_FILES.into() {
                                        error = Some(e.into());
                                    }
                                    break;
                                }
                            }
                        },
                        Err(e) => {
                            error = Some(e.into());
                        }
                    }
                }
                Err(e) => {
                    error = Some(anyhow!("couldn't cast process entry size to u32: {e}"));
                }
            }
        }
        CloseHandle(snapshot_handle)?;
    }

    if let Some(error) = error {
        return Err(error);
    }

    Ok(processes)
}
