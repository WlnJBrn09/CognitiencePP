//! Export presentation to json / txt.

use serde::Deserialize;

use crate::documents::Slide;

#[derive(Debug, Deserialize)]
pub struct ExportBody {
    pub format: String,
    pub title: Option<String>,
    pub slides: Vec<Slide>,
    pub active_slide: Option<usize>,
}

#[derive(Debug)]
pub struct ExportFile {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ExportError {
    Unsupported,
    Other(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "unsupported export format"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

pub fn export_document(body: &ExportBody) -> Result<ExportFile, ExportError> {
    let title = sanitize_filename(body.title.as_deref().unwrap_or("presentation"));
    match body.format.to_lowercase().as_str() {
        "json" | "cog" => {
            let json = serde_json::to_vec_pretty(&serde_json::json!({
                "title": body.title,
                "slides": body.slides,
                "active_slide": body.active_slide.unwrap_or(0),
            }))
            .map_err(|e| ExportError::Other(e.to_string()))?;
            Ok(ExportFile {
                filename: format!("{title}.json"),
                content_type: "application/json".into(),
                bytes: json,
            })
        }
        "txt" => {
            let mut plain = String::new();
            for (i, slide) in body.slides.iter().enumerate() {
                plain.push_str(&format!("--- Slide {} ---\n", i + 1));
                for el in &slide.elements {
                    if !el.text.trim().is_empty() {
                        plain.push_str(&el.text);
                        plain.push('\n');
                    }
                }
                plain.push('\n');
            }
            Ok(ExportFile {
                filename: format!("{title}.txt"),
                content_type: "text/plain; charset=utf-8".into(),
                bytes: plain.into_bytes(),
            })
        }
        _ => Err(ExportError::Unsupported),
    }
}

fn sanitize_filename(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let t = t.trim().trim_matches('.');
    if t.is_empty() {
        "presentation".into()
    } else {
        t.chars().take(80).collect()
    }
}
