use crate::config::Preferences;

pub fn log_startup(prefs: &Preferences) {
    let os = os_info::get();
    let cpu = cpu_brand().unwrap_or_else(|| "unknown".into());
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    let memory_mb = total_memory_bytes()
        .map(|bytes| bytes / (1024 * 1024))
        .unwrap_or(0);
    let webview = tauri::webview_version().unwrap_or_default();
    let system_locale = sys_locale::get_locale().unwrap_or_default();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        platform = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        os = %os,
        cpu = %cpu,
        cpu_features = %cpu_features(),
        cores,
        memory_mb,
        webview = %webview,
        system_locale = %system_locale,
        ui_locale = crate::i18n::locale().as_str(),
        debug = cfg!(debug_assertions),
        "运行环境"
    );

    tracing::info!(
        audio_source = %prefs.audio_source,
        device = prefs.device_id.as_deref().unwrap_or("default"),
        mix_mic = prefs.mix_mic_device_id.as_deref().unwrap_or("default"),
        mix_system = prefs.mix_system_device_id.as_deref().unwrap_or("default"),
        asr_engine = prefs.asr_engine.as_str(),
        asr_model_tier = prefs.asr_model_tier.as_str(),
        translate_engine = prefs.translate_engine.as_str(),
        mt_model_tier = prefs.mt_model_tier.as_str(),
        source_lang = %prefs.source_lang,
        target_lang = %prefs.target_lang,
        zh_script = prefs.zh_script.as_str(),
        models_dir = prefs.models_dir.as_deref().unwrap_or("default"),
        analytics_enabled = prefs.analytics_enabled,
        analytics_tier = crate::telemetry::ANALYTICS_TIER.as_str(),
        "当前配置"
    );
}

pub fn os_version() -> Option<String> {
    let version = os_info::get().version().to_string();
    (!version.is_empty() && version != "Unknown").then_some(version)
}

pub fn app_audio_supported() -> bool {
    #[cfg(target_os = "windows")]
    {
        matches!(
            os_info::get().version(),
            os_info::Version::Semantic(_, _, build) if *build >= 20_348
        )
    }
    #[cfg(target_os = "macos")]
    {
        matches!(
            os_info::get().version(),
            os_info::Version::Semantic(major, minor, _) if *major > 14 || (*major == 14 && *minor >= 4)
        )
    }
    #[cfg(target_os = "linux")]
    {
        pipewire_available()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn pipewire_available() -> bool {
    if std::env::var_os("PIPEWIRE_REMOTE").is_some() {
        return true;
    }
    let dir =
        std::env::var_os("PIPEWIRE_RUNTIME_DIR").or_else(|| std::env::var_os("XDG_RUNTIME_DIR"));
    dir.map(|dir| std::path::Path::new(&dir).join("pipewire-0").exists())
        .unwrap_or(false)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn cpu_features() -> String {
    let mut flags = Vec::new();
    for (flag, enabled) in [
        ("sse4.2", std::is_x86_feature_detected!("sse4.2")),
        ("avx", std::is_x86_feature_detected!("avx")),
        ("avx2", std::is_x86_feature_detected!("avx2")),
        ("fma", std::is_x86_feature_detected!("fma")),
        ("avx512f", std::is_x86_feature_detected!("avx512f")),
        ("avx512_vnni", std::is_x86_feature_detected!("avx512vnni")),
    ] {
        if enabled {
            flags.push(flag);
        }
    }
    flags.join(",")
}

#[cfg(target_arch = "aarch64")]
fn cpu_features() -> String {
    "neon".into()
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
fn cpu_features() -> String {
    String::new()
}

#[cfg(target_os = "macos")]
fn cpu_brand() -> Option<String> {
    sysctl_string("machdep.cpu.brand_string")
}

#[cfg(target_os = "linux")]
fn cpu_brand() -> Option<String> {
    let raw = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    raw.lines()
        .find_map(|line| {
            let value = line.strip_prefix("model name")?.split_once(':')?.1;
            Some(value.trim().to_string())
        })
        .filter(|name| !name.is_empty())
}

#[cfg(target_os = "windows")]
fn cpu_brand() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .ok()?
        .get_value::<String, _>("ProcessorNameString")
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn cpu_brand() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn total_memory_bytes() -> Option<u64> {
    let raw = sysctl_raw("hw.memsize")?;
    Some(u64::from_ne_bytes(raw.get(..8)?.try_into().ok()?))
}

#[cfg(target_os = "linux")]
fn total_memory_bytes() -> Option<u64> {
    let raw = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = raw.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[cfg(target_os = "windows")]
fn total_memory_bytes() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    (ok != 0).then_some(status.ullTotalPhys)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn total_memory_bytes() -> Option<u64> {
    None
}

#[cfg(target_os = "macos")]
fn sysctl_raw(name: &str) -> Option<Vec<u8>> {
    let cname = std::ffi::CString::new(name).ok()?;
    let mut size = 0usize;
    let first = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if first != 0 || size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; size];
    let second = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if second != 0 {
        return None;
    }
    buffer.truncate(size);
    Some(buffer)
}

#[cfg(target_os = "macos")]
fn sysctl_string(name: &str) -> Option<String> {
    let buffer = sysctl_raw(name)?;
    let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
    String::from_utf8(buffer[..end].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_version_is_parseable() {
        let version = os_version().expect("必须能读到系统版本");
        let major = version
            .split('.')
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .expect("版本号格式应为 major.minor.patch");
        assert!(major >= 10, "{version}");
    }

    #[test]
    fn hardware_probes_are_present() {
        assert!(cpu_brand().is_some(), "必须能读到 CPU 型号");
        assert!(total_memory_bytes().is_some(), "必须能读到总内存");
    }

    #[test]
    fn startup_log_records_environment_and_config() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<u8>>>);

        impl std::io::Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> MakeWriter<'a> for Capture {
            type Writer = Capture;

            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            log_startup(&Preferences::default());
        });

        let out = String::from_utf8(capture.0.lock().unwrap_or_else(|e| e.into_inner()).clone())
            .unwrap_or_default();
        assert!(out.contains("运行环境"), "{out}");
        assert!(out.contains(env!("CARGO_PKG_VERSION")), "{out}");
        assert!(out.contains("当前配置"), "{out}");
        assert!(out.contains("asr_model_tier"), "{out}");
    }
}
