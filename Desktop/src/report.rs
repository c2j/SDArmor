use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::scanner::ScanResult;

/// Report format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Html,
    Markdown,
    Pdf,
}

impl ReportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Html => "html",
            Self::Markdown => "md",
            Self::Pdf => "pdf",
        }
    }
}

/// Complete scan report with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub scan_target: PathBuf,
    pub results: ScanResult,
    pub summary: String,
    pub report_id: String,
    // 新增字段
    pub application_name: Option<String>,  // 归属应用
    pub uploader_name: Option<String>,     // 上传人
    pub rule_version: Option<String>,      // 扫描规则版本
    pub notes: Option<String>,             // 备注信息
}

impl Default for Report {
    fn default() -> Self {
        Self {
            title: "Security Scan Report".to_string(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            scan_target: PathBuf::new(),
            results: ScanResult::default(),
            summary: String::new(),
            report_id: uuid::Uuid::new_v4().to_string(),
            application_name: None,
            uploader_name: None,
            rule_version: None,
            notes: None,
        }
    }
}

/// Report generator for creating and saving reports
pub struct ReportGenerator {
    report: Report,
    output_dir: PathBuf,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(scan_results: ScanResult, scan_target: PathBuf) -> Self {
        let mut report = Report::default();
        let scan_target_clone = scan_target.clone();
        report.results = scan_results.clone();
        report.scan_target = scan_target;
        report.title = format!("Security Scan Report - {}",
            scan_target_clone.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        report.summary = Self::generate_summary(&scan_results);

        let output_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("./reports"))
            .join("SDChat-Scanner/reports");

        Self {
            report,
            output_dir,
        }
    }

    /// Set a custom output directory
    pub fn with_output_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.output_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Generate a report summary
    fn generate_summary(results: &ScanResult) -> String {
        let stats = &results.stats;
        let total_vulns = stats.critical + stats.high + stats.medium + stats.low;

        format!(
            "Scan Summary:\n\
            - Total files scanned: {}\n\
            - Total vulnerabilities found: {}\n\
            - Critical: {}\n\
            - High: {}\n\
            - Medium: {}\n\
            - Low: {}\n\
            - Scan duration: {} ms",
            stats.scanned_files,
            total_vulns,
            stats.critical,
            stats.high,
            stats.medium,
            stats.low,
            stats.scan_time_ms
        )
    }

    /// Generate and save a report
    pub fn generate(&self, format: ReportFormat) -> Result<PathBuf> {
        // Ensure output directory exists
        fs::create_dir_all(&self.output_dir)?;

        // Generate filename based on timestamp and format
        let timestamp = self.report.timestamp.format("%Y%m%d_%H%M%S");
        let filename = format!("security_scan_{}_{}.{}",
            self.report.scan_target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            timestamp,
            format.extension()
        );

        let output_path = self.output_dir.join(filename);

        // Generate report in the specified format
        match format {
            ReportFormat::Json => self.generate_json(&output_path),
            ReportFormat::Html => self.generate_html(&output_path),
            ReportFormat::Markdown => self.generate_markdown(&output_path),
            ReportFormat::Pdf => self.generate_pdf(&output_path),
        }?;

        info!("Report generated at: {}", output_path.display());
        Ok(output_path)
    }

    /// Generate a JSON format report
    fn generate_json(&self, output_path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.report)?;
        fs::write(output_path, json)?;
        Ok(())
    }

