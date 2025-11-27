//! Authentication client for communicating with the backend API.
//!
//! This module handles JWT-based authentication with the SDChat backend,
//! including login, registration, token management, and user operations.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{info, warn, error};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::config::ConfigManager;
use crate::network::NetworkClient;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT access token
    pub access_token: Option<String>,
    /// JWT refresh token
    pub refresh_token: Option<String>,
    /// Token expiry time
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Remember login flag
    pub remember_login: bool,
    /// Last login attempt
    pub last_login_attempt: Option<DateTime<Utc>>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            remember_login: false,
            last_login_attempt: None,
        }
    }
}

/// User information from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
}

/// Login request data
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Register request data
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Change password request data
#[derive(Debug, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Login response from API
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: User,
}

/// API error response
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: Option<String>,
}

/// Authentication client
pub struct AuthClient {
    /// Network client for API requests
    network_client: Arc<RwLock<NetworkClient>>,
    /// Authentication configuration
    auth_config: Arc<RwLock<AuthConfig>>,
    /// Currently authenticated user
    current_user: Arc<RwLock<Option<User>>>,
    /// Configuration manager
    config_manager: Arc<ConfigManager>,
}

impl AuthClient {
    /// Create a new authentication client
    pub fn new(config_manager: Arc<ConfigManager>) -> Result<Self> {
        let network_client = Arc::new(RwLock::new(NetworkClient::new()?));
        let auth_config = Arc::new(RwLock::new(AuthConfig::default()));

        Ok(Self {
            network_client,
            auth_config,
            current_user: Arc::new(RwLock::new(None)),
            config_manager,
        })
    }

    /// Initialize auth client from saved configuration
    pub async fn initialize(&self) -> Result<()> {
        // Load saved auth config if it exists
        if let Ok(auth_config) = self.load_auth_config().await {
            *self.auth_config.write().await = auth_config;

            // Try to refresh token if needed
            if self.is_token_expired().await? {
                info!("Token expired, attempting refresh");
                self.refresh_access_token().await?;
            }
        }

        Ok(())
    }

