//! Import audio/video files, decode + resample them to 16 kHz mono, and run
//! them through the existing transcription engine (local model or cloud STT,
//! selected by [`crate::managers::transcription::TranscriptionManager`]).
//!
//! Long files are split into fixed-size chunks so progress can be reported
//! and the operation can be cancelled between chunks. Each chunk becomes one
//! [`crate::transcript_export::TranscriptSegment`]; engines that don't return
//! per-word timestamps still get one segment per chunk, which is enough for
//! usable (if coarse) SRT/VTT export.

use crate::audio_toolkit::audio::FrameResampler;
use crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE;
use crate::managers::history::HistoryManager;
use crate::managers::transcription::TranscriptionManager;
use crate::transcript_export::TranscriptSegment;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::conv::IntoSample;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tauri::AppHandle;
use tauri_specta::Event;

/// File extensions accepted by the import dialog and drop zone. Kept in sync
/// with the symphonia bundles enabled in Cargo.toml (mp3, flac, aac/mp4/mov,
/// ogg/vorbis, wav/riff). Containers like webm/mkv are intentionally not
/// listed: no demuxer/codec for them is in the dependency tree.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["wav", "mp3", "m4a", "flac", "ogg", "mp4", "mov"];

/// Each imported chunk covers this much source audio before being handed to
/// the transcription engine. 30s matches Whisper's native context window.
const CHUNK_DURATION_SECS: f64 = 30.0;

/// Progress emitted to the frontend while an import is running.
#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct FileImportProgressEvent {
    /// 0-based index of the chunk currently being transcribed.
    pub current_chunk: u32,
    pub total_chunks: u32,
}

pub fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(ext))
        })
        .unwrap_or(false)
}

/// Single-flight guard: only one import can run at a time, and it can be
/// cancelled from a separate command call (the frontend has no handle to the
/// blocking task itself).
pub struct FileImportManager {
    busy: AtomicBool,
    cancel_flag: Mutex<Option<Arc<AtomicBool>>>,
}

impl Default for FileImportManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileImportManager {
    pub fn new() -> Self {
        Self {
            busy: AtomicBool::new(false),
            cancel_flag: Mutex::new(None),
        }
    }

    /// Claim the single import slot. Returns `None` if an import is already
    /// running.
    fn try_start(&self) -> Option<ImportGuard<'_>> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return None;
        }
        let cancel_flag = Arc::new(AtomicBool::new(false));
        *self.cancel_flag.lock().unwrap() = Some(cancel_flag.clone());
        Some(ImportGuard {
            manager: self,
            cancel_flag,
        })
    }

    /// Signal the running import (if any) to stop at the next chunk boundary.
    pub fn cancel(&self) {
        if let Some(flag) = self.cancel_flag.lock().unwrap().as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

struct ImportGuard<'a> {
    manager: &'a FileImportManager,
    cancel_flag: Arc<AtomicBool>,
}

impl Drop for ImportGuard<'_> {
    fn drop(&mut self) {
        *self.manager.cancel_flag.lock().unwrap() = None;
        self.manager.busy.store(false, Ordering::SeqCst);
    }
}

/// Decode an audio/video file with symphonia and resample it to 16 kHz mono.
/// Returns the PCM samples and the duration in seconds (from the decoded
/// sample count, so it reflects exactly what will be transcribed).
fn decode_and_resample(path: &Path) -> Result<(Vec<f32>, f64)> {
    let file = std::fs::File::open(path).context("Failed to open file")?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Unsupported or corrupt audio/video file")?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No decodable audio track found in file"))?;
    let track_id = track.id;
    let source_hz = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("Audio track has no sample rate"))?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1)
        .max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Unsupported audio codec")?;

    // Decode to interleaved f32, downmixed to mono, at the source rate.
    let mut mono: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break, // end of stream
            Err(e) => return Err(anyhow!("Error reading packet: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => append_downmixed(&decoded, channels, &mut mono),
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("Error decoding packet: {e}")),
        }
    }

    if mono.is_empty() {
        return Err(anyhow!("File contains no decodable audio samples"));
    }

    let duration_secs = mono.len() as f64 / source_hz as f64;

    let resampled = resample_to_16k_mono(&mono, source_hz);
    Ok((resampled, duration_secs))
}

