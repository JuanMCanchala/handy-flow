//! Per-app tone styling: classifies the foreground app into a category
//! (Personal / Work / Email / Other) and maps a user-selected tone to a
//! prompt fragment appended to the post-processing system prompt.

/// Category the foreground app is classified into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleCategory {
    Personal,
    Work,
    Email,
    Other,
}

impl StyleCategory {
    /// Stable key used to store/read the per-category tone in settings.
    pub fn as_key(&self) -> &'static str {
        match self {
            StyleCategory::Personal => "personal",
            StyleCategory::Work => "work",
            StyleCategory::Email => "email",
            StyleCategory::Other => "other",
        }
    }
}

/// User-selected tone for a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleTone {
    Formal,
    Casual,
    VeryCasual,
    Excited,
}

impl StyleTone {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "formal" => Some(StyleTone::Formal),
            "casual" => Some(StyleTone::Casual),
            "very_casual" => Some(StyleTone::VeryCasual),
            "excited" => Some(StyleTone::Excited),
            _ => None,
        }
    }

    /// Prompt fragment describing this tone, appended to the system prompt.
    pub fn instruction(&self) -> &'static str {
        match self {
            StyleTone::Formal => {
                "Rewrite the text in a formal, professional tone. Use complete sentences and correct grammar, avoid slang and contractions."
            }
            StyleTone::Casual => {
                "Rewrite the text in a casual, friendly tone, as if talking to a colleague or friend. Contractions are fine."
            }
            StyleTone::VeryCasual => {
                "Rewrite the text in a very casual, relaxed tone, like a quick text to a close friend. Short sentences, slang and abbreviations are fine."
            }
            StyleTone::Excited => {
                "Rewrite the text in an upbeat, enthusiastic tone. Convey energy and excitement while keeping it natural."
            }
        }
    }
}

/// Process name + window title of the foreground app, as detected by the
/// platform-specific backends below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundApp {
    pub process_name: String,
    pub window_title: String,
}

/// Case-insensitive substring match helper.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

const PERSONAL_PROCESSES: &[&str] = &["whatsapp", "telegram", "messenger", "signal", "discord"];

const WORK_PROCESSES: &[&str] = &["slack", "teams", "ms-teams", "msteams"];

const EMAIL_PROCESSES: &[&str] = &["outlook", "thunderbird", "mail"];

// Browser process names used to check the window title for webmail.
const BROWSER_PROCESSES: &[&str] = &[
    "chrome", "msedge", "firefox", "safari", "brave", "opera", "vivaldi", "arc",
];

const EMAIL_TITLE_HINTS: &[&str] = &["gmail", "outlook"];

/// Classify a foreground app (process name + window title) into a
/// [`StyleCategory`]. Pure function: no I/O, easy to unit test.
pub fn classify(process_name: &str, window_title: &str) -> StyleCategory {
    let process_lower = process_name.to_lowercase();

    if PERSONAL_PROCESSES
        .iter()
        .any(|p| contains_ci(&process_lower, p))
    {
        return StyleCategory::Personal;
    }

    if WORK_PROCESSES
        .iter()
        .any(|p| contains_ci(&process_lower, p))
    {
        return StyleCategory::Work;
    }

    if EMAIL_PROCESSES
        .iter()
        .any(|p| contains_ci(&process_lower, p))
    {
        return StyleCategory::Email;
    }

    if BROWSER_PROCESSES
        .iter()
        .any(|p| contains_ci(&process_lower, p))
        && EMAIL_TITLE_HINTS
            .iter()
            .any(|hint| contains_ci(window_title, hint))
    {
        return StyleCategory::Email;
    }

    StyleCategory::Other
}

/// Build the style instruction to append to the post-processing system
/// prompt for the given category, using the tone the user selected for it
/// (looked up by `StyleCategory::as_key()` in the caller's settings map).
/// Returns `None` when no tone is configured for the category.
pub fn instruction_for(tone_key: Option<&str>) -> Option<&'static str> {
    tone_key
        .and_then(StyleTone::from_key)
        .map(|t| t.instruction())
}

