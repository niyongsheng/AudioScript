use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(mac_speech)");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        build_speech_bridge();
    }

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(target_os = "macos")]
fn sdk_major() -> Option<u32> {
    command_stdout("xcrun", &["--show-sdk-version"])?
        .split('.')
        .next()?
        .parse()
        .ok()
}

#[cfg(target_os = "macos")]
fn build_speech_bridge() {
    println!("cargo:rerun-if-changed=macos/speech_bridge.swift");
    println!("cargo:rerun-if-env-changed=AUDIOSCRIPT_NO_MAC_SPEECH");

    if std::env::var_os("AUDIOSCRIPT_NO_MAC_SPEECH").is_some() {
        println!("cargo:warning=AUDIOSCRIPT_NO_MAC_SPEECH 已设置，跳过 macOS 系统语音桥");
        return;
    }

    match sdk_major() {
        Some(major) if major >= 26 => {}
        Some(major) => {
            println!(
                "cargo:warning=macOS SDK 版本为 {major}，缺少 SpeechAnalyzer（需 26+），系统语音不可用"
            );
            return;
        }
        None => {
            println!("cargo:warning=无法读取 macOS SDK 版本，跳过系统语音桥");
            return;
        }
    }

    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        Ok(other) => {
            println!("cargo:warning=不支持的系统语音目标架构 {other}");
            return;
        }
        Err(_) => return,
    };

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let lib_path = out_dir.join("libAudioScriptSpeech.a");
    let target = format!("{arch}-apple-macos14.2");

    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-emit-library",
            "-static",
            "-parse-as-library",
            "-swift-version",
            "5",
            "-module-name",
            "AudioScriptSpeech",
            "-target",
            &target,
            "-O",
            "macos/speech_bridge.swift",
            "-o",
        ])
        .arg(&lib_path)
        .status();

    match status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("编译 macOS 系统语音桥失败：swiftc 退出码 {status}"),
        Err(e) => panic!("无法执行 swiftc：{e}"),
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=AudioScriptSpeech");

    if let Some(sdk) = command_stdout("xcrun", &["--show-sdk-path"]) {
        println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    }
    println!("cargo:rustc-link-lib=dylib=swiftCore");
    println!("cargo:rustc-link-lib=dylib=swift_Concurrency");
    for framework in ["Speech", "AVFoundation", "Foundation"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }

    println!("cargo:rustc-cfg=mac_speech");
}