    /// User registration
    pub async fn register(&self, username: String, email: String, password: String) -> Result<User> {
        info!("Registering user: {}", username);

        let register_request = RegisterRequest {
            username: username.clone(),
            email: email.clone(),
            password,
        };

        let network_client = self.network_client.read().await;
        let url = format!("{}/auth/register", self.get_base_url());

        let response = network_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&register_request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED => {
                let register_response: serde_json::Value = response.json().await?;
                let user_data = register_response.get("user_id")
                    .and_then(|v| v.as_u64())
                    .map(|id| User {
                        id: id as u32,
                        username,
                        email,
                        is_admin: false,
                        is_active: true,
                        created_at: None,
                    });

                if let Some(user) = user_data {
                    info!("User registered successfully: {}", username);
                    Ok(user)
                } else {
                    Err(anyhow!("Invalid registration response format"))
                }
            }
            StatusCode::CONFLICT => {
                let error: ApiError = response.json().await?;
                Err(anyhow!("Registration failed: {}", error.error))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Registration failed with status {}: {}", status, error_text))
            }
        }
    }

    /// User login
    pub async fn login(&self, username: String, password: String, remember: bool) -> Result<User> {
        info!("Attempting login for user: {}", username);

        let login_request = LoginRequest {
            username: username.clone(),
            password,
        };

        let network_client = self.network_client.read().await;
        let url = format!("{}/auth/login", self.get_base_url());

        let response = network_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&login_request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let login_response: LoginResponse = response.json().await?;

                // Update auth config
                {
                    let mut auth_config = self.auth_config.write().await;
                    auth_config.access_token = Some(login_response.access_token.clone());
                    auth_config.refresh_token = Some(login_response.refresh_token.clone());
                    auth_config.token_expires_at = Some(Utc::now() + chrono::Duration::hours(1));
                    auth_config.remember_login = remember;
                    auth_config.last_login_attempt = Some(Utc::now());
                }

                // Update current user
                {
                    *self.current_user.write().await = Some(login_response.user.clone());
                }

                // Save auth config if remember is true
                if remember {
                    self.save_auth_config().await?;
                }

                info!("User logged in successfully: {}", username);
                Ok(login_response.user)
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Invalid username or password"))
            }
            StatusCode::FORBIDDEN => {
                Err(anyhow!("Account is disabled"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Login failed with status {}: {}", status, error_text))
            }
        }
    }

    /// User logout
    pub async fn logout(&self) -> Result<()> {
        info!("Logging out current user");

        // Clear auth config
        {
            let mut auth_config = self.auth_config.write().await;
            auth_config.access_token = None;
            auth_config.refresh_token = None;
            auth_config.token_expires_at = None;
            auth_config.last_login_attempt = None;
        }

        // Clear current user
        {
            *self.current_user.write().await = None;
        }

        // Remove saved auth config file
        let config_path = self.get_auth_config_path();
        if let Err(e) = fs::remove_file(&config_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("Failed to remove auth config file: {}", e);
            }
        }

        info!("User logged out successfully");
        Ok(())
    }

    /// Refresh access token
    pub async fn refresh_access_token(&self) -> Result<()> {
        info!("Refreshing access token");

        let refresh_token = {
            let auth_config = self.auth_config.read().await;
            auth_config.refresh_token.clone()
        };

        if let Some(token) = refresh_token {
            let network_client = self.network_client.read().await;
            let url = format!("{}/auth/refresh", self.get_base_url());

            let response = network_client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;

            match response.status() {
                StatusCode::OK => {
                    let refresh_response: serde_json::Value = response.json().await?;

                    if let Some(access_token) = refresh_response.get("access_token")
                        .and_then(|v| v.as_str())
                    {
                        // Update access token
                        {
                            let mut auth_config = self.auth_config.write().await;
                            auth_config.access_token = Some(access_token.to_string());
                            auth_config.token_expires_at = Some(Utc::now() + chrono::Duration::hours(1));
                        }

                        info!("Access token refreshed successfully");
                        Ok(())
                    } else {
                        Err(anyhow!("Invalid refresh response format"))
                    }
                }
                StatusCode::UNAUTHORIZED => {
                    // Refresh token is invalid, clear auth
                    self.logout().await?;
                    Err(anyhow!("Refresh token expired, please login again"))
                }
                status => {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(anyhow!("Token refresh failed with status {}: {}", status, error_text))
                }
            }
        } else {
            Err(anyhow!("No refresh token available"))
        }
    }

    /// Change password
    pub async fn change_password(&self, current_password: String, new_password: String) -> Result<()> {
        info!("Changing password");

        let change_request = ChangePasswordRequest {
            current_password,
            new_password,
        };

        let network_client = self.network_client.read().await;
        let access_token = self.get_access_token().await?;

        let url = format!("{}/auth/change-password", self.get_base_url());

        let response = network_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&change_request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                info!("Password changed successfully");
                Ok(())
            }
            StatusCode::UNAUTHORIZED => {
                Err(anyhow!("Current password is incorrect"))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("Password change failed with status {}: {}", status, error_text))
            }
        }
    }

    /// Get current user information
    pub async fn get_current_user(&self) -> Result<User> {
        let current_user = self.current_user.read().await.clone();

        if let Some(user) = current_user {
            Ok(user)
        } else {
            Err(anyhow!("No user is currently logged in"))
        }
    }

    /// Check if user is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.current_user.read().await.is_some() &&
        !self.is_token_expired().await.unwrap_or(true)
    }

    /// Check if user is admin
    pub async fn is_admin(&self) -> bool {
        if let Some(user) = self.current_user.read().await.clone() {
            user.is_admin
        } else {
            false
        }
    }

    /// Get access token for API requests
    pub async fn get_access_token(&self) -> Result<String> {
        // Check if token is expired and try to refresh
        if self.is_token_expired().await? {
            self.refresh_access_token().await?;
        }

        let auth_config = self.auth_config.read().await;
        auth_config.access_token
            .clone()
            .ok_or_else(|| anyhow!("No access token available"))
    }

    /// Check if token is expired
    async fn is_token_expired(&self) -> Result<bool> {
        let auth_config = self.auth_config.read().await;

        if let Some(expires_at) = auth_config.token_expires_at {
            Ok(Utc::now() >= expires_at - chrono::Duration::minutes(5)) // 5 min buffer
        } else {
            Ok(true) // No expiry time, assume expired
        }
    }

    /// Get base URL for API
    fn get_base_url(&self) -> String {
        self.config_manager.config().server.base_url.clone()
    }

    /// Get path for auth config file
    fn get_auth_config_path(&self) -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("sdchat-scanner");
        path.push("auth.json");
        path
    }

    /// Save authentication configuration
    async fn save_auth_config(&self) -> Result<()> {
        let auth_config = self.auth_config.read().await;

        if auth_config.remember_login {
            let config_path = self.get_auth_config_path();

            // Create directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            let config_json = serde_json::to_string_pretty(&*auth_config)?;
            fs::write(&config_path, config_json).await?;

            info!("Authentication configuration saved");
        }

        Ok(())
    }

    /// Load authentication configuration
    async fn load_auth_config(&self) -> Result<AuthConfig> {
        let config_path = self.get_auth_config_path();

        if fs::metadata(&config_path).await.is_err() {
            return Ok(AuthConfig::default());
        }

        let config_content = fs::read_to_string(&config_path).await?;
        let auth_config: AuthConfig = serde_json::from_str(&config_content)?;

        info!("Authentication configuration loaded");
        Ok(auth_config)
    }

    /// Ensure authenticated before making API request
    pub async fn ensure_authenticated(&self) -> Result<()> {
        if !self.is_authenticated().await {
            Err(anyhow!("User is not authenticated"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_auth_config_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("auth.json");

        let auth_config = AuthConfig {
            access_token: Some("test_token".to_string()),
            refresh_token: Some("refresh_token".to_string()),
            token_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            remember_login: true,
            last_login_attempt: Some(Utc::now()),
        };

        // Save config
        let config_json = serde_json::to_string_pretty(&auth_config).unwrap();
        fs::write(&config_path, config_json).await.unwrap();

        // Load config
        let loaded_content = fs::read_to_string(&config_path).await.unwrap();
        let loaded_config: AuthConfig = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(auth_config.access_token, loaded_config.access_token);
        assert_eq!(auth_config.refresh_token, loaded_config.refresh_token);
        assert_eq!(auth_config.remember_login, loaded_config.remember_login);
    }
}