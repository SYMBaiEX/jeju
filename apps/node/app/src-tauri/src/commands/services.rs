//! Service management commands

use crate::hardware::HardwareDetector;
use crate::services::{ServiceId, ServiceMetadata, ServiceState};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartServiceRequest {
    pub service_id: String,
    pub auto_stake: bool,
    pub stake_amount: Option<String>,
    pub custom_settings: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceWithStatus {
    pub metadata: ServiceMetadata,
    pub status: ServiceState,
    pub meets_requirements: bool,
    pub requirement_issues: Vec<String>,
}

#[tauri::command]
pub async fn get_available_services(
    state: State<'_, AppState>,
) -> Result<Vec<ServiceWithStatus>, String> {
    tracing::info!("get_available_services called");
    let inner = state.inner.read().await;

    // Detect current hardware
    let mut detector = HardwareDetector::new();
    let hardware = detector.detect();
    tracing::info!("Hardware detected: {} cores, {} MB RAM", hardware.cpu.cores_physical, hardware.memory.total_mb);

    // Get all service statuses at once
    let all_statuses = inner.service_manager.get_all_status().await;
    tracing::info!("Got {} service statuses", all_statuses.len());

    // Get all services with their metadata and requirements
    let services: Vec<ServiceWithStatus> = inner
        .service_manager
        .get_available_services(&hardware)
        .into_iter()
        .map(|metadata| {
            let service_id: ServiceId = metadata.id.parse().unwrap_or(ServiceId::Compute);

            // Check requirements - use lower thresholds for basic services
            let reqs = match service_id {
                ServiceId::Compute => crate::hardware::ServiceRequirements {
                    min_cpu_cores: 1,
                    min_memory_mb: 8 * 1024, // 8 GB for testing
                    min_storage_gb: 50,
                    requires_gpu: false,
                    min_gpu_memory_mb: None,
                    requires_tee: false,
                    min_bandwidth_mbps: Some(10),
                },
                ServiceId::Sequencer => crate::hardware::ServiceRequirements {
                    min_cpu_cores: 4,
                    min_memory_mb: 16 * 1024,
                    min_storage_gb: 500,
                    requires_gpu: false,
                    min_gpu_memory_mb: None,
                    requires_tee: false,
                    min_bandwidth_mbps: Some(100),
                },
                _ => crate::hardware::ServiceRequirements {
                    min_cpu_cores: 1,
                    min_memory_mb: 2 * 1024,
                    min_storage_gb: 20,
                    requires_gpu: false,
                    min_gpu_memory_mb: None,
                    requires_tee: false,
                    min_bandwidth_mbps: Some(10),
                },
            };

            let (meets, issues) = detector.meets_requirements(&hardware, &reqs);

            // Get actual service status from the service manager
            let status = all_statuses
                .get(service_id.as_str())
                .cloned()
                .unwrap_or_else(|| ServiceState {
                    running: false,
                    uptime_seconds: 0,
                    requests_served: 0,
                    earnings_wei: "0".to_string(),
                    last_error: None,
                    health: "stopped".to_string(),
                });

            ServiceWithStatus {
                metadata: metadata.clone(),
                status,
                meets_requirements: meets,
                requirement_issues: issues,
            }
        })
        .collect();

    tracing::info!("Returning {} services: {:?}", services.len(), services.iter().map(|s| &s.metadata.id).collect::<Vec<_>>());
    Ok(services)
}

#[tauri::command]
pub async fn start_service(
    state: State<'_, AppState>,
    request: StartServiceRequest,
) -> Result<ServiceState, String> {
    let mut inner = state.inner.write().await;

    // Parse service ID
    let service_id: ServiceId = request.service_id.parse()?;

    // For compute service, extract values needed for configuration before modifying config
    let (dws_url, wallet_address) = if service_id == ServiceId::Compute {
        // Determine DWS URL based on network
        let dws = match inner.config.network.network.as_str() {
            "mainnet" => "https://dws.jejunetwork.org/compute",
            "testnet" => "https://dws.testnet.jejunetwork.org/compute",
            "localnet" => "http://127.0.0.1:4030/compute",
            _ => "https://dws.testnet.jejunetwork.org/compute",
        }.to_string();

        // Get wallet address if available
        let wallet = inner.wallet_manager
            .as_ref()
            .and_then(|wm| wm.get_info())
            .map(|info| info.address);

        (Some(dws), wallet)
    } else {
        (None, None)
    };

    // Get or create service config
    let config = inner
        .config
        .services
        .entry(request.service_id.clone())
        .or_insert_with(crate::config::ServiceConfig::default);

    config.enabled = true;
    config.stake_amount = request.stake_amount;

    if let Some(settings) = request.custom_settings {
        config.custom_settings = settings;
    }

    // For compute service, inject DWS URL and wallet address
    if service_id == ServiceId::Compute {
        if let Some(dws) = &dws_url {
            config.custom_settings.insert(
                "dws_url".to_string(),
                serde_json::Value::String(dws.clone()),
            );
        }

        if let Some(wallet) = &wallet_address {
            config.custom_settings.insert(
                "wallet_address".to_string(),
                serde_json::Value::String(wallet.clone()),
            );
        }

        // Default Ollama endpoint - use localhost since Ollama typically binds to 127.0.0.1
        // Can be overridden via OLLAMA_HOST environment variable
        if !config.custom_settings.contains_key("ollama_endpoint") {
            let ollama_host = std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
            config.custom_settings.insert(
                "ollama_endpoint".to_string(),
                serde_json::Value::String(ollama_host),
            );
        }

        // Default models
        if !config.custom_settings.contains_key("models") {
            config.custom_settings.insert(
                "models".to_string(),
                serde_json::json!(["tinyllama"]),
            );
        }

        tracing::info!(
            "Starting compute service with DWS URL: {:?}, wallet: {:?}",
            dws_url,
            wallet_address
        );
    }

    // Clone config for service start before saving
    let service_config = config.clone();

    // Save config
    inner.config.save().map_err(|e| e.to_string())?;

    // Start service
    inner
        .service_manager
        .start_service(service_id, &service_config)
        .await?;

    // Get status
    inner.service_manager.get_service_status(service_id).await
}

#[tauri::command]
pub async fn stop_service(
    state: State<'_, AppState>,
    service_id: String,
) -> Result<ServiceState, String> {
    let mut inner = state.inner.write().await;

    let id: ServiceId = service_id.parse()?;

    // Update config
    if let Some(config) = inner.config.services.get_mut(&service_id) {
        config.enabled = false;
    }
    inner.config.save().map_err(|e| e.to_string())?;

    // Stop service
    inner.service_manager.stop_service(id).await?;

    // Get status
    inner.service_manager.get_service_status(id).await
}

#[tauri::command]
pub async fn get_service_status(
    state: State<'_, AppState>,
    service_id: String,
) -> Result<ServiceState, String> {
    let inner = state.inner.read().await;

    let id: ServiceId = service_id.parse()?;
    inner.service_manager.get_service_status(id).await
}

#[tauri::command]
pub async fn get_all_service_status(
    state: State<'_, AppState>,
) -> Result<HashMap<String, ServiceState>, String> {
    let inner = state.inner.read().await;
    Ok(inner.service_manager.get_all_status().await)
}
