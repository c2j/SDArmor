use anyhow::{anyhow, Result};
use log::{info, warn, error};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;

// Hyperscan is loaded conditionally when the feature is enabled

use crate::rules::{RuleSet, Severity};

// Abstract regex engine for pattern matching
enum RegexEngine {
    // Standard Rust regex engine
    Standard(HashMap<String, Regex>),

    #[cfg(feature = "hyperscan_engine")]
    // Hyperscan engine for high-performance matching
    Hyperscan(hyperscan::BlockDatabase),
}

impl RegexEngine {
    fn new(patterns: &[String]) -> Result<Self> {
        // Try Hyperscan only if the feature is enabled
        #[cfg(feature = "hyperscan_engine")]
        {
            // Try to use Hyperscan first if the feature is enabled
            match Self::create_hyperscan(patterns) {
                Ok(engine) => return Ok(engine),
                Err(e) => {
                    warn!("Failed to initialize Hyperscan: {}", e);
                    warn!("Falling back to standard regex engine");
                }
            }
        }

        // Use standard regex (either as fallback or as primary if Hyperscan isn't available)
        Self::create_standard(patterns)
    }

    #[cfg(feature = "hyperscan_engine")]
    fn create_hyperscan(patterns: &[String]) -> Result<Self> {
        use hyperscan::prelude::*;

        let mut builder = BlockDatabaseBuilder::new()?;

        for pattern in patterns {
            builder.add(pattern, 0)?;
        }

        let database = builder.build()?;
        Ok(RegexEngine::Hyperscan(database))
    }

    fn create_standard(patterns: &[String]) -> Result<Self> {
        let mut regex_map = HashMap::new();

        for (_, pattern) in patterns.iter().enumerate() {
            match Regex::new(pattern) {
                Ok(regex) => {
                    regex_map.insert(pattern.clone(), regex);
                },
                Err(e) => {
                    warn!("Failed to compile regex pattern '{}': {}", pattern, e);
                    // Continue with other patterns
                }
            }
        }

        Ok(RegexEngine::Standard(regex_map))
    }

    fn is_match(&self, pattern: &str, text: &str) -> bool {
        match self {
            RegexEngine::Standard(map) => {
                if let Some(regex) = map.get(pattern) {
                    regex.is_match(text)
                } else {
                    false
                }
            },

            #[cfg(feature = "hyperscan_engine")]
            RegexEngine::Hyperscan(db) => {
                use hyperscan::prelude::*;

                let mut result = false;
                match db.alloc_scratch() {
                    Ok(scratch) => {
                        let handler = |_id: u32, _from: u64, _to: u64, _flags: u32| {
                            result = true;
                            // Stop scanning once a match is found
                            Matching::Terminate
                        };

                        match db.scan(text.as_bytes(), &scratch, handler) {
                            Ok(_) => result,
                            Err(_) => false,
                        }
                    },
                    Err(_) => false
                }
            }
        }
    }
}

/// Represents a hotspot in the 3D visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub severity: Severity,
    pub file_path: PathBuf,
    pub rule_id: String,
}

/// Represents a code snippet with vulnerability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub title: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub code: String,
    pub severity: Severity,
    pub description: String,
    pub rule_id: String,
}

/// Statistics about scan results
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_files: usize,
    pub scanned_files: usize,
    pub ignored_files: usize,
    pub total_lines: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub scan_time_ms: u64,
}

/// Complete scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub hotspots: Vec<Hotspot>,
    pub code_snippets: Vec<Snippet>,
    pub stats: Stats,
}

impl Default for ScanResult {
    fn default() -> Self {
        Self {
            hotspots: Vec::new(),
            code_snippets: Vec::new(),
            stats: Stats::default(),
        }
    }
}