    /// Generate an HTML format report
    fn generate_html(&self, output_path: &Path) -> Result<()> {
        let mut html = String::new();

        // HTML header
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", self.report.title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 0; padding: 20px; }\n");
        html.push_str("h1, h2 { color: #333; }\n");
        html.push_str(".summary { background-color: #f5f5f5; padding: 15px; border-radius: 5px; }\n");
        html.push_str(".critical { color: #d9534f; }\n");
        html.push_str(".high { color: #f0ad4e; }\n");
        html.push_str(".medium { color: #5bc0de; }\n");
        html.push_str(".low { color: #5cb85c; }\n");
        html.push_str(".vulnerability { margin-bottom: 20px; border-left: 4px solid #ddd; padding-left: 15px; }\n");
        html.push_str(".code { background-color: #f8f9fa; padding: 10px; border-radius: 3px; font-family: monospace; overflow-x: auto; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        // Report header
        html.push_str(&format!("<h1>{}</h1>\n", self.report.title));
        html.push_str(&format!("<p>Generated at: {}</p>\n", self.report.timestamp));
        html.push_str(&format!("<p>Target: {}</p>\n", self.report.scan_target.display()));

        // Summary
        html.push_str("<div class='summary'>\n");
        html.push_str("<h2>Summary</h2>\n");
        html.push_str(&format!("<pre>{}</pre>\n", self.report.summary));
        html.push_str("</div>\n");

        // Vulnerabilities
        html.push_str("<h2>Detected Vulnerabilities</h2>\n");

        if self.report.results.code_snippets.is_empty() {
            html.push_str("<p>No vulnerabilities detected.</p>\n");
        } else {
            for (i, snippet) in self.report.results.code_snippets.iter().enumerate() {
                let severity_class = match snippet.severity {
                    crate::rules::Severity::Critical => "critical",
                    crate::rules::Severity::High => "high",
                    crate::rules::Severity::Medium => "medium",
                    crate::rules::Severity::Low => "low",
                };

                html.push_str(&format!("<div class='vulnerability'>\n"));
                html.push_str(&format!("<h3><span class='{}'>{}: {}</span></h3>\n",
                    severity_class, i+1, snippet.title));
                html.push_str(&format!("<p><strong>File:</strong> {}</p>\n", snippet.file_path.display()));
                html.push_str(&format!("<p><strong>Line:</strong> {}</p>\n", snippet.line_number));
                html.push_str(&format!("<p><strong>Description:</strong> {}</p>\n", snippet.description));
                html.push_str(&format!("<div class='code'><pre>{}</pre></div>\n",
                    html_escape::encode_safe(&snippet.code)));
                html.push_str("</div>\n");
            }
        }

        // HTML footer
        html.push_str("</body>\n</html>");

        fs::write(output_path, html)?;
        Ok(())
    }

    /// Generate a Markdown format report
    fn generate_markdown(&self, output_path: &Path) -> Result<()> {
        let mut md = String::new();

        // Report header
        md.push_str(&format!("# {}\n\n", self.report.title));
        md.push_str(&format!("Generated at: {}\n\n", self.report.timestamp));
        md.push_str(&format!("Target: {}\n\n", self.report.scan_target.display()));

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str("```\n");
        md.push_str(&self.report.summary);
        md.push_str("\n```\n\n");

        // Vulnerabilities
        md.push_str("## Detected Vulnerabilities\n\n");

        if self.report.results.code_snippets.is_empty() {
            md.push_str("No vulnerabilities detected.\n\n");
        } else {
            for (i, snippet) in self.report.results.code_snippets.iter().enumerate() {
                let severity = match snippet.severity {
                    crate::rules::Severity::Critical => "CRITICAL",
                    crate::rules::Severity::High => "HIGH",
                    crate::rules::Severity::Medium => "MEDIUM",
                    crate::rules::Severity::Low => "LOW",
                };

                md.push_str(&format!("### {}. {} ({})\n\n", i+1, snippet.title, severity));
                md.push_str(&format!("- **File:** {}\n", snippet.file_path.display()));
                md.push_str(&format!("- **Line:** {}\n", snippet.line_number));
                md.push_str(&format!("- **Description:** {}\n\n", snippet.description));
                md.push_str("```\n");
                md.push_str(&snippet.code);
                md.push_str("\n```\n\n");
            }
        }

        fs::write(output_path, md)?;
        Ok(())
    }

    /// Generate a PDF format report
    fn generate_pdf(&self, output_path: &Path) -> Result<()> {
        // Generate HTML first, then convert to PDF
        let temp_html_path = output_path.with_extension("html");
        self.generate_html(&temp_html_path)?;

        // For a real implementation, you'd use a PDF conversion library here
        // For this example, we'll just copy the HTML file and change the extension
        warn!("PDF generation not fully implemented; creating an HTML report instead");
        fs::copy(&temp_html_path, output_path)?;

        // Clean up temporary file
        let _ = fs::remove_file(&temp_html_path);

        Ok(())
    }

    /// Upload the report to a server
    pub async fn upload_report(&self, server_url: &str) -> Result<String> {
        info!("Uploading report to server: {}", server_url);

        // First generate a JSON report as string
        let report_json = serde_json::to_string_pretty(&self.report)?;

        // Send to server
        let client = reqwest::Client::new();
        let response = client.post(server_url)
            .header("Content-Type", "application/json")
            .body(report_json)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to upload report: HTTP {}", response.status()));
        }

        // Parse response to get report URL
        let response_text = response.text().await?;

        // In a real implementation, you would parse the response to get the URL
        // For this example, we'll just return a fake URL
        let report_url = format!("{}/reports/{}", server_url, self.report.report_id);

        info!("Report uploaded successfully: {}", report_url);
        Ok(report_url)
    }
}