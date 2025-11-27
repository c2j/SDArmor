//! Reports client for communicating with backend reports API.
//!
//! This module handles report management operations with the SDChat backend,
//! including uploading reports, fetching reports, and report management.

use anyhow::{anyhow, Result};
use log::{info, warn, error};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth_client::AuthClient;
use crate::network::NetworkClient;

/// Report data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: u32,
    pub report_id: String,
    pub title: String,
    pub scan_target: String,
    pub summary: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
    pub application_name: Option<String>,
    pub uploader_name: Option<String>,
    pub rule_version: Option<String>>,
    pub notes: Option<String>,
}

/// Report summary for lists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: u32,
    pub report_id: String,
    pub title: String,
    pub scan_target: String,
    pub created_at: Option<String>,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
}

/// Report creation request
#[derive(Debug, Serialize)]
pub struct CreateReportRequest {
    pub title: String,
    pub scan_target: String,
    pub summary: Option<String>,
    pub results: serde_json::Value,
    pub stats: ReportStats,
    pub application_name: Option<String>,
    pub uploader_name: Option<String>,
    pub rule_version: Option<String>,
    pub notes: Option<String>,
}

/// Report statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStats {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub scanned_files: u32,
    pub patterns_matched: u32,
}

/// Report update request
#[derive(Debug, Serialize)]
pub struct UpdateReportRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub notes: Option<String>,
}

/// Paginated reports response
#[derive(Debug, Deserialize)]
pub struct ReportsResponse {
    pub reports: Vec<ReportSummary>,
    pub total: u32,
    pub pages: u32,
    pub current_page: u32,
}

/// API error response
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: Option<String>,
}

/// Reports client for API communication
pub struct ReportsClient {
    network_client: Arc<RwLock<NetworkClient>>,
    auth_client: Arc<RwLock<AuthClient>>,
}

impl ReportsClient {
    /// Create a new reports client
    pub fn new(auth_client: Arc<RwLock<AuthClient>>) -> Result<Self> {
        let network_client = Arc::new(RwLock::new(NetworkClient::new()?));

        Ok(Self {
            network_client,
            auth_client,
        })
    }

    /// Get user reports with pagination
    pub async fn get_user_reports(&self, page: u32 = 1, per_page: u32 = 10,
                              start_date: Option<&str> = None, end_date: Option<&str> = None,
                              search_term: Option<&str> = None) -> Result<ReportsResponse> {
        info!("Getting user reports (page: {}, per_page: {})", page, per_page);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let mut url = format!("{}/reports", network_client.config.report_server_url);

        // Add query parameters
        let mut query_params = Vec::new();
        query_params.push(format!("page={}", page));
        query_params.push(format!("per_page={}", per_page));

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end));
        }

        if let Some(search) = search_term {
            query_params.push(format!("search={}", search));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let reports_response: ReportsResponse = response.json().await?;
                info!("Retrieved {} reports (page {} of {})",
                       reports_response.reports.len(),
                       reports_response.current_page,
                       reports_response.pages);
                Ok(reports_response)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to reports"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get reports: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Get all reports (admin function)
    pub async fn get_all_reports(&self, page: u32 = 1, per_page: u32 = 10,
                             start_date: Option<&str> = None, end_date: Option<&str> = None,
                             search_term: Option<&str> = None) -> Result<ReportsResponse> {
        info!("Getting all reports (page: {}, per_page: {})", page, per_page);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let mut url = format!("{}/reports", network_client.config.report_server_url);

        // Add query parameters
        let mut query_params = Vec::new();
        query_params.push(format!("page={}", page));
        query_params.push(format!("per_page={}", per_page));

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end));
        }

        if let Some(search) = search_term {
            query_params.push(format!("search={}", search));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let reports_response: ReportsResponse = response.json().await?;
                info!("Retrieved {} reports (page {} of {})",
                       reports_response.reports.len(),
                       reports_response.current_page,
                       reports_response.pages);
                Ok(reports_response)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to reports"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get all reports: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Get report by ID
    pub async fn get_report(&self, report_id: &str) -> Result<Report> {
        info!("Getting report with ID: {}", report_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/reports/{}", network_client.config.report_server_url, report_id);

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let report: Report = response.json().await?;
                info!("Retrieved report: {}", report.title);
                Ok(report)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to report"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Report not found"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get report: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Create a new report
    pub async fn create_report(&self, request: CreateReportRequest) -> Result<Report> {
        info!("Creating report: {}", request.title);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/reports", network_client.config.report_server_url);

        let response = network_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED => {
                let report: Report = response.json().await?;
                info!("Created report: {} (ID: {})", report.title, report.report_id);
                Ok(report)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to create report"))
            }
            StatusCode::BAD_REQUEST => {
                let error: ApiError = response.json().await?;
                Err(anyhow!("Bad request: {}", error.error))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to create report: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Update an existing report
    pub async fn update_report(&self, report_id: &str, request: UpdateReportRequest) -> Result<Report> {
        info!("Updating report: {}", report_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/reports/{}", network_client.config.report_server_url, report_id);

        let response = network_client
            .put(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let report: Report = response.json().await?;
                info!("Updated report: {}", report.title);
                Ok(report)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to update report"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Report not found"))
            }
            StatusCode::BAD_REQUEST => {
                let error: ApiError = response.json().await?;
                Err(anyhow!("Bad request: {}", error.error))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to update report: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Delete a report
    pub async fn delete_report(&self, report_id: &str) -> Result<()> {
        info!("Deleting report: {}", report_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/reports/{}", network_client.config.report_server_url, report_id);

        let response = network_client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                info!("Deleted report: {}", report_id);
                Ok(())
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to delete report"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Report not found"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to delete report: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Export report as JSON
    pub async fn export_report(&self, report_id: &str) -> Result<String> {
        info!("Exporting report as JSON: {}", report_id);

        let report = self.get_report(report_id).await?;

        let export_data = serde_json::json!({
            "report_id": report.report_id,
            "title": report.title,
            "scan_target": report.scan_target,
            "summary": report.summary,
            "results": serde_json::Value::Null, // Would need to include actual results
            "stats": {
                "critical": report.critical_count,
                "high": report.high_count,
                "medium": report.medium_count,
                "low": report.low_count,
            },
            "application_name": report.application_name,
            "uploader_name": report.uploader_name,
            "rule_version": report.rule_version,
            "notes": report.notes,
            "created_at": report.created_at,
            "updated_at": report.updated_at,
        });

        Ok(serde_json::to_string_pretty(&export_data)?)
    }

    /// Get report statistics
    pub async fn get_statistics(&self) -> Result<ReportStats> {
        info!("Getting report statistics");

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/reports/stats", network_client.config.report_server_url);

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let stats: ReportStats = response.json().await?;
                info!("Retrieved report statistics");
                Ok(stats)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to report statistics"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get report statistics: HTTP {} - {}", status, error_text))
            }
        }
    }
}