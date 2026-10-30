use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::catalog::{ArchiveFormat, CatalogEntry};
use super::{ModelError, IO_CHUNK};
use crate::i18n::{tf, Key};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneProfile {
    Off,
    MixedInt8,
    AllInt8,
}

impl PruneProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "sherpa-asr-mixed-int8" => Some(Self::MixedInt8),
            "sherpa-asr-all-int8" => Some(Self::AllInt8),
            _ => None,
        }
    }

    fn from_catalog(value: Option<&str>) -> Self {
        value.and_then(Self::parse).unwrap_or(Self::Off)
    }

    fn keeps(self, name: &str) -> bool {
        if matches!(name, "tokens.txt" | "bpe.model") {
            return true;
        }
        if !name.ends_with(".onnx") {
            return false;
        }
        let int8 = name.contains(".int8.");
        let role = ["encoder", "decoder", "joiner"]
            .into_iter()
            .find(|prefix| name.starts_with(*prefix));
        match (self, role) {
            (Self::MixedInt8, Some("encoder" | "joiner")) => int8,
            (Self::MixedInt8, Some("decoder")) => !int8,
            (Self::AllInt8, Some(_)) => int8,
            _ => false,
        }
    }
}

pub struct ExtractSpec<'a> {
    pub format: ArchiveFormat,
    pub roots: Vec<&'a str>,
    pub prune: PruneProfile,
    pub allowed: Vec<&'a str>,
    pub max_bytes: u64,
}

impl<'a> ExtractSpec<'a> {
    pub fn from_entry(entry: &'a CatalogEntry) -> Result<Self, ModelError> {
        let archive = entry
            .archive
            .as_ref()
            .ok_or_else(|| ModelError::Internal(tf(Key::ModelNoArchiveInfo, &[&entry.id])))?;
        let mut allowed = Vec::new();
        if let Some(asr) = &entry.required.asr {
            allowed.extend(asr.files.iter().map(String::as_str));
        }
        if let Some(mt) = &entry.required.mt {
            allowed.extend(mt.files.iter().map(String::as_str));
        }
        Ok(Self {
            format: archive.format,
            roots: entry.archive_roots(),
            prune: PruneProfile::from_catalog(entry.prune_profile.as_deref()),
            allowed,
            max_bytes: archive.max_extracted_bytes,
        })
    }
}

fn safe_rel_path(name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in name.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains(':') {
            return None;
        }
        out.push(part);
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

fn strip_root<'a>(rel: &'a Path, roots: &[&str]) -> Option<&'a Path> {
    if roots.is_empty() {
        return Some(rel);
    }
    let first = rel.components().next()?;
    let first = first.as_os_str().to_str()?;
    if !roots.contains(&first) {
        return None;
    }
    let rest = rel.strip_prefix(first).ok()?;
    Some(rest)
}

