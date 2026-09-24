//! Audio capture for live subtitles: microphone (cross-platform, via cpal) or
//! system loopback audio (Windows only, via raw WASAPI). Both paths resample
//! to 16 kHz mono and hand frames to a callback.
//!
//! The system-audio loopback approach is adapted from wxkingstar/TransEcho
//! (`audio/capture_windows.rs`, MIT licensed): open the default render
//! endpoint in shared mode with `AUDCLNT_STREAMFLAGS_LOOPBACK` so it captures
//! what's currently playing instead of a microphone.

use crate::audio_toolkit::audio::FrameResampler;
use crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::time::Duration;

/// Frame size handed to the caller's callback, matching the 30ms Silero frame
/// used elsewhere in the app.
const CAPTURE_FRAME_DURATION: Duration = Duration::from_millis(30);

/// A running capture stream. Dropping it stops capture.
pub struct CaptureStream {
    _cpal_stream: Option<cpal::Stream>,
    #[cfg(target_os = "windows")]
    _loopback: Option<windows_loopback::LoopbackHandle>,
}

/// Starts capturing from the system microphone (default input device) and
/// invokes `on_frame` with 16 kHz mono f32 frames from a background thread.
pub fn start_microphone_capture(
    mut on_frame: impl FnMut(&[f32]) + Send + 'static,
) -> Result<CaptureStream, String> {
    let host = crate::audio_toolkit::get_cpal_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No default microphone device available".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get default input config: {e}"))?;

    let in_hz = config.sample_rate().0 as usize;
    let channels = config.channels() as usize;
    let mut resampler = FrameResampler::new(in_hz, WHISPER_SAMPLE_RATE as usize, CAPTURE_FRAME_DURATION);

    let err_fn = |err| log::error!("Live subtitles microphone stream error: {err}");
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                let mono = downmix(data, channels);
                resampler.push(&mono, |frame| on_frame(frame));
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                let converted: Vec<f32> = data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                let mono = downmix(&converted, channels);
                resampler.push(&mono, |frame| on_frame(frame));
            },
            err_fn,
            None,
        ),
        other => return Err(format!("Unsupported microphone sample format: {other:?}")),
    }
    .map_err(|e| format!("Failed to build microphone input stream: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start microphone stream: {e}"))?;

    Ok(CaptureStream {
        _cpal_stream: Some(stream),
        #[cfg(target_os = "windows")]
        _loopback: None,
    })
}

