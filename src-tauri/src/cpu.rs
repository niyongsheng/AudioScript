//! CPU 指令集探测与本地模型的后端选择。

const CT2_USE_MKL: &str = "CT2_USE_MKL";

/// 必须在任何 CTranslate2 调用之前执行：MKL 是否启用只在第一次调用时读取一次环境变量。
///
/// ct2rs 静态链接的 Intel oneMKL 2025 起要求 AVX（官方弃用公告），2026 只把 AVX2 及以上
/// 当作受支持分支。GenuineIntel 且没有 AVX2 的办公机（Pentium / Celeron / Atom）
/// 加载模型、打包 GEMM 权重时会执行 AVX/AVX2 指令，进程以
/// 0xc000001d（STATUS_ILLEGAL_INSTRUCTION）静默退出；`CT2_FORCE_CPU_ISA` 管不到 MKL，
/// 只能在 MKL 被选中之前关掉它，退回 ruy。
///
/// CTranslate2 的 CPU 分配器在 `CT2_WITH_MKL` 下无条件走 `mkl_malloc`，`CT2_USE_MKL=0`
/// 只切 GEMM 后端；真正不进 MKL 依赖 fork 里的 allocator 补丁（按 `mayiuse_mkl()` 选分配器）。
pub fn apply_ct2_backend_env() {
    apply_mkl_preference(true);
}

/// 必须在 prefs 加载后、第一次 CTranslate2 调用之前执行；显式设置的环境变量优先，便于排障。
pub fn apply_mkl_preference(enabled: bool) {
    if !mkl_platform_supported() || std::env::var_os(CT2_USE_MKL).is_some() {
        return;
    }
    if mkl_should_disable(enabled, mkl_cpu_supported()) {
        set_native_env(CT2_USE_MKL, "0");
    }
}

/// 当前平台是否编译了 MKL 后端且 CPU 能安全执行（有 AVX2）。
pub fn mkl_supported() -> bool {
    mkl_platform_supported() && mkl_cpu_supported()
}

fn mkl_platform_supported() -> bool {
    cfg!(all(
        not(target_os = "macos"),
        any(target_arch = "x86", target_arch = "x86_64")
    ))
}

pub fn ct2_mkl_disabled() -> bool {
    std::env::var(CT2_USE_MKL)
        .map(|value| !matches!(value.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(false)
}

/// CTranslate2 的 x86-64 基线要求 SSE4.1（见 upstream hardware_support）。
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn ct2_usable() -> bool {
    std::is_x86_feature_detected!("sse4.1")
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub fn ct2_usable() -> bool {
    true
}

#[cfg_attr(
    not(any(target_arch = "x86", target_arch = "x86_64")),
    allow(dead_code)
)]
fn should_disable_mkl(has_avx2: bool) -> bool {
    !has_avx2
}

fn mkl_should_disable(enabled: bool, cpu_supported: bool) -> bool {
    !enabled || !cpu_supported
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn mkl_cpu_supported() -> bool {
    !mkl_unusable()
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn mkl_cpu_supported() -> bool {
    false
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn mkl_unusable() -> bool {
    should_disable_mkl(std::is_x86_feature_detected!("avx2"))
}

fn set_native_env(key: &str, value: &str) {
    std::env::set_var(key, value);

    #[cfg(target_os = "windows")]
    unsafe {
        // SAFETY: CString 保证非空且以 NUL 结尾；std::env::set_var 只改进程环境块，
        // CTranslate2 用 std::getenv 读的是 MSVC 运行时的另一份副本。
        use std::ffi::{c_char, c_int, CString};
        extern "C" {
            fn _putenv_s(name: *const c_char, value: *const c_char) -> c_int;
        }
        if let (Ok(key), Ok(value)) = (CString::new(key), CString::new(value)) {
            _putenv_s(key.as_ptr(), value.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mkl_is_disabled_below_avx2_or_by_preference() {
        assert!(should_disable_mkl(false));
        assert!(!should_disable_mkl(true));
        assert!(!mkl_should_disable(true, true));
        assert!(mkl_should_disable(false, true));
        assert!(mkl_should_disable(true, false));
    }

    #[test]
    fn native_env_is_visible_to_the_process() {
        set_native_env("AUDIOSCRIPT_CPU_ENV_PROBE", "1");
        assert_eq!(
            std::env::var("AUDIOSCRIPT_CPU_ENV_PROBE").as_deref(),
            Ok("1")
        );
        std::env::remove_var("AUDIOSCRIPT_CPU_ENV_PROBE");
    }

    #[test]
    fn ct2_is_usable_on_this_machine() {
        assert!(ct2_usable(), "本地模型至少要求 SSE4.1 / arm64");
    }
}
