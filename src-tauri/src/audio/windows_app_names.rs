use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoExW, GetFileVersionInfoSizeExW, VerQueryValueW, FILE_VER_GET_LOCALISED,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

const IMAGE_PATH_CAPACITY: usize = 32_768;

pub fn description(pid: u32) -> Option<String> {
    let path = image_path(pid)?;
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    cache
        .entry(path.clone())
        .or_insert_with(|| file_description(&path))
        .clone()
}

fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn image_path(pid: u32) -> Option<String> {
    let mut buffer = vec![0u16; IMAGE_PATH_CAPACITY];
    let mut len = buffer.len() as u32;
    let ok = unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let ok =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        ok
    };
    if ok == 0 {
        return None;
    }
    buffer.truncate(len as usize);
    String::from_utf16(&buffer).ok()
}

fn file_description(path: &str) -> Option<String> {
    let path_wide = wide(path);
    let mut handle = 0u32;
    let size = unsafe {
        GetFileVersionInfoSizeExW(FILE_VER_GET_LOCALISED, path_wide.as_ptr(), &mut handle)
    };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u32; size as usize / 4 + 1];
    let ok = unsafe {
        GetFileVersionInfoExW(
            FILE_VER_GET_LOCALISED,
            path_wide.as_ptr(),
            0,
            size,
            data.as_mut_ptr().cast(),
        )
    };
    if ok == 0 {
        return None;
    }
    let (language, codepage) = translation(&data)?;
    let key = wide(&format!(
        "\\StringFileInfo\\{language:04x}{codepage:04x}\\FileDescription"
    ));
    query_string(&data, &key)
}

fn translation(data: &[u32]) -> Option<(u16, u16)> {
    let key = wide("\\VarFileInfo\\Translation");
    let mut ptr: *mut c_void = std::ptr::null_mut();
    let mut len = 0u32;
    let ok = unsafe { VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), &mut ptr, &mut len) };
    if ok == 0 || ptr.is_null() || len < 4 {
        return None;
    }
    let pairs = unsafe { std::slice::from_raw_parts(ptr.cast::<u16>(), len as usize / 2) };
    let ui = unsafe { GetUserDefaultUILanguage() };
    let exact = pairs.chunks_exact(2).find(|pair| pair[0] == ui);
    if let Some(pair) = exact {
        return Some((pair[0], pair[1]));
    }
    let primary = ui & 0x03ff;
    let same_language = pairs
        .chunks_exact(2)
        .find(|pair| pair[0] & 0x03ff == primary);
    if let Some(pair) = same_language {
        return Some((pair[0], pair[1]));
    }
    pairs
        .chunks_exact(2)
        .find(|pair| pair[0] == 0x0409)
        .or_else(|| pairs.chunks_exact(2).next())
        .map(|pair| (pair[0], pair[1]))
}

fn query_string(data: &[u32], key: &[u16]) -> Option<String> {
    let mut ptr: *mut c_void = std::ptr::null_mut();
    let mut len = 0u32;
    let ok = unsafe { VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), &mut ptr, &mut len) };
    if ok == 0 || ptr.is_null() || len == 0 {
        return None;
    }
    let units = unsafe { std::slice::from_raw_parts(ptr.cast::<u16>(), len as usize) };
    let text = String::from_utf16_lossy(units);
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
