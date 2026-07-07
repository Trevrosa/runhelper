use tracing::warn;
// sends `SIGKILL` on unix, `WM_CLOSE` on windows.
pub fn force_kill(pid: u32) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_TERMINATE, TerminateProcess,
        };

        let handle = unsafe { OpenProcess(PROCESS_TERMINATE, false.into(), pid) };
        let res = unsafe { TerminateProcess(handle, 1) };
        if res == 0 {
            let err = std::io::Error::last_os_error();
            warn!("could not kill pid {pid}: {err}");
        }
    }

    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid, libc::SIGKILL) };
        if res == -1 {
            let err = std::io::Error::last_os_error();
            warn!("could not kill pid {pid}: {err}");
        }
    }
}
