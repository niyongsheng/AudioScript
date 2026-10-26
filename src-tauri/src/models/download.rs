use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::{ModelError, IO_CHUNK};
use crate::i18n::{tf, Key};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

const OVERSHOOT: u64 = 1024 * 1024;

pub fn download_verified(
    url: &str,
    expected_sha256: &str,
    expected_size: u64,
    dest: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(), ModelError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ModelError::Cancelled);
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(60))
        .user_agent(concat!("AudioScript/", env!("CARGO_PKG_VERSION")))
        .build();

    let response = agent.get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => ModelError::Http(code),
        ureq::Error::Transport(t) => ModelError::Network(t.to_string()),
    })?;

    let total = response
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .or((expected_size > 0).then_some(expected_size));

    let mut reader = response.into_reader();
    let mut file = File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; IO_CHUNK];
    let mut received = 0u64;
    let mut last_report = Instant::now();

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(ModelError::Cancelled);
        }

        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        received += read as u64;

        if expected_size > 0 && received > expected_size + OVERSHOOT {
            return Err(ModelError::Network(tf(
                Key::ModelDownloadTooLarge,
                &[&received.to_string()],
            )));
        }

        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])?;

        if last_report.elapsed() >= PROGRESS_INTERVAL {
            on_progress(received, total);
            last_report = Instant::now();
        }
    }

    on_progress(received, total);
    file.sync_all()?;

    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(ModelError::Checksum {
            expected: expected_sha256.to_owned(),
            actual,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256_hex(data: &[u8]) -> String {
        format!("{:x}", Sha256::digest(data))
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn cancel_before_start_is_observed() {
        let cancel = AtomicBool::new(true);
        let dest = std::env::temp_dir().join(format!("dl-{}", uuid::Uuid::new_v4()));
        let e = download_verified("http://127.0.0.1:9/never", "", 0, &dest, &cancel, |_, _| {})
            .unwrap_err();
        assert!(matches!(e, ModelError::Cancelled), "{e}");
    }
}