pub fn extract_archive(
    archive: &Path,
    spec: &ExtractSpec<'_>,
    staging: &Path,
    cancel: &AtomicBool,
) -> Result<(), ModelError> {
    let file = File::open(archive)?;
    let reader: Box<dyn Read> = match spec.format {
        ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
        ArchiveFormat::TarBz2 => Box::new(bzip2::read::BzDecoder::new(file)),
    };
    let mut tar = tar::Archive::new(reader);
    let mut total = 0u64;
    let mut buffer = [0u8; IO_CHUNK];

    for entry in tar
        .entries()
        .map_err(|e| ModelError::Extract(e.to_string()))?
    {
        if cancel.load(Ordering::SeqCst) {
            return Err(ModelError::Cancelled);
        }

        let mut entry = entry.map_err(|e| ModelError::Extract(e.to_string()))?;
        let header = entry.header();
        let entry_type = header.entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(ModelError::UnsafeArchive(tf(
                Key::ModelUnsupportedEntry,
                &[&format!("{entry_type:?}")],
            )));
        }
        if entry_type.is_dir() {
            continue;
        }

        let path = entry
            .path()
            .map_err(|e| ModelError::Extract(e.to_string()))?
            .to_string_lossy()
            .into_owned();
        let Some(rel) = safe_rel_path(&path) else {
            return Err(ModelError::UnsafeArchive(tf(
                Key::ModelUnsafePath,
                &[&path],
            )));
        };
        let Some(rel) = strip_root(&rel, &spec.roots) else {
            continue;
        };
        let components: Vec<_> = rel.components().collect();
        let top_level = components.len() == 1;
        if spec.prune != PruneProfile::Off {
            if !top_level {
                continue;
            }
            let Some(name) = components[0].as_os_str().to_str() else {
                continue;
            };
            if !spec.prune.keeps(name) {
                continue;
            }
        } else if !top_level
            && !spec
                .allowed
                .iter()
                .any(|allowed| Path::new(*allowed) == rel)
        {
            continue;
        }

        total = total.saturating_add(entry.size());
        if spec.max_bytes > 0 && total > spec.max_bytes {
            return Err(ModelError::UnsafeArchive(tf(
                Key::ModelTooLarge,
                &[&spec.max_bytes.to_string()],
            )));
        }

        let dest = staging.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&dest)?;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(ModelError::Cancelled);
            }
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            out.write_all(&buffer[..read])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (name, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, Cursor::new(data))
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn spec<'a>(
        format: ArchiveFormat,
        roots: Vec<&'a str>,
        prune: PruneProfile,
    ) -> ExtractSpec<'a> {
        ExtractSpec {
            format,
            roots,
            prune,
            allowed: Vec::new(),
            max_bytes: 10 * 1024 * 1024,
        }
    }

    #[test]
    fn safe_paths_only() {
        assert_eq!(safe_rel_path("a/b.txt").unwrap(), PathBuf::from("a/b.txt"));
        assert_eq!(safe_rel_path("./a//b").unwrap(), PathBuf::from("a/b"));
        assert!(safe_rel_path("../x").is_none());
        assert!(safe_rel_path("a/../../x").is_none());
        assert!(
            safe_rel_path("/etc/passwd").is_some(),
            "前导 / 会被当作空段剥掉，落点仍限制在 staging 内"
        );
        assert!(safe_rel_path("C:\\x").is_none());
        assert!(safe_rel_path("a\\..\\x").is_none());
    }

    #[test]
    fn asr_prune_keeps_only_the_expected_files() {
        let mixed = PruneProfile::MixedInt8;
        assert!(mixed.keeps("encoder-x.int8.onnx"));
        assert!(!mixed.keeps("encoder-x.onnx"));
        assert!(mixed.keeps("decoder-x.onnx"));
        assert!(!mixed.keeps("decoder-x.int8.onnx"));
        assert!(mixed.keeps("joiner-x.int8.onnx"));
        assert!(mixed.keeps("tokens.txt"));
        assert!(mixed.keeps("bpe.model"));
        assert!(!mixed.keeps("test_wavs/a.wav"));

        let all = PruneProfile::AllInt8;
        assert!(all.keeps("encoder-x.int8.onnx"));
        assert!(all.keeps("decoder-x.int8.onnx"));
        assert!(all.keeps("joiner-x.int8.onnx"));
        assert!(!all.keeps("decoder-x.onnx"));
        assert!(all.keeps("tokens.txt"));
    }

    #[test]
    fn extracts_tar_gz_and_strips_root() {
        let bytes = tar_gz(&[
            ("root/encoder-x.int8.onnx", b"enc"),
            ("root/decoder-x.onnx", b"dec"),
            ("root/joiner-x.int8.onnx", b"joi"),
            ("root/tokens.txt", b"tok"),
            ("root/64/extra.onnx", b"extra"),
            ("root/test_wavs/a.wav", b"wav"),
        ]);
        let dir = std::env::temp_dir().join(format!("extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        std::fs::write(&archive, &bytes).unwrap();

        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let cancel = AtomicBool::new(false);
        let s = spec(ArchiveFormat::TarGz, vec!["root"], PruneProfile::MixedInt8);
        extract_archive(&archive, &s, &out, &cancel).unwrap();

        assert!(out.join("encoder-x.int8.onnx").is_file());
        assert!(out.join("tokens.txt").is_file());
        assert!(!out.join("64").exists());
        assert!(!out.join("test_wavs").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_nested_required_files() {
        let bytes = tar_gz(&[
            ("root/conv_frontend.onnx", b"conv"),
            ("root/encoder.int8.onnx", b"enc"),
            ("root/decoder.int8.onnx", b"dec"),
            ("root/tokenizer/merges.txt", b"merges"),
            ("root/tokenizer/tokenizer_config.json", b"config"),
            ("root/tokenizer/vocab.json", b"vocab"),
            ("root/test_wavs/a.wav", b"wav"),
        ]);
        let dir = std::env::temp_dir().join(format!("extract-nested-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        std::fs::write(&archive, &bytes).unwrap();

        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let cancel = AtomicBool::new(false);
        let mut s = spec(ArchiveFormat::TarGz, vec!["root"], PruneProfile::Off);
        s.allowed = vec![
            "tokenizer/merges.txt",
            "tokenizer/tokenizer_config.json",
            "tokenizer/vocab.json",
        ];
        extract_archive(&archive, &s, &out, &cancel).unwrap();

        assert!(out.join("conv_frontend.onnx").is_file());
        assert!(out.join("tokenizer/merges.txt").is_file());
        assert!(out.join("tokenizer/tokenizer_config.json").is_file());
        assert!(out.join("tokenizer/vocab.json").is_file());
        assert!(!out.join("test_wavs").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_tar_bz2() {
        let mut builder = tar::Builder::new(bzip2::write::BzEncoder::new(
            Vec::new(),
            bzip2::Compression::fast(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_size(3);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "root/model.bin", Cursor::new(b"abc"))
            .unwrap();
        let bytes = builder.into_inner().unwrap().finish().unwrap();

        let dir = std::env::temp_dir().join(format!("extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.bz2");
        std::fs::write(&archive, &bytes).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let cancel = AtomicBool::new(false);
        let s = spec(ArchiveFormat::TarBz2, vec!["root"], PruneProfile::Off);
        extract_archive(&archive, &s, &out, &cancel).unwrap();
        assert_eq!(std::fs::read(out.join("model.bin")).unwrap(), b"abc");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_entries_outside_the_roots_but_continues() {
        let bytes = tar_gz(&[("other/x.txt", b"x"), ("root/keep.txt", b"k")]);
        let dir = std::env::temp_dir().join(format!("extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        std::fs::write(&archive, &bytes).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let cancel = AtomicBool::new(false);
        let s = spec(ArchiveFormat::TarGz, vec!["root"], PruneProfile::Off);
        extract_archive(&archive, &s, &out, &cancel).unwrap();
        assert!(out.join("keep.txt").is_file());
        assert!(!out.join("x.txt").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn aborts_when_extracted_size_exceeds_the_cap() {
        let bytes = tar_gz(&[("root/a.bin", &[0u8; 2048])]);
        let dir = std::env::temp_dir().join(format!("extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        std::fs::write(&archive, &bytes).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let cancel = AtomicBool::new(false);
        let mut s = spec(ArchiveFormat::TarGz, vec!["root"], PruneProfile::Off);
        s.max_bytes = 1024;
        let e = extract_archive(&archive, &s, &out, &cancel).unwrap_err();
        assert!(matches!(e, ModelError::UnsafeArchive(_)), "{e}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
