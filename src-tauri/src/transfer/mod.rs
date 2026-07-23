use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

use crate::error::AppError;

/// Default chunk size: 4MB
/// Keep upload updates frequent enough for responsive LAN transfer telemetry
/// without creating excessive HTTP overhead.
pub const CHUNK_SIZE: i64 = 512 * 1024;

/// Sanitize a filename: remove path components, control chars, reserved names.
pub fn sanitize_filename(name: &str) -> Result<String, AppError> {
    // Take only the file name component (strip any directory separators)
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();

    if name.is_empty() || name == "." || name == ".." {
        return Err(AppError::InvalidFilename);
    }

    // Remove control characters and other problematic characters
    let sanitized: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .collect();

    let sanitized = sanitized.trim().to_string();

    if sanitized.is_empty() {
        return Err(AppError::InvalidFilename);
    }

    // Check for Windows reserved names
    let lower = sanitized.to_lowercase();
    let stem = lower.split('.').next().unwrap_or(&lower);
    const RESERVED: &[&str] = &[
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if RESERVED.contains(&stem) {
        return Err(AppError::InvalidFilename);
    }

    // Limit length to 255 bytes
    if sanitized.len() > 255 {
        let ext_pos = sanitized.rfind('.');
        let ext = ext_pos.map(|pos| &sanitized[pos..]).unwrap_or("");
        let max_stem = 255usize.saturating_sub(ext.len()).min(sanitized.len());
        // Find the largest char boundary at or below max_stem so we never
        // split a multi-byte UTF-8 character (e.g. Chinese, emoji).
        let mut end = max_stem;
        while end > 0 && !sanitized.is_char_boundary(end) {
            end -= 1;
        }
        let stem_part = &sanitized[..end];
        return Ok(format!("{}{}", stem_part, ext));
    }

    Ok(sanitized)
}

/// Generate a unique filename if the target already exists: "file (1).pdf", "file (2).pdf"
pub fn unique_filename(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }

    let (stem, ext) = match name.rfind('.') {
        Some(pos) if pos > 0 => (&name[..pos], &name[pos..]),
        _ => (name, ""),
    };

    for i in 1..10000 {
        let new_name = format!("{} ({}){}", stem, i, ext);
        let candidate = dir.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    // Fallback: append timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    dir.join(format!("{} ({}){}", stem, timestamp, ext))
}

/// Compute SHA-256 of a file by streaming (8KB buffer).
pub async fn compute_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Default receive folder path: ~/Downloads/LYNQO
pub fn default_receive_folder() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    } else {
        std::env::var("HOME").ok().map(PathBuf::from)
    };

    match base {
        Some(home) => home.join("Downloads").join("LYNQO"),
        None => PathBuf::from("./received"),
    }
}

/// Ensure receive folder exists and is writable.
pub fn ensure_receive_folder(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(AppError::Io)?;
    }

    // Verify it's a directory
    if !path.is_dir() {
        return Err(AppError::Internal(format!(
            "Receive path is not a directory: {}",
            path.display()
        )));
    }

    // Verify writability by attempting to create and remove a temp file
    let test_file = path.join(".lynqo_write_test");
    std::fs::write(&test_file, b"test").map_err(AppError::Io)?;
    std::fs::remove_file(&test_file).map_err(AppError::Io)?;

    Ok(())
}

/// Build an isolated temp-file path for one transfer.
///
/// A filename alone is not a safe temporary-file identity: two phones can
/// upload `photo.jpg` at the same time. Keeping each transfer in its own
/// directory prevents data corruption and makes cancellation safe.
pub fn temp_file_path(receive_folder: &Path, transfer_id: &str, filename: &str) -> PathBuf {
    receive_folder
        .join(".lynqo-tmp")
        .join(transfer_id)
        .join(format!(".{}.uploading", filename))
}

/// Return the isolated directory containing temporary files for a transfer.
pub fn temp_transfer_dir(receive_folder: &Path, transfer_id: &str) -> PathBuf {
    receive_folder.join(".lynqo-tmp").join(transfer_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_normal() {
        assert_eq!(sanitize_filename("hello.pdf").unwrap(), "hello.pdf");
    }

    #[test]
    fn temp_paths_are_isolated_per_transfer() {
        let receive = Path::new("C:/receive");
        let first = temp_file_path(receive, "transfer-a", "photo.jpg");
        let second = temp_file_path(receive, "transfer-b", "photo.jpg");

        assert_ne!(first, second);
        assert_eq!(
            first,
            PathBuf::from("C:/receive/.lynqo-tmp/transfer-a/.photo.jpg.uploading")
        );
        assert_eq!(
            temp_transfer_dir(receive, "transfer-a"),
            PathBuf::from("C:/receive/.lynqo-tmp/transfer-a")
        );
    }

    #[test]
    fn test_sanitize_filename_strips_path() {
        assert_eq!(sanitize_filename("/etc/passwd").unwrap(), "passwd");
        assert_eq!(
            sanitize_filename("C:\\Users\\file.txt").unwrap(),
            "file.txt"
        );
    }

    #[test]
    fn test_sanitize_filename_rejects_empty() {
        assert!(sanitize_filename("").is_err());
        assert!(sanitize_filename(".").is_err());
        assert!(sanitize_filename("..").is_err());
    }

    #[test]
    fn test_sanitize_filename_rejects_reserved() {
        assert!(sanitize_filename("CON").is_err());
        assert!(sanitize_filename("nul.txt").is_err());
    }

    #[test]
    fn test_sanitize_filename_removes_special_chars() {
        assert_eq!(sanitize_filename("file<name>.txt").unwrap(), "filename.txt");
    }

    #[test]
    fn test_sanitize_filename_truncates_multibyte_safely() {
        // 100 Chinese chars (3 bytes each) + ".txt" => 304 bytes, must be
        // truncated at a char boundary without panicking.
        let long_name = format!("{}.txt", "中".repeat(100));
        let result = sanitize_filename(&long_name).unwrap();
        assert!(result.len() <= 255);
        assert!(result.ends_with(".txt"));
        // Must be valid UTF-8 ending on a char boundary (no panic above),
        // and the stem must be whole Chinese chars.
        let stem = result.strip_suffix(".txt").unwrap();
        assert!(stem.chars().all(|c| c == '中'));
    }
}