/// Downmix a decoded audio buffer to mono f32 and append it to `out`.
fn append_downmixed(decoded: &AudioBufferRef<'_>, channels: usize, out: &mut Vec<f32>) {
    macro_rules! downmix {
        ($buf:expr) => {{
            let spec_channels = $buf.spec().channels.count().max(1);
            let frames = $buf.frames();
            out.reserve(frames);
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..spec_channels {
                    let sample: f32 = $buf.chan(ch)[i].into_sample();
                    sum += sample;
                }
                out.push(sum / spec_channels as f32);
            }
        }};
    }
    let _ = channels; // channel count comes from the buffer spec itself
    match decoded {
        AudioBufferRef::U8(buf) => downmix!(buf),
        AudioBufferRef::U16(buf) => downmix!(buf),
        AudioBufferRef::U24(buf) => downmix!(buf),
        AudioBufferRef::U32(buf) => downmix!(buf),
        AudioBufferRef::S8(buf) => downmix!(buf),
        AudioBufferRef::S16(buf) => downmix!(buf),
        AudioBufferRef::S24(buf) => downmix!(buf),
        AudioBufferRef::S32(buf) => downmix!(buf),
        AudioBufferRef::F32(buf) => downmix!(buf),
        AudioBufferRef::F64(buf) => downmix!(buf),
    }
}

/// One-shot resample of a full buffer using the existing streaming
/// [`FrameResampler`], flushed at the end so no tail samples are dropped.
fn resample_to_16k_mono(samples: &[f32], source_hz: u32) -> Vec<f32> {
    if source_hz == WHISPER_SAMPLE_RATE {
        return samples.to_vec();
    }
    // Frame size only affects internal buffering granularity here, so any
    // small duration works; the resampler still emits every sample via push+finish.
    let mut resampler = FrameResampler::new(
        source_hz as usize,
        WHISPER_SAMPLE_RATE as usize,
        Duration::from_millis(30),
    );
    let mut out =
        Vec::with_capacity(samples.len() * WHISPER_SAMPLE_RATE as usize / source_hz as usize);
    resampler.push(samples, |frame| out.extend_from_slice(frame));
    resampler.finish(|frame| out.extend_from_slice(frame));
    out
}

/// Result of a successful import, ready to be saved as a history entry.
pub struct ImportedTranscript {
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    pub duration_secs: f64,
    /// 16 kHz mono PCM, so the caller can persist exactly what was
    /// transcribed without re-decoding the source file.
    pub samples: Vec<f32>,
}

/// Decode, resample, and transcribe `path` in chunks, emitting progress
/// events and checking for cancellation between chunks. Blocking — callers
/// must run this on a background thread (e.g. `spawn_blocking`).
pub fn run_import(
    app: &AppHandle,
    transcription_manager: &Arc<TranscriptionManager>,
    manager: &FileImportManager,
    path: &Path,
) -> Result<ImportedTranscript> {
    let guard = manager
        .try_start()
        .ok_or_else(|| anyhow!("An import is already in progress"))?;

    if !is_supported_file(path) {
        return Err(anyhow!(
            "Unsupported file type: {}",
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
        ));
    }

    let (samples, duration_secs) = decode_and_resample(path)?;

    let chunk_samples = (CHUNK_DURATION_SECS * WHISPER_SAMPLE_RATE as f64) as usize;
    let total_chunks = samples.len().div_ceil(chunk_samples).max(1);

    let mut text_parts: Vec<String> = Vec::new();
    let mut segments: Vec<TranscriptSegment> = Vec::new();

    for (chunk_index, chunk) in samples.chunks(chunk_samples.max(1)).enumerate() {
        if guard.cancel_flag.load(Ordering::SeqCst) {
            return Err(anyhow!("Import cancelled"));
        }

        let _ = (FileImportProgressEvent {
            current_chunk: chunk_index as u32,
            total_chunks: total_chunks as u32,
        })
        .emit(app);

        let chunk_text = transcription_manager
            .transcribe(chunk.to_vec())
            .map_err(|e| anyhow!("Transcription failed: {e}"))?;

        let start_ms = (chunk_index * chunk_samples) as u64 * 1000 / WHISPER_SAMPLE_RATE as u64;
        let end_ms = start_ms + (chunk.len() as u64 * 1000 / WHISPER_SAMPLE_RATE as u64).max(1);

        if !chunk_text.trim().is_empty() {
            text_parts.push(chunk_text.clone());
        }
        segments.push(TranscriptSegment {
            start_ms,
            end_ms,
            text: chunk_text,
            speaker: None,
        });

        if guard.cancel_flag.load(Ordering::SeqCst) {
            return Err(anyhow!("Import cancelled"));
        }
    }

    Ok(ImportedTranscript {
        text: text_parts.join(" "),
        segments,
        duration_secs,
        samples,
    })
}

