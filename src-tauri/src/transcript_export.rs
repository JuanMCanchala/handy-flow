//! Pure formatters for exporting a transcript's segments to plain text,
//! SRT, or VTT. No I/O, no Tauri types — callers own persistence/dialogs.

/// A single timed span of transcript text, in milliseconds from the start
/// of the recording.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Txt,
    Srt,
    Vtt,
}

/// Render segments as plain text, one line per segment.
pub fn format_txt(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render segments as SRT (SubRip).
pub fn format_srt(segments: &[TranscriptSegment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&(i + 1).to_string());
        out.push('\n');
        out.push_str(&srt_timestamp(seg.start_ms));
        out.push_str(" --> ");
        out.push_str(&srt_timestamp(seg.end_ms));
        out.push('\n');
        out.push_str(&seg.text);
        out.push_str("\n\n");
    }
    out
}

/// Render segments as WebVTT.
pub fn format_vtt(segments: &[TranscriptSegment]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for seg in segments {
        out.push_str(&vtt_timestamp(seg.start_ms));
        out.push_str(" --> ");
        out.push_str(&vtt_timestamp(seg.end_ms));
        out.push('\n');
        out.push_str(&seg.text);
        out.push_str("\n\n");
    }
    out
}

/// Format a transcript in the given export format.
pub fn format_transcript(segments: &[TranscriptSegment], format: ExportFormat) -> String {
    match format {
        ExportFormat::Txt => format_txt(segments),
        ExportFormat::Srt => format_srt(segments),
        ExportFormat::Vtt => format_vtt(segments),
    }
}

/// `HH:MM:SS,mmm` as required by SRT.
fn srt_timestamp(ms: u64) -> String {
    let (h, m, s, ms) = split_ms(ms);
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

/// `HH:MM:SS.mmm` as required by WebVTT.
fn vtt_timestamp(ms: u64) -> String {
    let (h, m, s, ms) = split_ms(ms);
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

fn split_ms(total_ms: u64) -> (u64, u64, u64, u64) {
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    (h, m, s, ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    #[test]
    fn srt_timestamp_formats_hours_minutes_seconds_millis() {
        assert_eq!(srt_timestamp(0), "00:00:00,000");
        assert_eq!(srt_timestamp(1_234), "00:00:01,234");
        assert_eq!(srt_timestamp(61_005), "00:01:01,005");
        assert_eq!(srt_timestamp(3_661_500), "01:01:01,500");
    }

    #[test]
    fn vtt_timestamp_uses_dot_separator() {
        assert_eq!(vtt_timestamp(0), "00:00:00.000");
        assert_eq!(vtt_timestamp(3_661_500), "01:01:01.500");
    }

    #[test]
    fn ms_rounding_is_truncating_and_carries_correctly() {
        // 999ms should not round up into the seconds column.
        assert_eq!(srt_timestamp(999), "00:00:00,999");
        // Exactly one second boundary.
        assert_eq!(srt_timestamp(1_000), "00:00:01,000");
        // Exactly one hour boundary.
        assert_eq!(srt_timestamp(3_600_000), "01:00:00,000");
    }

    #[test]
    fn format_srt_multi_segment_multi_line_text() {
        let segments = vec![
            seg(0, 1_500, "Hello world"),
            seg(1_500, 4_000, "Line one\nLine two"),
        ];
        let out = format_srt(&segments);
        assert_eq!(
            out,
            "1\n00:00:00,000 --> 00:00:01,500\nHello world\n\n\
             2\n00:00:01,500 --> 00:00:04,000\nLine one\nLine two\n\n"
        );
    }

    #[test]
    fn format_vtt_multi_segment_multi_line_text() {
        let segments = vec![
            seg(0, 1_500, "Hello world"),
            seg(1_500, 4_000, "Line one\nLine two"),
        ];
        let out = format_vtt(&segments);
        assert_eq!(
            out,
            "WEBVTT\n\n\
             00:00:00.000 --> 00:00:01.500\nHello world\n\n\
             00:00:01.500 --> 00:00:04.000\nLine one\nLine two\n\n"
        );
    }

    #[test]
    fn format_txt_joins_segments_with_newline() {
        let segments = vec![seg(0, 1_000, "Hello"), seg(1_000, 2_000, "World")];
        assert_eq!(format_txt(&segments), "Hello\nWorld");
    }

    #[test]
    fn empty_segments_produce_empty_or_header_only_output() {
        assert_eq!(format_txt(&[]), "");
        assert_eq!(format_srt(&[]), "");
        assert_eq!(format_vtt(&[]), "WEBVTT\n\n");
    }

    #[test]
    fn format_transcript_dispatches_by_format() {
        let segments = vec![seg(0, 1_000, "Hi")];
        assert_eq!(
            format_transcript(&segments, ExportFormat::Txt),
            format_txt(&segments)
        );
        assert_eq!(
            format_transcript(&segments, ExportFormat::Srt),
            format_srt(&segments)
        );
        assert_eq!(
            format_transcript(&segments, ExportFormat::Vtt),
            format_vtt(&segments)
        );
    }
}
