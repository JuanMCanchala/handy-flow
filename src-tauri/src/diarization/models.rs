//! On-demand ONNX model downloads for speaker diarization: a segmentation
//! model (pyannote segmentation-3.0) and a speaker-embedding model (WeSpeaker),
//! both small enough to keep diarization opt-in and separate from the
//! transcription model catalog in `managers/model.rs`. Nothing here is
//! downloaded unless the user enables diarization in Settings.
//!
//! Both models are sourced from k2-fsa/sherpa-onnx's Apache-2.0-licensed
//! GitHub release assets (not the blob.handy.computer mirror used for
//! transcription models), since sherpa-onnx already publishes ONNX exports
//! of pyannote segmentation-3.0 and WeSpeaker with permissive packaging.

use anyhow::{anyhow, Context, Result};
use bzip2::read::BzDecoder;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;
use tauri::AppHandle;

/// One of the two fixed diarization models. There is no user-facing catalog
/// (unlike transcription models) since exactly one segmentation model and one
/// embedding model are required for the pipeline to run at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum DiarizationModelKind {
    Segmentation,
    Embedding,
}

/// Where the downloaded bytes for a model come from.
enum ModelSource {
    /// The URL serves the ONNX file directly.
    DirectFile,
    /// The URL serves a `.tar.bz2` archive; `entry_path` is the path of the
    /// `.onnx` file inside it to extract.
    TarBz2 { entry_path: &'static str },
}

struct DiarizationModelSpec {
    /// Filename the model is stored under locally (independent of the
    /// archive entry name or upstream asset name).
    filename: &'static str,
    url: &'static str,
    source: ModelSource,
    /// SHA-256 of the final extracted/downloaded `.onnx` file (not the
    /// archive, when `source` is `TarBz2`).
    sha256: &'static str,
    size_bytes: u64,
}

/// pyannote/segmentation-3.0 exported to ONNX (fp32), from sherpa-onnx's
/// `speaker-segmentation-models` GitHub release. The release asset is a
/// `.tar.bz2` containing `model.onnx` (fp32) and `model.int8.onnx`; the
/// fp32 model is used for accuracy.
const SEGMENTATION_MODEL: DiarizationModelSpec = DiarizationModelSpec {
    filename: "pyannote-segmentation-3.0.onnx",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
    source: ModelSource::TarBz2 {
        entry_path: "sherpa-onnx-pyannote-segmentation-3-0/model.onnx",
    },
    sha256: "220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079",
    size_bytes: 5_992_913,
};

/// WeSpeaker ResNet34 English speaker-embedding model, from sherpa-onnx's
/// `speaker-recongition-models` [sic] GitHub release.
const EMBEDDING_MODEL: DiarizationModelSpec = DiarizationModelSpec {
    filename: "wespeaker_en_voxceleb_resnet34.onnx",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34.onnx",
    source: ModelSource::DirectFile,
    sha256: "5ef208a9da1453335308a6b6f4e6dfbd7e183a38b604de0a57664f45d257fe94",
    size_bytes: 26_534_365,
};

fn spec_for(kind: DiarizationModelKind) -> &'static DiarizationModelSpec {
    match kind {
        DiarizationModelKind::Segmentation => &SEGMENTATION_MODEL,
        DiarizationModelKind::Embedding => &EMBEDDING_MODEL,
    }
}

/// Frontend-facing status of one diarization model.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DiarizationModelInfo {
    pub kind: DiarizationModelKind,
    pub filename: String,
    pub size_bytes: u64,
    pub is_downloaded: bool,
}

/// Downloads and locates the two diarization ONNX models in the app data
/// directory (`{app_data_dir}/models/diarization/`).
pub struct DiarizationModelManager {
    models_dir: PathBuf,
}