#[cfg(target_os = "windows")]
mod platform {
    use super::ForegroundApp;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    pub fn detect_foreground_app() -> Option<ForegroundApp> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let window_title = String::from_utf16_lossy(&title_buf[..len.max(0) as usize]);

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return Some(ForegroundApp {
                    process_name: String::new(),
                    window_title,
                });
            }

            let process_name = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(handle) => {
                    let mut buf = [0u16; MAX_PATH as usize];
                    let mut size = buf.len() as u32;
                    let name = if QueryFullProcessImageNameW(
                        handle,
                        PROCESS_NAME_WIN32,
                        windows::core::PWSTR(buf.as_mut_ptr()),
                        &mut size,
                    )
                    .is_ok()
                    {
                        String::from_utf16_lossy(&buf[..size as usize])
                    } else {
                        String::new()
                    };
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
                    name
                }
                Err(_) => String::new(),
            };

            Some(ForegroundApp {
                process_name,
                window_title,
            })
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::ForegroundApp;
    use objc2_app_kit::NSWorkspace;

    pub fn detect_foreground_app() -> Option<ForegroundApp> {
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace.frontmostApplication()?;
        let process_name = app
            .localizedName()
            .map(|s| s.to_string())
            .unwrap_or_default();

        // AppKit does not expose the frontmost window title without the
        // Accessibility API; window-title based email detection (Gmail tab
        // title in a browser) is best-effort only on macOS.
        Some(ForegroundApp {
            process_name,
            window_title: String::new(),
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    use super::ForegroundApp;

    /// Best-effort: no portable foreground-window API on Linux (varies by
    /// compositor/display server). Returns `None`, which callers treat as
    /// [`StyleCategory::Other`].
    pub fn detect_foreground_app() -> Option<ForegroundApp> {
        None
    }
}

pub use platform::detect_foreground_app;

/// Classify the current foreground app, falling back to `Other` when
/// detection is unavailable (e.g. Linux best-effort, or no window focused).
pub fn classify_foreground_app() -> StyleCategory {
    match detect_foreground_app() {
        Some(app) => classify(&app.process_name, &app.window_title),
        None => StyleCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_personal_messaging_apps() {
        assert_eq!(
            classify("WhatsApp.exe", "WhatsApp"),
            StyleCategory::Personal
        );
        assert_eq!(classify("Telegram", "Telegram"), StyleCategory::Personal);
        assert_eq!(classify("Messenger", ""), StyleCategory::Personal);
        assert_eq!(classify("Signal", ""), StyleCategory::Personal);
        assert_eq!(classify("Discord.exe", "general"), StyleCategory::Personal);
    }

    #[test]
    fn classifies_work_messaging_apps() {
        assert_eq!(classify("slack.exe", "Slack"), StyleCategory::Work);
        assert_eq!(
            classify("Teams.exe", "Microsoft Teams"),
            StyleCategory::Work
        );
    }

    #[test]
    fn classifies_email_clients() {
        assert_eq!(classify("OUTLOOK.EXE", "Inbox"), StyleCategory::Email);
        assert_eq!(classify("thunderbird", "Inbox"), StyleCategory::Email);
        assert_eq!(classify("Mail", "Inbox"), StyleCategory::Email);
    }

    #[test]
    fn classifies_webmail_in_browser_by_title() {
        assert_eq!(
            classify("chrome.exe", "Inbox - Gmail"),
            StyleCategory::Email
        );
        assert_eq!(classify("firefox", "Mail - Outlook"), StyleCategory::Email);
    }

    #[test]
    fn browser_without_email_title_is_other() {
        assert_eq!(
            classify("chrome.exe", "GitHub - Pull Requests"),
            StyleCategory::Other
        );
    }

    #[test]
    fn unknown_app_is_other() {
        assert_eq!(classify("notepad.exe", "Untitled"), StyleCategory::Other);
        assert_eq!(classify("", ""), StyleCategory::Other);
    }

    #[test]
    fn classification_is_case_insensitive() {
        assert_eq!(classify("WHATSAPP", ""), StyleCategory::Personal);
        assert_eq!(classify("whatsapp", ""), StyleCategory::Personal);
    }

    #[test]
    fn tone_from_key_round_trips() {
        assert_eq!(StyleTone::from_key("formal"), Some(StyleTone::Formal));
        assert_eq!(StyleTone::from_key("casual"), Some(StyleTone::Casual));
        assert_eq!(
            StyleTone::from_key("very_casual"),
            Some(StyleTone::VeryCasual)
        );
        assert_eq!(StyleTone::from_key("excited"), Some(StyleTone::Excited));
        assert_eq!(StyleTone::from_key("unknown"), None);
    }

    #[test]
    fn instruction_for_missing_tone_is_none() {
        assert_eq!(instruction_for(None), None);
        assert_eq!(instruction_for(Some("nope")), None);
    }

    #[test]
    fn instruction_for_known_tone_is_some() {
        assert!(instruction_for(Some("formal")).is_some());
    }

    #[test]
    fn category_keys_are_stable() {
        assert_eq!(StyleCategory::Personal.as_key(), "personal");
        assert_eq!(StyleCategory::Work.as_key(), "work");
        assert_eq!(StyleCategory::Email.as_key(), "email");
        assert_eq!(StyleCategory::Other.as_key(), "other");
    }
}
