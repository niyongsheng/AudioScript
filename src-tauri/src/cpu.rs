//! CPU 指令集探测与本地模型的后端选择。

const CT2_USE_MKL: &str = "CT2_USE_MKL";

/// 必须在任何 CTranslate2 调用之前执行：MKL 是否启用只在第一次调用时读取一次环境变量。
///
/// ct2rs 静态链接的 Intel oneMKL 2025 起要求 AVX（官方弃用公告），2026 只把 AVX2 及以上
/// 当作受支持分支。GenuineIntel 且没有 AVX2 的办公机（Pentium / Celeron / Atom）
/// 加载模型、打包 GEMM 权重时会执行 AVX/AVX2 指令，进程以
/// 0xc000001d（STATUS_ILLEGAL_INSTRUCTION）静默退出；`CT2_FORCE_CPU_ISA` 管不到 MKL，
/// 只能在 MKL 被选中之前关掉它，退回 ruy。
pub fn apply_ct2_backend_env() {
    if mkl_unusable() && std::env::var_os(CT2_USE_MKL).is_none() {
        set_native_env(CT2_USE_MKL, "0");
    }
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn mkl_unusable() -> bool {
    should_disable_mkl(std::is_x86_feature_detected!("avx2"))
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn mkl_unusable() -> bool {
    false
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
    fn mkl_is_disabled_below_avx2() {
        assert!(should_disable_mkl(false));
        assert!(!should_disable_mkl(true));
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