impl DiarizationModelManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let models_dir = app_data_dir.join("models").join("diarization");
        fs::create_dir_all(&models_dir)
            .with_context(|| format!("Failed to create {:?}", models_dir))?;
        Ok(Self { models_dir })
    }

    fn path_for(&self, kind: DiarizationModelKind) -> PathBuf {
        self.models_dir.join(spec_for(kind).filename)
    }

    pub fn is_downloaded(&self, kind: DiarizationModelKind) -> bool {
        self.path_for(kind).exists()
    }

    /// Both models present — the pipeline can run.
    pub fn is_ready(&self) -> bool {
        self.is_downloaded(DiarizationModelKind::Segmentation)
            && self.is_downloaded(DiarizationModelKind::Embedding)
    }

    pub fn model_path(&self, kind: DiarizationModelKind) -> Result<PathBuf> {
        let path = self.path_for(kind);
        if !path.exists() {
            return Err(anyhow!("Diarization model {:?} is not downloaded", kind));
        }
        Ok(path)
    }

    pub fn get_models_status(&self) -> Vec<DiarizationModelInfo> {
        [DiarizationModelKind::Segmentation, DiarizationModelKind::Embedding]
            .into_iter()
            .map(|kind| {
                let spec = spec_for(kind);
                DiarizationModelInfo {
                    kind,
                    filename: spec.filename.to_string(),
                    size_bytes: spec.size_bytes,
                    is_downloaded: self.is_downloaded(kind),
                }
            })
            .collect()
    }

    /// Download one model (if not already present) and verify its SHA-256.
    /// On mismatch or I/O failure the partial file is removed.
    pub async fn download(&self, kind: DiarizationModelKind) -> Result<()> {
        let spec = spec_for(kind);
        let dest = self.path_for(kind);
        if dest.exists() {
            return Ok(());
        }

        let downloaded = fetch_bytes(spec.url).await?;

        let final_bytes = match &spec.source {
            ModelSource::DirectFile => downloaded,
            ModelSource::TarBz2 { entry_path } => {
                extract_from_tar_bz2(&downloaded, entry_path)
                    .with_context(|| format!("Failed to extract {entry_path} from {}", spec.url))?
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(&final_bytes);
        let digest = hex_encode(&hasher.finalize());
        if digest != spec.sha256 {
            return Err(anyhow!(
                "Checksum mismatch for {}: expected {}, got {}",
                spec.filename,
                spec.sha256,
                digest
            ));
        }

        let tmp_path = dest.with_extension("part");
        fs::write(&tmp_path, &final_bytes)
            .with_context(|| format!("Failed to write {:?}", tmp_path))?;
        fs::rename(&tmp_path, &dest)
            .with_context(|| format!("Failed to move {:?} into place", dest))?;
        Ok(())
    }

    /// Remove a downloaded model, freeing disk space.
    pub fn delete(&self, kind: DiarizationModelKind) -> Result<()> {
        let path = self.path_for(kind);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete {:?}", path))?;
        }
        Ok(())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Download a URL's full response body into memory, retrying transient
/// network failures a few times (GitHub release asset downloads can be
/// flaky). Models here are small enough (<30 MB) that buffering the whole
/// response is fine.
async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    const MAX_ATTEMPTS: u32 = 5;
    let mut last_err = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match fetch_bytes_once(url).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                last_err = Some(e);
                if attempt < MAX_ATTEMPTS {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("Failed to download {url}")))
}

async fn fetch_bytes_once(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to request {url}"))?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Download of {url} failed with status {}",
            response.status()
        ));
    }
    let mut out = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("Failed reading {url} stream"))?;
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

/// Extract one file's bytes from a `.tar.bz2` archive by its path within
/// the archive.
fn extract_from_tar_bz2(archive_bytes: &[u8], entry_path: &str) -> Result<Vec<u8>> {
    let decoder = BzDecoder::new(archive_bytes);
    let mut archive = Archive::new(decoder);
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry.path().context("Failed to read tar entry path")?;
        if path.to_string_lossy() == entry_path {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("Failed to read tar entry contents")?;
            return Ok(buf);
        }
    }
    Err(anyhow!("Entry '{entry_path}' not found in archive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_status_reports_both_kinds() {
        let dir = std::env::temp_dir().join(format!(
            "handy_diarization_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let manager = DiarizationModelManager { models_dir: dir.clone() };

        let statuses = manager.get_models_status();
        assert_eq!(statuses.len(), 2);
        assert!(statuses.iter().all(|s| !s.is_downloaded));
        assert!(!manager.is_ready());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn is_downloaded_reflects_file_presence() {
        let dir = std::env::temp_dir().join(format!(
            "handy_diarization_test2_{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let manager = DiarizationModelManager { models_dir: dir.clone() };

        fs::write(
            manager.path_for(DiarizationModelKind::Segmentation),
            b"stub",
        )
        .unwrap();

        assert!(manager.is_downloaded(DiarizationModelKind::Segmentation));
        assert!(!manager.is_downloaded(DiarizationModelKind::Embedding));
        assert!(!manager.is_ready());

        fs::remove_dir_all(&dir).ok();
    }
}
