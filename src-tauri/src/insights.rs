//! Pure computation of dictation insights (word counts, WPM, streak) from
//! history entries. Kept free of I/O and Tauri types so it is easy to unit
//! test; callers (see `commands::insights`) fetch the raw data from
//! `HistoryManager` and pass it here.

use chrono::{Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Minimal view of a history entry needed to compute insights.
#[derive(Clone, Debug)]
pub struct InsightsEntry {
    /// Unix timestamp (seconds, UTC) the entry was recorded at.
    pub timestamp: i64,
    /// Final transcribed text (post-processed text when present).
    pub text: String,
    /// Recording duration in seconds, if known.
    pub duration_seconds: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Insights {
    pub total_words: u64,
    pub words_today: u64,
    pub average_wpm: f64,
    pub day_streak: u32,
}

fn word_count(text: &str) -> u64 {
    text.split_whitespace().count() as u64
}

fn local_date(timestamp: i64) -> Option<NaiveDate> {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.date_naive())
}

/// Compute insights from history entries as of `today` (local date).
/// Pure function; `today` is injected so tests don't depend on wall-clock time.
pub fn compute_insights(entries: &[InsightsEntry], today: NaiveDate) -> Insights {
    let total_words: u64 = entries.iter().map(|e| word_count(&e.text)).sum();

    let words_today: u64 = entries
        .iter()
        .filter(|e| local_date(e.timestamp) == Some(today))
        .map(|e| word_count(&e.text))
        .sum();

    let (word_sum, seconds_sum) = entries
        .iter()
        .filter_map(|e| e.duration_seconds.map(|d| (word_count(&e.text), d)))
        .filter(|(_, d)| *d > 0.0)
        .fold((0u64, 0.0f64), |(w_acc, s_acc), (w, s)| {
            (w_acc + w, s_acc + s)
        });
    let average_wpm = if seconds_sum > 0.0 {
        (word_sum as f64) / (seconds_sum / 60.0)
    } else {
        0.0
    };

    let mut days: Vec<NaiveDate> = entries
        .iter()
        .filter_map(|e| local_date(e.timestamp))
        .collect();
    days.sort_unstable();
    days.dedup();

    let day_streak = compute_day_streak(&days, today);

    Insights {
        total_words,
        words_today,
        average_wpm,
        day_streak,
    }
}

/// `days` must be sorted ascending and deduplicated. Counts consecutive days
/// ending at `today` or `today - 1` (yesterday); any other gap breaks the streak.
fn compute_day_streak(days: &[NaiveDate], today: NaiveDate) -> u32 {
    let Some(&last_day) = days.last() else {
        return 0;
    };

    let yesterday = today - chrono::Duration::days(1);
    if last_day != today && last_day != yesterday {
        return 0;
    }

    let mut streak = 1u32;
    let mut expected = last_day - chrono::Duration::days(1);
    for day in days.iter().rev().skip(1) {
        if *day == expected {
            streak += 1;
            expected -= chrono::Duration::days(1);
        } else {
            break;
        }
    }
    streak
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(timestamp: i64, text: &str, duration_seconds: Option<f64>) -> InsightsEntry {
        InsightsEntry {
            timestamp,
            text: text.to_string(),
            duration_seconds,
        }
    }

    fn ts_for(date: NaiveDate, hour: u32) -> i64 {
        let dt = date.and_hms_opt(hour, 0, 0).unwrap();
        Local.from_local_datetime(&dt).unwrap().timestamp()
    }

    #[test]
    fn empty_history_returns_zeros() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let insights = compute_insights(&[], today);
        assert_eq!(insights.total_words, 0);
        assert_eq!(insights.words_today, 0);
        assert_eq!(insights.average_wpm, 0.0);
        assert_eq!(insights.day_streak, 0);
    }

    #[test]
    fn streak_broken_by_gap() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let day_minus_5 = today - chrono::Duration::days(5);
        let entries = vec![
            entry(ts_for(day_minus_5, 9), "old entry here", None),
            entry(ts_for(today, 9), "recent entry here", None),
        ];
        let insights = compute_insights(&entries, today);
        // Only today has an entry contiguous with "today"; the gap 5 days
        // back does not extend the streak.
        assert_eq!(insights.day_streak, 1);
    }

    #[test]
    fn streak_across_consecutive_days_including_yesterday() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let yesterday = today - chrono::Duration::days(1);
        let two_days_ago = today - chrono::Duration::days(2);
        let entries = vec![
            entry(ts_for(two_days_ago, 8), "two days ago words here", None),
            entry(ts_for(yesterday, 8), "yesterday words here", None),
        ];
        // No entry today yet, but yesterday continues the streak.
        let insights = compute_insights(&entries, today);
        assert_eq!(insights.day_streak, 2);
    }

    #[test]
    fn streak_zero_when_last_entry_older_than_yesterday() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let three_days_ago = today - chrono::Duration::days(3);
        let entries = vec![entry(ts_for(three_days_ago, 8), "stale entry", None)];
        let insights = compute_insights(&entries, today);
        assert_eq!(insights.day_streak, 0);
    }

    #[test]
    fn wpm_excludes_entries_with_missing_duration() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let entries = vec![
            // 10 words in 30 seconds -> 20 wpm, included.
            entry(
                ts_for(today, 9),
                "one two three four five six seven eight nine ten",
                Some(30.0),
            ),
            // No duration -> excluded from WPM but still counted in total_words.
            entry(ts_for(today, 10), "eleven twelve thirteen", None),
        ];
        let insights = compute_insights(&entries, today);
        assert_eq!(insights.total_words, 13);
        assert_eq!(insights.average_wpm, 20.0);
    }

    #[test]
    fn words_today_only_counts_todays_entries() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let yesterday = today - chrono::Duration::days(1);
        let entries = vec![
            entry(ts_for(today, 9), "today has three words", None),
            entry(ts_for(yesterday, 9), "yesterday two words here", None),
        ];
        let insights = compute_insights(&entries, today);
        assert_eq!(insights.words_today, 4);
        assert_eq!(insights.total_words, 8);
    }
}
