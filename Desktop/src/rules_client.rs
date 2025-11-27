//! Rules client for communicating with backend rules API.
//!
//! This module handles rule set management operations with the SDChat backend,
//! including fetching rules, creating/updating rule sets, and managing file types.

use anyhow::{anyhow, Result};
use log::{info, warn, error};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth_client::AuthClient;
use crate::network::NetworkClient;

/// File type data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileType {
    pub id: u32,
    pub name: String,
    pub ruleset_id: u32,
    pub identifiers: Vec<FileTypeIdentifier>,
    pub patterns: Vec<Pattern>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// File type identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeIdentifier {
    #[serde(rename = "type")]
    pub id_type: String,
    pub pattern: String,
}

/// Pattern data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: u32,
    pub id_code: String,
    pub description: String,
    pub severity: String,
    pub regex: String,
    pub filetype_id: u32,
}

/// Rule set summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSetSummary {
    pub id: u32,
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub file_types_count: u32,
}

/// Rule set details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub id: u32,
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub file_types: Vec<FileType>,
}

/// Rule set creation request
#[derive(Debug, Serialize)]
pub struct CreateRuleSetRequest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub file_types: Vec<CreateFileTypeRequest>,
}

/// File type creation request
#[derive(Debug, Serialize)]
pub struct CreateFileTypeRequest {
    pub name: String,
    pub identifiers: Vec<FileTypeIdentifier>,
    pub patterns: Vec<CreatePatternRequest>,
}

/// Pattern creation request
#[derive(Debug, Serialize)]
pub struct CreatePatternRequest {
    pub id: String,
    pub description: String,
    pub severity: String,
    pub regex: String,
}

/// Rule set update request
#[derive(Debug, Serialize)]
pub struct UpdateRuleSetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// API error response
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: Option<String>,
}

/// Rules client for API communication
pub struct RulesClient {
    network_client: Arc<RwLock<NetworkClient>>,
    auth_client: Arc<RwLock<AuthClient>>,
}

impl RulesClient {
    /// Create a new rules client
    pub fn new(auth_client: Arc<RwLock<AuthClient>>) -> Result<Self> {
        let network_client = Arc::new(RwLock::new(NetworkClient::new()?));

        Ok(Self {
            network_client,
            auth_client,
        })
    }

    /// Get all rule sets
    pub async fn get_rulesets(&self, active_only: bool) -> Result<Vec<RuleSetSummary>> {
        info!("Getting rule sets (active_only: {})", active_only);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let mut url = network_client.config.rule_server_url.clone();

        if active_only {
            url.push_str("?active=true");
        }

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let rulesets: Vec<RuleSetSummary> = response.json().await?;
                info!("Retrieved {} rule sets", rulesets.len());
                Ok(rulesets)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to rules"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get rule sets: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Get rule set by ID
    pub async fn get_ruleset(&self, ruleset_id: u32) -> Result<RuleSet> {
        info!("Getting rule set with ID: {}", ruleset_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/{}", network_client.config.rule_server_url, ruleset_id);

        let response = network_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let ruleset: RuleSet = response.json().await?;
                info!("Retrieved rule set: {}", ruleset.name);
                Ok(ruleset)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to rule set"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Rule set not found"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to get rule set: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Create a new rule set
    pub async fn create_ruleset(&self, request: CreateRuleSetRequest) -> Result<RuleSet> {
        info!("Creating rule set: {}", request.name);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = network_client.config.rule_server_url.clone();

        let response = network_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED => {
                let ruleset: RuleSet = response.json().await?;
                info!("Created rule set: {}", ruleset.name);
                Ok(ruleset)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to create rule set"))
            }
            StatusCode::BAD_REQUEST => {
                let error: ApiError = response.json().await?;
                Err(anyhow!("Bad request: {}", error.error))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to create rule set: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Update an existing rule set
    pub async fn update_ruleset(&self, ruleset_id: u32, request: UpdateRuleSetRequest) -> Result<RuleSet> {
        info!("Updating rule set with ID: {}", ruleset_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/{}", network_client.config.rule_server_url, ruleset_id);

        let response = network_client
            .put(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let ruleset: RuleSet = response.json().await?;
                info!("Updated rule set: {}", ruleset.name);
                Ok(ruleset)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to update rule set"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Rule set not found"))
            }
            StatusCode::BAD_REQUEST => {
                let error: ApiError = response.json().await?;
                Err(anyhow!("Bad request: {}", error.error))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to update rule set: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Delete a rule set
    pub async fn delete_ruleset(&self, ruleset_id: u32) -> Result<()> {
        info!("Deleting rule set with ID: {}", ruleset_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/{}", network_client.config.rule_server_url, ruleset_id);

        let response = network_client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                info!("Deleted rule set with ID: {}", ruleset_id);
                Ok(())
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to delete rule set"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Rule set not found"))
            }
            StatusCode::FORBIDDEN => {
                Err(anyhow!("Cannot delete active rule set"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to delete rule set: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Activate a rule set
    pub async fn activate_ruleset(&self, ruleset_id: u32) -> Result<()> {
        info!("Activating rule set with ID: {}", ruleset_id);

        let auth_client = self.auth_client.read().await;
        let access_token = auth_client.get_access_token().await?;

        let network_client = self.network_client.read().await;
        let url = format!("{}/activate", network_client.config.rule_server_url, ruleset_id);

        let response = network_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                info!("Activated rule set with ID: {}", ruleset_id);
                Ok(())
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Unauthorized access to activate rule set"))
            }
            StatusCode::NOT_FOUND => {
                Err(anyhow!("Rule set not found"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Failed to activate rule set: HTTP {} - {}", status, error_text))
            }
        }
    }

    /// Export rule set
    pub async fn export_ruleset(&self, ruleset_id: u32) -> Result<serde_json::Value> {
        info!("Exporting rule set with ID: {}", ruleset_id);

        let ruleset = self.get_ruleset(ruleset_id).await?;

        let export_data = serde_json::json!({
            "name": ruleset.name,
            "version": ruleset.version,
            "description": ruleset.description.unwrap_or_default(),
            "file_types": ruleset.file_types.into_iter().map(|ft| {
                serde_json::json!({
                    "name": ft.name,
                    "identifiers": ft.identifiers,
                    "patterns": ft.patterns.into_iter().map(|p| {
                        serde_json::json!({
                            "id": p.id_code,
                            "description": p.description,
                            "severity": p.severity,
                            "regex": p.regex
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });

        Ok(export_data)
    }
}