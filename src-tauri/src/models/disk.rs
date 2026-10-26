use std::path::Path;

#[cfg(unix)]
pub fn available_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    (ok == 0).then(|| stat.f_bavail as u64 * stat.f_frsize as u64)
}

#[cfg(windows)]
pub fn available_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

#[cfg(not(any(unix, windows)))]
pub fn available_space(_path: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_space_for_existing_directory() {
        let dir = std::env::temp_dir();
        if let Some(bytes) = available_space(&dir) {
            assert!(bytes > 0);
        }
    }
}
