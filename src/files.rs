//! User Documents folder listing + open/import for pdf, png, pptx (and presentation json).

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::ZipArchive;

use crate::documents::{Slide, SlideElement};

/// Extensions shown in the Documents sidebar.
const LIST_EXT: &[&str] = &["pdf", "png", "pptx"];
/// Also openable via Open/Import.
const OPEN_EXT: &[&str] = &[
    "pdf", "png", "pptx", "json", "cog", "jpg", "jpeg", "webp", "gif", "svg",
];

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub ext: String,
    pub size: u64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenedFile {
    pub name: String,
    pub path: String,
    pub ext: String,
    pub title: String,
    pub format: String,
    /// Presentation slides (pptx import or empty for pure media).
    #[serde(default)]
    pub slides: Vec<Slide>,
    #[serde(default)]
    pub active_slide: usize,
    /// When true, frontend should open `view_url` / binary instead of editable body.
    #[serde(default)]
    pub binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenPathBody {
    pub path: String,
}

#[derive(Debug)]
pub enum FileError {
    NotFound,
    InvalidPath,
    Unsupported,
    Io(std::io::Error),
    Other(String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "file not found"),
            Self::InvalidPath => write!(f, "invalid path"),
            Self::Unsupported => write!(f, "unsupported file type"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

pub fn resolve_documents_dir() -> PathBuf {
    if let Ok(p) = std::env::var("COGNITION_DOCS_DIR") {
        return PathBuf::from(p);
    }
    dirs::document_dir().unwrap_or_else(|| {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("Documents")
    })
}

pub fn list_documents_folder(root: &Path) -> Result<Vec<FileEntry>, FileError> {
    if !root.exists() {
        fs::create_dir_all(root).map_err(FileError::Io)?;
    }
    let mut out = Vec::new();
    collect_files(root, root, 0, &mut out)?;
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

fn collect_files(
    root: &Path,
    dir: &Path,
    depth: u8,
    out: &mut Vec<FileEntry>,
) -> Result<(), FileError> {
    if depth > 3 {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(FileError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, depth + 1, out)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !LIST_EXT.contains(&ext.as_str()) {
            continue;
        }
        let meta = entry.metadata().map_err(FileError::Io)?;
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(FileEntry {
            name,
            path: rel,
            ext: ext.clone(),
            size: meta.len(),
            kind: kind_for_ext(&ext).into(),
        });
    }
    Ok(())
}

fn kind_for_ext(ext: &str) -> &'static str {
    match ext {
        "pdf" => "pdf",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => "image",
        "pptx" => "presentation",
        "json" | "cog" => "cognition",
        _ => "file",
    }
}

/// Resolve a relative path under Documents; reject path traversal.
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, FileError> {
    let rel = rel.trim().trim_start_matches(['/', '\\']);
    if rel.is_empty() {
        return Err(FileError::InvalidPath);
    }
    let candidate = root.join(rel);
    let canon_root = fs::canonicalize(root).map_err(FileError::Io)?;
    let full = if candidate.exists() {
        fs::canonicalize(&candidate).map_err(FileError::Io)?
    } else {
        let mut norm = PathBuf::new();
        for c in Path::new(rel).components() {
            match c {
                Component::Normal(s) => norm.push(s),
                Component::CurDir => {}
                _ => return Err(FileError::InvalidPath),
            }
        }
        if norm.as_os_str().is_empty() {
            return Err(FileError::InvalidPath);
        }
        return Ok(root.join(norm));
    };
    if !full.starts_with(&canon_root) {
        return Err(FileError::InvalidPath);
    }
    Ok(full)
}

pub fn open_file(root: &Path, rel: &str) -> Result<OpenedFile, FileError> {
    let path = safe_join(root, rel)?;
    if !path.is_file() {
        return Err(FileError::NotFound);
    }
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let title = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled presentation".into());

    let bytes = fs::read(&path).map_err(FileError::Io)?;
    open_bytes_with_options(&name, rel, &ext, &title, &bytes, true)
}

pub fn open_bytes(
    name: &str,
    rel: &str,
    ext: &str,
    title: &str,
    bytes: &[u8],
) -> Result<OpenedFile, FileError> {
    open_bytes_with_options(name, rel, ext, title, bytes, false)
}

pub fn open_bytes_with_options(
    name: &str,
    rel: &str,
    ext: &str,
    title: &str,
    bytes: &[u8],
    from_disk: bool,
) -> Result<OpenedFile, FileError> {
    if !OPEN_EXT.contains(&ext) {
        return Err(FileError::Unsupported);
    }
    match ext {
        "pdf" => binary_open(name, rel, ext, title, bytes, from_disk, "application/pdf", "pdf"),
        "png" => binary_open(name, rel, ext, title, bytes, from_disk, "image/png", "image"),
        "jpg" | "jpeg" => {
            binary_open(name, rel, ext, title, bytes, from_disk, "image/jpeg", "image")
        }
        "webp" => binary_open(name, rel, ext, title, bytes, from_disk, "image/webp", "image"),
        "gif" => binary_open(name, rel, ext, title, bytes, from_disk, "image/gif", "image"),
        "svg" => binary_open(name, rel, ext, title, bytes, from_disk, "image/svg+xml", "image"),
        "pptx" => open_pptx(name, rel, title, bytes, from_disk),
        "json" | "cog" => open_presentation_json(name, rel, ext, title, bytes),
        _ => Err(FileError::Unsupported),
    }
}

fn binary_open(
    name: &str,
    rel: &str,
    ext: &str,
    title: &str,
    bytes: &[u8],
    from_disk: bool,
    mime: &str,
    format: &str,
) -> Result<OpenedFile, FileError> {
    let view_url = if from_disk && !rel.is_empty() {
        Some(format!("/api/files/raw?path={}", urlencoding_encode(rel)))
    } else {
        None
    };
    let binary_base64 = if view_url.is_none() {
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(bytes))
    } else {
        None
    };
    // Single slide placeholder so the app has a canvas; media shown in viewer overlay.
    let slides = vec![media_cover_slide(title, format)];
    Ok(OpenedFile {
        name: name.into(),
        path: rel.into(),
        ext: ext.into(),
        title: title.into(),
        format: format.into(),
        slides,
        active_slide: 0,
        binary: true,
        view_url,
        binary_base64,
        mime: Some(mime.into()),
    })
}

fn el(
    kind: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    text: String,
    font_size: f64,
    align: &str,
    bold: Option<bool>,
) -> SlideElement {
    SlideElement {
        id: Uuid::new_v4().to_string(),
        kind: kind.into(),
        x,
        y,
        w,
        h,
        text,
        table: None,
        src: None,
        mime: None,
        font_size: Some(font_size),
        font_family: Some("Inter".into()),
        align: Some(align.into()),
        color: None,
        bold,
        italic: None,
        underline: None,
        strikethrough: None,
        highlight: None,
        vertical_align: None,
    }
}

fn media_cover_slide(title: &str, kind: &str) -> Slide {
    Slide {
        id: Uuid::new_v4().to_string(),
        background: Some("#ffffff".into()),
        elements: vec![
            el("title", 8.0, 36.0, 84.0, 18.0, title.into(), 32.0, "center", Some(true)),
            el(
                "subtitle",
                12.0,
                56.0,
                76.0,
                10.0,
                format!("{kind} file"),
                16.0,
                "center",
                None,
            ),
        ],
    }
}

fn open_pptx(
    name: &str,
    rel: &str,
    title: &str,
    bytes: &[u8],
    from_disk: bool,
) -> Result<OpenedFile, FileError> {
    let slides = parse_pptx_slides(bytes)?;
    let view_url = if from_disk && !rel.is_empty() {
        Some(format!("/api/files/raw?path={}", urlencoding_encode(rel)))
    } else {
        None
    };
    let binary_base64 = if view_url.is_none() {
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(bytes))
    } else {
        None
    };
    Ok(OpenedFile {
        name: name.into(),
        path: rel.into(),
        ext: "pptx".into(),
        title: title.into(),
        format: "presentation".into(),
        slides,
        active_slide: 0,
        binary: true,
        view_url,
        binary_base64,
        mime: Some(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation".into(),
        ),
    })
}

/// Extract text runs from each ppt/slides/slideN.xml into presentation slides.
fn parse_pptx_slides(bytes: &[u8]) -> Result<Vec<Slide>, FileError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| FileError::Other(e.to_string()))?;

    let mut slide_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| FileError::Other(e.to_string()))?;
        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") && !name.contains("_rels")
        {
            slide_names.push(name);
        }
    }
    slide_names.sort_by(|a, b| {
        let na = extract_slide_num(a);
        let nb = extract_slide_num(b);
        na.cmp(&nb)
    });

    if slide_names.is_empty() {
        return Ok(vec![crate::documents::default_title_slide()]);
    }

    let mut slides = Vec::new();
    for path in slide_names {
        let mut file = archive
            .by_name(&path)
            .map_err(|e| FileError::Other(e.to_string()))?;
        let mut xml = String::new();
        file.read_to_string(&mut xml)
            .map_err(|e| FileError::Other(e.to_string()))?;
        let texts = extract_a_t_texts(&xml);
        let mut elements = Vec::new();
        if let Some(first) = texts.first() {
            elements.push(el(
                "title",
                8.0,
                18.0,
                84.0,
                16.0,
                first.clone(),
                32.0,
                "left",
                Some(true),
            ));
        }
        if texts.len() > 1 {
            let body = texts[1..].join("\n");
            elements.push(el("text", 8.0, 40.0, 84.0, 50.0, body, 18.0, "left", None));
        }
        if elements.is_empty() {
            elements.push(el(
                "title",
                8.0,
                40.0,
                84.0,
                16.0,
                String::new(),
                28.0,
                "center",
                None,
            ));
        }
        slides.push(Slide {
            id: Uuid::new_v4().to_string(),
            elements,
            background: Some("#ffffff".into()),
        });
    }
    Ok(slides)
}

