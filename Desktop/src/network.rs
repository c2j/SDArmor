use anyhow::{anyhow, Result};
use log::{info, warn};
use reqwest::{Client, ClientBuilder, StatusCode, Url};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

// Constants
const USER_AGENT: &str = concat!("SDChat-Scanner/", env!("CARGO_PKG_VERSION"));
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// API response for rule updates
#[derive(Debug, Deserialize)]
pub struct RuleUpdateResponse {
    pub version: String,
    pub updated_at: String,
    pub file_types_count: usize,
    pub patterns_count: usize,
    pub download_url: String,
}

/// API response for report uploads
#[derive(Debug, Deserialize)]
pub struct ReportUploadResponse {
    pub success: bool,
    pub report_id: String,
    pub view_url: String,
}

/// Signed response for update checks
#[derive(Debug, Deserialize)]
pub struct SignedResponse<T> {
    pub payload: T,
    pub signature: String,
}

/// Update information
#[derive(Debug, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub description: String,
    pub download_url: String,
    pub release_date: String,
    pub size_bytes: u64,
    pub changelog: Vec<String>,
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub rule_server_url: String,
    pub report_api_url: String,
    pub update_check_url: String,
    pub api_key: Option<String>,
    pub proxy_url: Option<String>,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub disable_certificate_validation: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            rule_server_url: "https://rules.sdchat-scanner.com/api/v1/rules".to_string(),
            report_api_url: "https://reports.sdchat-scanner.com/api/v1/upload".to_string(),
            update_check_url: "https://api.sdchat-scanner.com/v1/updates/check".to_string(),
            api_key: None,
            proxy_url: None,
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            disable_certificate_validation: false,
        }
    }
}

/// Network client for API communication
pub struct NetworkClient {
    client: Client,
    config: NetworkConfig,
}

impl NetworkClient {
    /// Create a new network client with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(NetworkConfig::default())
    }
    
    /// Create a new network client with custom configuration
    pub fn with_config(config: NetworkConfig) -> Result<Self> {
        let mut client_builder = ClientBuilder::new()
            .user_agent(USER_AGENT)
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout);
            
        // Configure proxy if provided
        if let Some(proxy_url) = &config.proxy_url {
            let proxy = reqwest::Proxy::all(proxy_url)?;
            client_builder = client_builder.proxy(proxy);
        }
        
        // Configure TLS
        if config.disable_certificate_validation {
            warn!("Certificate validation disabled. This is insecure!");
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        
        let client = client_builder.build()?;
        
        Ok(Self {
            client,
            config,
        })
    }
    
    /// Check for rule updates
    pub async fn check_rule_updates(&self) -> Result<RuleUpdateResponse> {
        info!("Checking for rule updates at {}", self.config.rule_server_url);
        
        let mut request = self.client.get(&self.config.rule_server_url);
        
        // Add API key if available
        if let Some(api_key) = &self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }
        
        let response = request.send().await?;
        
        match response.status() {
            StatusCode::OK => {
                let update_info = response.json::<RuleUpdateResponse>().await?;
                info!("Rule update available: version {}", update_info.version);
                Ok(update_info)
            },
            status => Err(anyhow!("Failed to check for rule updates: HTTP {}", status)),
        }
    }
    
    /// Download rules from the provided URL
    pub async fn download_rules(&self, url: &str, output_path: &Path) -> Result<()> {
        info!("Downloading rules from {}", url);
        
        let mut request = self.client.get(url);
        
        // Add API key if available
        if let Some(api_key) = &self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download rules: HTTP {}", response.status()));
        }
        
        // Create output file
        let mut file = File::create(output_path).await?;
        
        // Write the content to the file
        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }
        
        info!("Rules downloaded to {}", output_path.display());
        Ok(())
    }
    
    /// Upload a scan report
    pub async fn upload_report(&self, report_json: &str) -> Result<ReportUploadResponse> {
        info!("Uploading report to {}", self.config.report_api_url);
        
        let mut request = self.client.post(&self.config.report_api_url)
            .header("Content-Type", "application/json");
            
        // Add API key if available
        if let Some(api_key) = &self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }
        
        // Send the request with the report JSON
        let response = request.body(report_json.to_string()).send().await?;
        
        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let upload_response = response.json::<ReportUploadResponse>().await?;
                info!("Report uploaded successfully: {}", upload_response.view_url);
                Ok(upload_response)
            },
            status => Err(anyhow!("Failed to upload report: HTTP {}", status)),
        }
    }
    
    /// Check for software updates
    pub async fn check_for_updates(&self, current_version: &str) -> Result<Option<UpdateInfo>> {
        info!("Checking for updates at {}", self.config.update_check_url);
        
        let url = Url::parse_with_params(
            &self.config.update_check_url,
            &[("current_version", current_version)]
        )?;
        
        let response = self.client.get(url).send().await?;
        
        match response.status() {
            StatusCode::OK => {
                let signed = response.json::<SignedResponse<UpdateInfo>>().await?;
                
                // In a real implementation, you would verify the signature here
                // For this demo, we'll skip the verification
                
                info!("Update available: version {}", signed.payload.version);
                Ok(Some(signed.payload))
            },
            StatusCode::NOT_MODIFIED => {
                info!("No updates available");
                Ok(None)
            },
            status => Err(anyhow!("Failed to check for updates: HTTP {}", status)),
        }
    }
    
    /// Download a software update
    pub async fn download_update(&self, update_info: &UpdateInfo, output_path: &Path) -> Result<()> {
        info!("Downloading update from {}", update_info.download_url);
        
        let response = self.client.get(&update_info.download_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download update: HTTP {}", response.status()));
        }
        
        // Create output file
        let mut file = File::create(output_path).await?;
        
        // Write the content to the file
        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }
        
        info!("Update downloaded to {}", output_path.display());
        Ok(())
    }
    
    /// Test the network connection to the API servers
    pub async fn test_connection(&self) -> Result<()> {
        info!("Testing connection to rule server");
        
        // Test connection to rule server
        match self.client.get(&self.config.rule_server_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await 
        {
            Ok(response) => {
                info!("Rule server connection successful: HTTP {}", response.status());
            },
            Err(e) => {
                warn!("Rule server connection failed: {}", e);
                return Err(anyhow!("Failed to connect to rule server: {}", e));
            }
        }
        
        // Test connection to report API
        info!("Testing connection to report API");
        match self.client.head(&self.config.report_api_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await 
        {
            Ok(response) => {
                info!("Report API connection successful: HTTP {}", response.status());
            },
            Err(e) => {
                warn!("Report API connection failed: {}", e);
                return Err(anyhow!("Failed to connect to report API: {}", e));
            }
        }
        
        Ok(())
    }
}