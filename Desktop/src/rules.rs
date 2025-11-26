use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use anyhow::{Result, anyhow};

/// Severity levels for vulnerabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Medium
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "Critical"),
            Severity::High => write!(f, "High"),
            Severity::Medium => write!(f, "Medium"),
            Severity::Low => write!(f, "Low"),
        }
    }
}

/// Rule for vulnerability detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub description: String,
    pub severity: Severity,
    pub regex: String,
}

/// Rule pattern for vulnerability detection (alias for Rule for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePattern {
    pub id: String,
    pub description: String,
    pub severity: Severity,
    pub regex: String,
}

/// File type identifier for rule matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeIdentifier {
    pub r#type: String,  // "extension" or "content"
    pub pattern: String,
}

/// File type with associated rule patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileType {
    pub name: String,
    pub identifiers: Vec<FileTypeIdentifier>,
    pub patterns: Vec<RulePattern>,
}

/// Complete rule set containing all file types and patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub version: String,
    pub file_types: Vec<FileType>,
}

impl Default for RuleSet {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            file_types: Vec::new(),
        }
    }
}

impl RuleSet {
    /// Load rules from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let ruleset: RuleSet = serde_json::from_reader(reader)?;
        Ok(ruleset)
    }

    /// Load rules from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let ruleset: RuleSet = serde_json::from_str(json)?;
        Ok(ruleset)
    }

    /// Create a new ruleset with a single rule
    pub fn with_rule(rule: Rule) -> Self {
        let pattern = RulePattern {
            id: rule.id,
            description: rule.description,
            severity: rule.severity,
            regex: rule.regex,
        };

        let file_type = FileType {
            name: "Custom Rules".to_string(),
            identifiers: vec![],
            patterns: vec![pattern],
        };

        Self {
            version: "1.0".to_string(),
            file_types: vec![file_type],
        }
    }

    /// Get all rule patterns across all file types
    pub fn all_patterns(&self) -> Vec<&RulePattern> {
        self.file_types
            .iter()
            .flat_map(|ft| ft.patterns.iter())
            .collect()
    }

    /// Find a rule pattern by ID
    pub fn find_pattern_by_id(&self, id: &str) -> Option<&RulePattern> {
        self.file_types
            .iter()
            .flat_map(|ft| ft.patterns.iter())
            .find(|pattern| pattern.id == id)
    }

    /// Add a new file type with patterns
    pub fn add_file_type(&mut self, file_type: FileType) {
        // Check if a file type with this name already exists
        if let Some(index) = self.file_types.iter().position(|ft| ft.name == file_type.name) {
            // Replace existing file type
            self.file_types[index] = file_type;
        } else {
            // Add new file type
            self.file_types.push(file_type);
        }
    }

    /// Merge another ruleset into this one
    pub fn merge(&mut self, other: RuleSet) {
        for file_type in other.file_types {
            self.add_file_type(file_type);
        }
    }

    /// Get statistics about this ruleset
    pub fn stats(&self) -> RuleStats {
        let mut stats = RuleStats::default();

        stats.file_types = self.file_types.len();

        for file_type in &self.file_types {
            stats.patterns += file_type.patterns.len();

            for pattern in &file_type.patterns {
                match pattern.severity {
                    Severity::Critical => stats.critical += 1,
                    Severity::High => stats.high += 1,
                    Severity::Medium => stats.medium += 1,
                    Severity::Low => stats.low += 1,
                }
            }
        }

        stats
    }
}

/// Statistics about a ruleset
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    pub file_types: usize,
    pub patterns: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Rule Manager to handle downloading and updating rules
#[derive(Clone)]
pub struct RuleManager {
    ruleset: RuleSet,
    server_url: String,
    cache_path: Option<std::path::PathBuf>,
}

impl RuleManager {
    pub fn new(server_url: &str) -> Self {
        Self {
            ruleset: RuleSet::default(),
            server_url: server_url.to_string(),
            cache_path: None,
        }
    }

    pub fn with_cache<P: AsRef<Path>>(mut self, cache_path: P) -> Self {
        self.cache_path = Some(cache_path.as_ref().to_path_buf());
        self
    }

    pub fn ruleset(&self) -> &RuleSet {
        &self.ruleset
    }

    pub fn update_ruleset(&mut self, new_ruleset: RuleSet) {
        self.ruleset = new_ruleset;
    }

    pub fn load_default_rules(&mut self) -> Result<()> {
        // Load a set of basic default rules (embedded in binary)
        let default_rules = include_str!("../resources/default_rules.json");
        self.ruleset = RuleSet::from_json(default_rules)?;
        Ok(())
    }

    pub fn load_from_cache(&mut self) -> Result<()> {
        if let Some(cache_path) = &self.cache_path {
            if cache_path.exists() {
                self.ruleset = RuleSet::from_file(cache_path)?;
                return Ok(());
            }
        }
        Err(anyhow!("No cache file available"))
    }

    pub fn load_rules_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.ruleset = RuleSet::from_file(path)?;
        Ok(())
    }

    pub async fn update_rules(&mut self) -> Result<bool> {
        // In a real implementation, this would make an HTTP request to the rules server
        // For now, we'll just simulate a successful update

        // Try loading from cache first as fallback
        let _ = self.load_from_cache();

        // Make HTTP request to server_url
        let client = reqwest::Client::new();
        let response = client.get(&self.server_url)
            .header("User-Agent", "SDChat-Scanner/0.1")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to update rules: HTTP {}", response.status()));
        }

        let json = response.text().await?;
        let new_ruleset = RuleSet::from_json(&json)?;

        // Update the ruleset
        self.ruleset = new_ruleset;

        // Cache the ruleset if cache_path is set
        if let Some(cache_path) = &self.cache_path {
            std::fs::write(cache_path, json)?;
        }

        Ok(true)
    }
}