fn extract_slide_num(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(0)
}

fn extract_a_t_texts(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<a:t") {
        let after = &rest[start..];
        let content_start = match after.find('>') {
            Some(i) => i + 1,
            None => break,
        };
        let body = &after[content_start..];
        if let Some(end) = body.find("</a:t>") {
            let text = body[..end].trim();
            // decode basic entities
            let text = text
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'");
            if !text.is_empty() {
                out.push(text);
            }
            rest = &body[end + 6..];
        } else {
            break;
        }
    }
    out
}

fn open_presentation_json(
    name: &str,
    rel: &str,
    ext: &str,
    title: &str,
    bytes: &[u8],
) -> Result<OpenedFile, FileError> {
    #[derive(Deserialize)]
    struct Doc {
        title: Option<String>,
        slides: Option<Vec<Slide>>,
    }
    let doc: Doc = serde_json::from_slice(bytes)
        .map_err(|e| FileError::Other(format!("invalid presentation json: {e}")))?;
    let slides = doc
        .slides
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| vec![crate::documents::default_title_slide()]);
    Ok(OpenedFile {
        name: name.into(),
        path: rel.into(),
        ext: ext.into(),
        title: doc.title.unwrap_or_else(|| title.into()),
        format: "cognition".into(),
        slides,
        active_slide: 0,
        binary: false,
        view_url: None,
        binary_base64: None,
        mime: Some("application/json".into()),
    })
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "json" | "cog" => "application/json",
        _ => "application/octet-stream",
    }
}

pub fn read_raw_file(root: &Path, rel: &str) -> Result<(Vec<u8>, &'static str), FileError> {
    let path = safe_join(root, rel)?;
    if !path.is_file() {
        return Err(FileError::NotFound);
    }
    let data = fs::read(&path).map_err(FileError::Io)?;
    Ok((data, mime_for_path(&path)))
}
