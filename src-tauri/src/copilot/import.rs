//! Plain-text extraction for profile import (.txt/.md/.pdf/.docx).

use std::path::Path;

/// Extracts plain text from a profile file based on its extension.
/// `.txt`/`.md` are read as UTF-8 (lossy); `.pdf` and `.docx` are parsed.
pub fn extract_text(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "txt" | "md" => {
            let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        "pdf" => pdf_extract::extract_text(path).map_err(|e| format!("Failed to read PDF: {e}")),
        "docx" => extract_docx_text(path),
        other => Err(format!("Unsupported file type: .{other}")),
    }
}

fn extract_docx_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    let docx = docx_rs::read_docx(&bytes).map_err(|e| format!("Failed to parse DOCX: {e:?}"))?;

    let mut text = String::new();
    for child in docx.document.children {
        collect_docx_text(&child, &mut text);
    }
    Ok(text)
}

/// Recursively walks a document child, appending any run text found. Only
/// text content is extracted; formatting and structure are discarded.
fn collect_docx_text(child: &docx_rs::DocumentChild, out: &mut String) {
    if let docx_rs::DocumentChild::Paragraph(paragraph) = child {
        for paragraph_child in &paragraph.children {
            if let docx_rs::ParagraphChild::Run(run) = paragraph_child {
                for run_child in &run.children {
                    if let docx_rs::RunChild::Text(text) = run_child {
                        out.push_str(&text.text);
                    }
                }
            }
        }
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_extension_is_rejected() {
        let result = extract_text(Path::new("profile.exe"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported file type"));
    }

    #[test]
    fn txt_file_is_read_as_utf8() {
        let dir = std::env::temp_dir();
        let path = dir.join("handy_copilot_test_profile.txt");
        std::fs::write(&path, "Hello profile").unwrap();

        let result = extract_text(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(result.unwrap(), "Hello profile");
    }
}