/// Full import pipeline: decode/transcribe `path`, copy it into the
/// recordings directory as a WAV, and save the result as a history entry
/// with its per-chunk segments. When `diarization_enabled` is set and the
/// diarization models are downloaded, each segment is labeled with its
/// speaker before saving.
pub fn import_and_save(
    app: &AppHandle,
    transcription_manager: &Arc<TranscriptionManager>,
    history_manager: &Arc<HistoryManager>,
    manager: &FileImportManager,
    diarization_model_manager: &crate::diarization::models::DiarizationModelManager,
    path: &Path,
) -> Result<crate::managers::history::HistoryEntry> {
    let mut imported = run_import(app, transcription_manager, manager, path)?;

    let settings = crate::settings::get_settings(app);
    if settings.diarization_enabled && diarization_model_manager.is_ready() {
        match crate::diarization::pipeline::diarize(
            diarization_model_manager,
            &imported.samples,
            settings.diarization_cluster_threshold,
        ) {
            Ok(diarized) if !diarized.is_empty() => {
                for segment in imported.segments.iter_mut() {
                    segment.speaker = crate::diarization::pipeline::speaker_for_segment(
                        &diarized,
                        segment.start_ms,
                        segment.end_ms,
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("Diarization failed, saving import without speaker labels: {e}");
            }
        }
    }

    let file_name = format!("handy-import-{}.wav", chrono::Utc::now().timestamp());
    let dest_path = history_manager.recordings_dir().join(&file_name);
    crate::audio_toolkit::save_wav_file(&dest_path, &imported.samples)
        .context("Failed to save imported audio as WAV")?;

    history_manager
        .save_entry_with_segments(
            file_name,
            imported.text,
            false,
            None,
            None,
            Some(imported.duration_secs),
            &imported.segments,
        )
        .map_err(|e| anyhow!("Failed to save history entry: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path, sample_rate: u32, secs: f64) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        let n = (sample_rate as f64 * secs) as usize;
        for i in 0..n {
            let sample = ((i as f32 / sample_rate as f32 * 440.0 * std::f32::consts::TAU).sin()
                * 1000.0) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn supported_extensions_are_case_insensitive() {
        assert!(is_supported_file(Path::new("clip.wav")));
        assert!(is_supported_file(Path::new("clip.WAV")));
        assert!(is_supported_file(Path::new("clip.Mp3")));
        assert!(is_supported_file(Path::new("clip.m4a")));
        assert!(is_supported_file(Path::new("clip.mp4")));
        assert!(is_supported_file(Path::new("clip.mov")));
    }

    #[test]
    fn unsupported_extensions_are_rejected() {
        assert!(!is_supported_file(Path::new("clip.webm")));
        assert!(!is_supported_file(Path::new("clip.mkv")));
        assert!(!is_supported_file(Path::new("clip.txt")));
        assert!(!is_supported_file(Path::new("clip")));
    }

    #[test]
    fn decode_wav_at_target_rate_returns_expected_duration() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("handy_test_{}.wav", std::process::id()));
        write_test_wav(&path, 16000, 1.0);

        let (samples, duration) = decode_and_resample(&path).expect("decode should succeed");
        std::fs::remove_file(&path).ok();

        assert!((duration - 1.0).abs() < 0.01);
        assert!(!samples.is_empty());
    }

    #[test]
    fn decode_wav_resamples_to_16k() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("handy_test_resample_{}.wav", std::process::id()));
        write_test_wav(&path, 48000, 0.5);

        let (samples, duration) = decode_and_resample(&path).expect("decode should succeed");
        std::fs::remove_file(&path).ok();

        assert!((duration - 0.5).abs() < 0.01);
        // ~0.5s at 16kHz => ~8000 samples, allow resampler edge slack.
        assert!(
            samples.len() > 7000 && samples.len() < 9500,
            "unexpected resampled length: {}",
            samples.len()
        );
    }

    #[test]
    fn decode_rejects_missing_file() {
        let result = decode_and_resample(Path::new("/no/such/file.wav"));
        assert!(result.is_err());
    }

    #[test]
    fn import_manager_rejects_concurrent_imports() {
        let manager = FileImportManager::new();
        let guard1 = manager.try_start();
        assert!(guard1.is_some());

        let guard2 = manager.try_start();
        assert!(
            guard2.is_none(),
            "second concurrent import should be rejected"
        );

        drop(guard1);
        let guard3 = manager.try_start();
        assert!(
            guard3.is_some(),
            "slot should be free after guard is dropped"
        );
    }

    #[test]
    fn cancel_sets_flag_seen_by_active_guard() {
        let manager = FileImportManager::new();
        let guard = manager.try_start().expect("should claim slot");
        assert!(!guard.cancel_flag.load(Ordering::SeqCst));

        manager.cancel();
        assert!(guard.cancel_flag.load(Ordering::SeqCst));
    }
}