fn downmix(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Starts capturing system audio (loopback of what's currently playing).
/// Only implemented on Windows (WASAPI loopback); other platforms return a
/// clear "not supported yet" error.
#[cfg(target_os = "windows")]
pub fn start_system_audio_capture(
    on_frame: impl FnMut(&[f32]) + Send + 'static,
) -> Result<CaptureStream, String> {
    let handle = windows_loopback::start(on_frame)?;
    Ok(CaptureStream {
        _cpal_stream: None,
        _loopback: Some(handle),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn start_system_audio_capture(
    _on_frame: impl FnMut(&[f32]) + Send + 'static,
) -> Result<CaptureStream, String> {
    Err("System audio capture is not supported yet on this platform. \
Use the microphone source instead."
        .to_string())
}

#[cfg(target_os = "windows")]
mod windows_loopback {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    /// Owns the capture thread; dropping stops it.
    pub struct LoopbackHandle {
        stop_flag: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for LoopbackHandle {
        fn drop(&mut self) {
            self.stop_flag.store(true, Ordering::SeqCst);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn start(
        mut on_frame: impl FnMut(&[f32]) + Send + 'static,
    ) -> Result<LoopbackHandle, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_stop_flag = Arc::clone(&stop_flag);

        // WASAPI errors surface from inside the capture thread (after COM is
        // initialized on it), so the setup error is relayed back via a
        // one-shot channel before we hand back a handle.
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        let thread = std::thread::spawn(move || {
            if let Err(e) = run_capture_loop(thread_stop_flag, &mut on_frame, ready_tx) {
                log::error!("Live subtitles system audio capture stopped: {e}");
            }
        });

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(LoopbackHandle {
                stop_flag,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                let _ = thread.join();
                Err("System audio capture thread exited before initializing".to_string())
            }
        }
    }

    fn run_capture_loop(
        stop_flag: Arc<AtomicBool>,
        on_frame: &mut dyn FnMut(&[f32]),
        ready_tx: mpsc::Sender<Result<(), String>>,
    ) -> Result<(), String> {
        // SAFETY: this thread owns its own COM apartment for its entire life;
        // it is uninitialized in the `Drop` path below.
        unsafe {
            if let Err(e) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                let msg = format!("CoInitializeEx failed: {e}");
                let _ = ready_tx.send(Err(msg.clone()));
                return Err(msg);
            }
        }

        let result = (|| -> Result<(), String> {
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                        .map_err(|e| format!("Failed to create device enumerator: {e}"))?;
                let device = enumerator
                    .GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|e| format!("Failed to get default render endpoint: {e}"))?;
                let audio_client: IAudioClient = device
                    .Activate(CLSCTX_ALL, None)
                    .map_err(|e| format!("Failed to activate audio client: {e}"))?;

                let mix_format = audio_client
                    .GetMixFormat()
                    .map_err(|e| format!("Failed to get mix format: {e}"))?;
                let format = &*mix_format;
                let in_hz = format.nSamplesPerSec as usize;
                let channels = format.nChannels as usize;
                let is_float = is_float_format(format);

                // 200ms buffer, shared mode, loopback flag captures the render
                // endpoint's output instead of recording a microphone.
                const REFTIMES_PER_SEC: i64 = 10_000_000;
                let buffer_duration = REFTIMES_PER_SEC / 5;
                audio_client
                    .Initialize(
                        AUDCLNT_SHAREMODE_SHARED,
                        AUDCLNT_STREAMFLAGS_LOOPBACK,
                        buffer_duration,
                        0,
                        mix_format,
                        None,
                    )
                    .map_err(|e| format!("Failed to initialize loopback audio client: {e}"))?;

                let capture_client: IAudioCaptureClient = audio_client
                    .GetService()
                    .map_err(|e| format!("Failed to get capture client: {e}"))?;

                audio_client
                    .Start()
                    .map_err(|e| format!("Failed to start loopback capture: {e}"))?;

                // Setup succeeded: unblock the caller now, then run the poll
                // loop until stopped.
                let _ = ready_tx.send(Ok(()));

                let mut resampler = FrameResampler::new(
                    in_hz,
                    WHISPER_SAMPLE_RATE as usize,
                    CAPTURE_FRAME_DURATION,
                );

                while !stop_flag.load(Ordering::SeqCst) {
                    let packet_size = capture_client
                        .GetNextPacketSize()
                        .map_err(|e| format!("GetNextPacketSize failed: {e}"))?;

                    if packet_size == 0 {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    let mut buffer_ptr = std::ptr::null_mut();
                    let mut frames_available = 0u32;
                    let mut flags = 0u32;
                    capture_client
                        .GetBuffer(
                            &mut buffer_ptr,
                            &mut frames_available,
                            &mut flags,
                            None,
                            None,
                        )
                        .map_err(|e| format!("GetBuffer failed: {e}"))?;

                    let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                    let sample_count = frames_available as usize * channels;

                    let mono: Vec<f32> = if silent || buffer_ptr.is_null() {
                        vec![0.0; frames_available as usize]
                    } else if is_float {
                        let samples =
                            std::slice::from_raw_parts(buffer_ptr as *const f32, sample_count);
                        downmix(samples, channels)
                    } else {
                        let samples =
                            std::slice::from_raw_parts(buffer_ptr as *const i16, sample_count);
                        let converted: Vec<f32> =
                            samples.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                        downmix(&converted, channels)
                    };

                    capture_client
                        .ReleaseBuffer(frames_available)
                        .map_err(|e| format!("ReleaseBuffer failed: {e}"))?;

                    resampler.push(&mono, |frame| on_frame(frame));
                }

                audio_client
                    .Stop()
                    .map_err(|e| format!("Failed to stop loopback capture: {e}"))?;

                Ok(())
            }
        })();

        unsafe {
            CoUninitialize();
        }

        if let Err(ref e) = result {
            // If setup already succeeded, ready_tx was consumed; log instead.
            log::error!("Live subtitles loopback capture error: {e}");
        }

        result
    }

    /// WAVEFORMATEX.wFormatTag / WAVE_FORMAT_EXTENSIBLE subformat check for
    /// IEEE float samples, which is what WASAPI mix formats normally use.
    fn is_float_format(format: &windows::Win32::Media::Audio::WAVEFORMATEX) -> bool {
        const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
        const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

        if format.wFormatTag == WAVE_FORMAT_IEEE_FLOAT {
            return true;
        }
        if format.wFormatTag == WAVE_FORMAT_EXTENSIBLE && format.cbSize >= 22 {
            // The extensible struct's SubFormat GUID starts right after the
            // fixed WAVEFORMATEX fields; the first 2 bytes of the GUID equal
            // the format tag for PCM (1) / IEEE float (3) subformats.
            let ext = format as *const _ as *const u8;
            unsafe {
                let sub_format_tag =
                    u16::from_le_bytes([*ext.add(18 + 4), *ext.add(18 + 5)]);
                return sub_format_tag == WAVE_FORMAT_IEEE_FLOAT;
            }
        }
        false
    }
}
