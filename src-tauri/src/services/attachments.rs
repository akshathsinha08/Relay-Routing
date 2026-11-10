use std::path::{Path, PathBuf};

use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::database::attachments::NewAttachment;
use crate::database::models::Attachment;
use crate::database::new_id;
use crate::error::AppError;
use crate::providers::ImagePart;
use crate::security::paths::{ensure_within, sanitize_filename};
use crate::state::AppContext;

const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 512 * 1024;
const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;

fn looks_textual(mime: &str, extension: &str) -> bool {
    mime.starts_with("text/")
        || matches!(
            mime,
            "application/json"
                | "application/xml"
                | "application/x-yaml"
                | "application/yaml"
                | "application/javascript"
                | "application/typescript"
                | "application/toml"
                | "application/x-sh"
        )
        || matches!(
            extension,
            "md" | "markdown"
                | "txt"
                | "json"
                | "jsonl"
                | "csv"
                | "tsv"
                | "yaml"
                | "yml"
                | "toml"
                | "xml"
                | "html"
                | "css"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "py"
                | "rs"
                | "go"
                | "java"
                | "c"
                | "h"
                | "cpp"
                | "sh"
                | "zsh"
                | "sql"
                | "log"
                | "ini"
                | "conf"
        )
}

pub fn import_files(
    ctx: &AppContext,
    project_id: Option<&str>,
    conversation_id: Option<&str>,
    paths: &[String],
) -> Result<Vec<Attachment>, AppError> {
    if paths.is_empty() {
        return Err(AppError::invalid("no files selected"));
    }
    if paths.len() > 20 {
        return Err(AppError::invalid("attach at most 20 files at once"));
    }
    if let Some(id) = project_id {
        ctx.db.get_project(id)?;
    }
    if let Some(id) = conversation_id {
        ctx.db.get_conversation(id)?;
    }

    let mut imported = Vec::with_capacity(paths.len());
    for raw in paths {
        let source = PathBuf::from(raw);
        let metadata = std::fs::metadata(&source)
            .map_err(|_| AppError::invalid(format!("cannot read file: {}", source.display())))?;
        if !metadata.is_file() {
            return Err(AppError::invalid(format!(
                "not a regular file: {}",
                source.display()
            )));
        }
        if metadata.len() > MAX_FILE_BYTES {
            return Err(AppError::invalid(format!(
                "file is larger than 50 MB: {}",
                source.display()
            )));
        }
        let bytes = std::fs::read(&source)?;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));

        let display_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .map(sanitize_filename)
            .unwrap_or_else(|| "file".to_string());
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let mime = mime_guess::from_path(&source)
            .first_or_octet_stream()
            .to_string();
        let kind = if mime.starts_with("image/") {
            "image"
        } else if looks_textual(&mime, &extension) {
            "text"
        } else {
            "file"
        };
        let text_content = if kind == "text" && bytes.len() <= MAX_TEXT_BYTES {
            String::from_utf8(bytes.clone()).ok()
        } else {
            None
        };
        if kind == "text" && text_content.is_none() {
            return Err(AppError::invalid(format!(
                "file is not valid UTF-8 text: {}",
                source.display()
            )));
        }

        let destination = ctx
            .paths
            .attachments_dir
            .join(format!("{}-{}", new_id(), display_name));
        std::fs::write(&destination, &bytes)?;

        let created = ctx.db.insert_attachment(NewAttachment {
            project_id,
            conversation_id,
            message_id: None,
            name: &display_name,
            mime: &mime,
            size: metadata.len() as i64,
            path: &destination.to_string_lossy(),
            sha256: &sha256,
            kind,
            text_content: text_content.as_deref(),
        });
        match created {
            Ok(attachment) => imported.push(attachment),
            Err(error) => {
                let _ = std::fs::remove_file(&destination);
                return Err(error);
            }
        }
    }
    Ok(imported)
}

pub fn verified_path(ctx: &AppContext, attachment: &Attachment) -> Result<PathBuf, AppError> {
    let path = Path::new(&attachment.path);
    ensure_within(&ctx.paths.attachments_dir, path)
}

pub fn image_parts(
    ctx: &AppContext,
    attachments: &[Attachment],
) -> Result<Vec<ImagePart>, AppError> {
    let mut parts = Vec::new();
    for attachment in attachments {
        if attachment.kind != "image" {
            continue;
        }
        if attachment.size as usize > MAX_IMAGE_BYTES {
            return Err(AppError::invalid(format!(
                "image is too large to send (max 12 MB): {}",
                attachment.name
            )));
        }
        let path = verified_path(ctx, attachment)?;
        let bytes = std::fs::read(path)?;
        parts.push(ImagePart {
            mime: attachment.mime.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        });
    }
    Ok(parts)
}

pub fn text_excerpts(attachments: &[Attachment]) -> String {
    let mut out = String::new();
    for attachment in attachments {
        if let Some(text) = &attachment.text_content {
            if text.trim().is_empty() {
                continue;
            }
            out.push_str(&format!(
                "\n\n[Attached file: {} ({}, {} bytes)]\n```\n{}\n```",
                attachment.name,
                attachment.mime,
                attachment.size,
                text.trim_end()
            ));
        } else if attachment.kind == "file" {
            out.push_str(&format!(
                "\n\n[Attached binary file: {} ({}, {} bytes). Its contents are not readable in this prompt.]",
                attachment.name, attachment.mime, attachment.size
            ));
        }
    }
    out
}

pub fn remove_file(ctx: &AppContext, attachment: &Attachment) {
    if let Ok(path) = verified_path(ctx, attachment) {
        let _ = std::fs::remove_file(path);
    }
}