/// Scanner configuration
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub target_path: PathBuf,
    pub rules: RuleSet,
    pub ignore_patterns: Vec<Regex>,
    pub max_file_size: usize,
    pub thread_count: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            target_path: PathBuf::new(),
            rules: RuleSet::default(),
            ignore_patterns: vec![
                Regex::new(r"\.git/").unwrap(),
                Regex::new(r"node_modules/").unwrap(),
                Regex::new(r"\.DS_Store$").unwrap(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            thread_count: num_cpus::get(),
        }
    }
}

/// The main scanner engine
pub struct Scanner {
    config: ScanConfig,
    progress: Arc<Mutex<f32>>,
    results: Arc<Mutex<ScanResult>>,
}

impl Scanner {
    pub fn new(config: ScanConfig) -> Self {
        Self {
            config,
            progress: Arc::new(Mutex::new(0.0)),
            results: Arc::new(Mutex::new(ScanResult::default())),
        }
    }

    // Helper method to create a regex engine from rule patterns
    fn create_regex_engine(file_type: &crate::rules::FileType) -> Result<RegexEngine> {
        let patterns: Vec<String> = file_type.patterns
            .iter()
            .map(|p| p.regex.clone())
            .collect();

        RegexEngine::new(&patterns)
    }

    /// Start scanning asynchronously and returns a channel for progress updates
    pub async fn start_scan_async(&self) -> Result<mpsc::Receiver<f32>> {
        let (tx, rx) = mpsc::channel(100);
        let config = self.config.clone();
        let progress = self.progress.clone();
        let results = self.results.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            match Self::scan_directory(config, progress.clone(), results) {
                Ok(_) => {
                    info!("Scan completed successfully");
                    // Send final progress update
                    let _ = tx_clone.send(1.0).await;
                }
                Err(e) => {
                    error!("Scan failed: {}", e);
                    // Send error signal (negative progress)
                    let _ = tx_clone.send(-1.0).await;
                }
            }
        });

        // Spawn a task to monitor progress and send updates
        let progress_clone = self.progress.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let current_progress = *progress_clone.lock().unwrap();

                if tx.send(current_progress).await.is_err() {
                    break;
                }

                if current_progress >= 1.0 || current_progress < 0.0 {
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// Get the current scan results
    pub fn get_results(&self) -> ScanResult {
        self.results.lock().unwrap().clone()
    }

    /// Scan a directory recursively
    fn scan_directory(
        config: ScanConfig,
        progress: Arc<Mutex<f32>>,
        results: Arc<Mutex<ScanResult>>,
    ) -> Result<()> {
        use crate::rules;
        let start_time = Instant::now();

        // First, collect all files that need to be scanned
        info!("Collecting files to scan from: {}", config.target_path.display());
        let files = Self::collect_files(&config.target_path, &config.ignore_patterns)?;
        let total_files = files.len();

        info!("Found {} files to scan", total_files);
        {
            let mut scan_results = results.lock().unwrap();
            scan_results.stats.total_files = total_files;
        }

        // Create a thread pool and process files in parallel
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.thread_count)
            .build()?;

        // Keep track of processed files for progress reporting
        let processed_files = Arc::new(Mutex::new(0));

        pool.install(|| {
            files.par_iter().for_each(|file_path| {
                match Self::scan_file(file_path, &config.rules) {
                    Ok(mut file_results) => {
                        // Update results with findings from this file
                        let mut scan_results = results.lock().unwrap();

                        // Count hotspots by severity before moving them
                        for hotspot in &file_results.hotspots {
                            match hotspot.severity {
                                Severity::Critical => scan_results.stats.critical += 1,
                                Severity::High => scan_results.stats.high += 1,
                                Severity::Medium => scan_results.stats.medium += 1,
                                Severity::Low => scan_results.stats.low += 1,
                            }
                        }

                        // Move the data
                        scan_results.hotspots.append(&mut file_results.hotspots);
                        scan_results.code_snippets.append(&mut file_results.code_snippets);
                        scan_results.stats.scanned_files += 1;
                    },
                    Err(e) => {
                        warn!("Failed to scan file {}: {}", file_path.display(), e);
                        // Update ignored files count
                        let mut scan_results = results.lock().unwrap();
                        scan_results.stats.ignored_files += 1;
                    }
                }

                // Update progress and scan time
                let mut processed = processed_files.lock().unwrap();
                *processed += 1;
                let current_progress = *processed as f32 / total_files as f32;
                *progress.lock().unwrap() = current_progress;

                // Update scan time periodically
                if *processed % 10 == 0 { // Update every 10 files
                    let current_time = start_time.elapsed();
                    let mut scan_results = results.lock().unwrap();
                    scan_results.stats.scan_time_ms = current_time.as_millis() as u64;
                }
            });
        });

        // Update final statistics
        let scan_time = start_time.elapsed();
        let mut scan_results = results.lock().unwrap();
        scan_results.stats.scan_time_ms = scan_time.as_millis() as u64;

        info!("Scan completed in {} ms", scan_results.stats.scan_time_ms);

        Ok(())
    }

    /// Scan a single file for vulnerabilities
    fn scan_file(file_path: &Path, rules: &RuleSet) -> Result<ScanResult> {
        let mut file_result = ScanResult::default();

        // Check if this file matches any file types defined in rules
        let file_type = Self::determine_file_type(file_path, rules)?;
        if file_type.is_none() {
            return Ok(file_result);
        }

        let file_type = file_type.unwrap();

        // Create a regex engine for this file type
        let regex_engine = Self::create_regex_engine(file_type)?;

        // Open and read the file
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // Process the file line by line
        for (line_number, line_result) in reader.lines().enumerate() {
            let line = line_result?;

            // Apply all patterns for this file type
            for pattern in &file_type.patterns {
                if regex_engine.is_match(&pattern.regex, &line) {
                    // Found a match, create a hotspot and snippet
                    let hotspot = Hotspot {
                        x: rand::random::<f32>() * 100.0,  // In real impl, this would be based on code metrics
                        y: rand::random::<f32>() * 100.0,
                        z: rand::random::<f32>() * 100.0,
                        severity: pattern.severity.clone(),
                        file_path: file_path.to_path_buf(),
                        rule_id: pattern.id.clone(),
                    };

                    let snippet = Snippet {
                        title: pattern.id.clone(),
                        file_path: file_path.to_path_buf(),
                        line_number: line_number + 1,
                        code: line.clone(),
                        severity: pattern.severity.clone(),
                        description: pattern.description.clone(),
                        rule_id: pattern.id.clone(),
                    };

                    file_result.hotspots.push(hotspot);
                    file_result.code_snippets.push(snippet);
                }
            }
        }

        Ok(file_result)
    }

    /// Determine the file type based on rules
    fn determine_file_type<'a>(file_path: &Path, rules: &'a RuleSet) -> Result<Option<&'a crate::rules::FileType>> {
        // Try to match by file extension first
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_string();

            for file_type in &rules.file_types {
                for identifier in &file_type.identifiers {
                    if identifier.r#type == "extension" {
                        if let Ok(regex) = Regex::new(&identifier.pattern) {
                            if regex.is_match(&ext) {
                                return Ok(Some(file_type));
                            }
                        }
                    }
                }
            }
        }

        // If no match by extension, try content-based identification
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        for file_type in &rules.file_types {
            for identifier in &file_type.identifiers {
                if identifier.r#type == "content" {
                    if let Ok(regex) = Regex::new(&identifier.pattern) {
                        if regex.is_match(&first_line) {
                            return Ok(Some(file_type));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Recursively collect all files from a directory
    fn collect_files(dir: &Path, ignore_patterns: &[Regex]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !dir.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", dir.display()));
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if path should be ignored
            let path_str = path.to_string_lossy();
            if ignore_patterns.iter().any(|re| re.is_match(&path_str)) {
                continue;
            }

            if path.is_dir() {
                // Recursively collect files from subdirectories
                let sub_files = Self::collect_files(&path, ignore_patterns)?;
                files.extend(sub_files);
            } else {
                files.push(path);
            }
        }

        Ok(files)
    }
}