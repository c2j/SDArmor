use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Scanner configuration
    pub scanner: ScannerConfig,
    /// UI configuration
    pub ui: UiConfig,
    /// File paths configuration
    pub paths: PathsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Base URL for server API
    pub base_url: String,
    /// Server URL for rule updates
    pub rule_server_url: String,
    /// Server URL for report submission
    pub report_server_url: String,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// API key for server authentication
    pub api_key: Option<String>,
    /// Enable automatic rule updates
    pub auto_update_rules: bool,
    /// Enable automatic report upload after scan completion
    pub auto_upload_reports: bool,
    /// Last rule update timestamp
    pub last_rule_update: Option<String>,
    /// Scan history with upload status
    pub scan_history: Vec<ScanHistoryEntry>,
}

/// Scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Number of threads to use for scanning
    pub thread_count: usize,
    /// Maximum file size to scan (in bytes)
    pub max_file_size: usize,
    /// File extensions to ignore
    pub ignored_extensions: Vec<String>,
    /// Directory patterns to ignore
    pub ignored_directories: Vec<String>,
    /// Use regex acceleration if available
    pub use_regex_acceleration: bool,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Use dark mode
    pub dark_mode: bool,
    /// Font size
    pub font_size: f32,
    /// Main window size
    pub window_size: (f32, f32),
    /// Show tooltips
    pub show_tooltips: bool,
    /// Collapse side panel by default
    pub collapse_sidepanel: bool,
}

/// File paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Default scan directory
    pub default_scan_dir: Option<PathBuf>,
    /// Report output directory
    pub report_output_dir: Option<PathBuf>,
    /// Rules cache directory
    pub rules_cache_dir: Option<PathBuf>,
    /// Recently scanned paths
    pub recent_scans: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            scanner: ScannerConfig::default(),
            ui: UiConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        // 默认基础URL
        let base_url = "http://127.0.0.1:5000/api/v1".to_string();

        Self {
            base_url: base_url.clone(),
            rule_server_url: format!("{}/rules", base_url),
            report_server_url: format!("{}/reports/upload", base_url),
            username: "admin".to_string(),
            password: "admin123".to_string(),
            api_key: None,
            auto_update_rules: true,
            auto_upload_reports: false,
            last_rule_update: None,
            scan_history: Vec::new(),
        }
    }
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            thread_count: num_cpus::get(),
            max_file_size: 10 * 1024 * 1024, // 10MB
            ignored_extensions: vec![
                "exe".to_string(),
                "dll".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "zip".to_string(),
                "tar".to_string(),
                "gz".to_string(),
            ],
            ignored_directories: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
            ],
            use_regex_acceleration: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            dark_mode: true,
            font_size: 14.0,
            window_size: (1280.0, 720.0),
            show_tooltips: true,
            collapse_sidepanel: false,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            default_scan_dir: None,
            report_output_dir: dirs::document_dir(),
            rules_cache_dir: dirs::cache_dir().map(|p| p.join("sdchat-scanner/rules")),
            recent_scans: Vec::new(),
        }
    }
}

/// Scan history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryEntry {
    /// Scan timestamp
    pub timestamp: DateTime<Utc>,
    /// Scan target path
    pub target_path: PathBuf,
    /// Report file path
    pub report_path: Option<PathBuf>,
    /// Upload status
    pub upload_status: UploadStatus,
    /// Server response (report ID if successful)
    pub server_response: Option<String>,
}

/// Upload status for scan reports
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadStatus {
    /// Not uploaded
    NotUploaded,
    /// Upload in progress
    Uploading,
    /// Upload successful
    Uploaded,
    /// Upload failed
    Failed(String),
}

/// Configuration manager
#[derive(Clone)]
pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new config manager
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sdchat-scanner");

        let config_path = config_dir.join("config.json");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(&config_dir) {
                warn!("Failed to create config directory: {}", e);
            }
        }

        Self {
            config: Config::default(),
            config_path,
        }
    }

    /// Load configuration from file
    pub fn load(&mut self) -> Result<()> {
        if !self.config_path.exists() {
            info!("Config file not found, using defaults");
            return Ok(());
        }

        let config_str = fs::read_to_string(&self.config_path)?;
        self.config = serde_json::from_str(&config_str)?;
        info!("Config loaded from {}", self.config_path.display());

        Ok(())
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_str = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, config_str)?;
        info!("Config saved to {}", self.config_path.display());

        Ok(())
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Add a path to recent scans
    pub fn add_recent_scan(&mut self, path: PathBuf) {
        // Remove if already exists
        self.config.paths.recent_scans.retain(|p| p != &path);

        // Add to front
        self.config.paths.recent_scans.insert(0, path);

        // Limit to 10 recent scans
        if self.config.paths.recent_scans.len() > 10 {
            self.config.paths.recent_scans.truncate(10);
        }
    }
}