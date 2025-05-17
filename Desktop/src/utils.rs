use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Generate a unique ID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Get current timestamp in ISO format
pub fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}

/// Get current local time formatted
pub fn formatted_local_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Convert UTC timestamp to local time
pub fn utc_to_local(utc_time: &DateTime<Utc>) -> DateTime<Local> {
    DateTime::<Local>::from(*utc_time)
}

/// Format a duration in a human-readable format
pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else if seconds < 3600 {
        format!("{} minutes, {} seconds", seconds / 60, seconds % 60)
    } else {
        format!("{} hours, {} minutes", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        debug!("Created directory: {}", path.display());
    }
    Ok(())
}

/// Check if a file exists and is readable
pub fn check_file_readable(path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        metadata.is_file()
    } else {
        false
    }
}

/// Get a file size in human-readable format
pub fn format_file_size(size_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size_bytes < KB {
        format!("{} B", size_bytes)
    } else if size_bytes < MB {
        format!("{:.2} KB", size_bytes as f64 / KB as f64)
    } else if size_bytes < GB {
        format!("{:.2} MB", size_bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", size_bytes as f64 / GB as f64)
    }
}

/// Calculate file hash (MD5)
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    use std::io::{BufReader, Read};
    
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut context = md5::Context::new();
    let mut buffer = [0; 1024];
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        context.consume(&buffer[..bytes_read]);
    }
    
    let digest = context.compute();
    Ok(format!("{:x}", digest))
}

/// Truncate a string to a maximum length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_len-3])
    }
}

/// Check if a string matches a regex pattern
pub fn string_matches_pattern(text: &str, pattern: &str) -> bool {
    match Regex::new(pattern) {
        Ok(regex) => regex.is_match(text),
        Err(e) => {
            warn!("Invalid regex pattern '{}': {}", pattern, e);
            false
        }
    }
}

/// Get extension from file path
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase())
}

/// Check if a file should be ignored based on extension
pub fn should_ignore_file(path: &Path, ignored_extensions: &[String], ignored_patterns: &[Regex]) -> bool {
    // Check extension
    if let Some(ext) = get_file_extension(path) {
        if ignored_extensions.contains(&ext) {
            return true;
        }
    }
    
    // Check patterns
    let path_str = path.to_string_lossy();
    for pattern in ignored_patterns {
        if pattern.is_match(&path_str) {
            return true;
        }
    }
    
    false
}

/// Create a temporary file with the given content
pub fn create_temp_file(content: &str) -> Result<PathBuf> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let path = temp_file.path().to_path_buf();
    fs::write(&path, content)?;
    Ok(path)
}

/// Load file content as string
pub fn read_file_to_string(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

/// Get file modification time
pub fn get_file_modification_time(path: &Path) -> Result<SystemTime> {
    let metadata = fs::metadata(path)?;
    metadata.modified().map_err(|e| anyhow!("Failed to get modification time: {}", e))
}

/// Compare semantic versions
pub fn compare_versions(v1: &str, v2: &str) -> Result<std::cmp::Ordering> {
    let v1 = semver::Version::parse(v1)?;
    let v2 = semver::Version::parse(v2)?;
    Ok(v1.cmp(&v2))
}

/// Check if path contains any of the specified patterns
pub fn path_contains_patterns(path: &Path, patterns: &[&str]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|&pattern| path_str.contains(pattern))
}

/// Log system information
pub fn log_system_info() {
    info!("System Information:");
    info!("  OS: {}", std::env::consts::OS);
    info!("  CPU Cores: {}", num_cpus::get());
    
    if let Ok(hostname) = hostname::get() {
        if let Some(hostname_str) = hostname.to_str() {
            info!("  Hostname: {}", hostname_str);
        }
    }
    
    #[cfg(feature = "hyperscan_engine")]
    info!("  Regex engine: Hyperscan (accelerated)");
    #[cfg(not(feature = "hyperscan_engine"))]
    info!("  Regex engine: Standard");
}