use crate::{
	models::{BlockChainType, Monitor},
	repositories::{NetworkRepository, NetworkService},
	services::{
		blockchain::{BlockChainClient, ClientPoolTrait},
		filter::FilterService,
	},
	utils::monitor::MonitorExecutionError,
};
use std::sync::Arc;

pub type ExecutionResult<T> = std::result::Result<T, MonitorExecutionError>;

/// Executes a monitor against a specific block number on a blockchain network.
///
/// This function allows testing monitors by running them against historical blocks.
/// It supports both EVM and Stellar networks, retrieving the block data and applying
/// the monitor's filters to check for matches.
///
/// # Arguments
///
/// * `monitor_name` - The name of the monitor to execute
/// * `network_slug` - The network identifier to run the monitor against
/// * `block_number` - The specific block number to analyze
/// * `active_monitors` - List of currently active monitors
///
/// # Returns
/// * `Result<String, ExecutionError>` - JSON string containing matches or error
pub async fn execute_monitor<T: ClientPoolTrait>(
	monitor_name: &str,
	network_slug: &str,
	block_number: &u64,
	active_monitors: Vec<Monitor>,
	client_pool: T,
) -> ExecutionResult<String> {
	// Initialize filter service
	let filter_service = Arc::new(FilterService::new());
	// Initialize network service
	let network_repository = NetworkRepository::new(None).map_err(|e| {
		MonitorExecutionError::execution_error(
			format!("Failed to initialize network repository: {}", e),
			None,
			None,
		)
	})?;
	let network_service = NetworkService::new_with_repository(network_repository).map_err(|e| {
		MonitorExecutionError::execution_error(
			format!("Failed to create network service: {}", e),
			None,
			None,
		)
	})?;

	// Get monitor from active monitors
	let monitor = active_monitors
		.iter()
		.find(|m| m.name == monitor_name)
		.ok_or_else(|| {
			MonitorExecutionError::not_found(
				format!("Monitor '{}' not found", monitor_name),
				None,
				None,
			)
		})?;

	if !monitor.networks.contains(&network_slug.to_string()) {
		return Err(MonitorExecutionError::not_found(
			format!(
				"Network '{}' not configured for monitor '{}'",
				network_slug, monitor_name
			),
			None,
			None,
		));
	}

	// Get network configuration
	let network = network_service.get(network_slug).ok_or_else(|| {
		MonitorExecutionError::not_found(
			format!("Network '{}' not found", network_slug),
			None,
			None,
		)
	})?;

	let matches = match network.network_type {
		BlockChainType::EVM => {
			let client = client_pool.get_evm_client(&network).await.map_err(|e| {
				MonitorExecutionError::execution_error(
					format!("Failed to get EVM client: {}", e),
					None,
					None,
				)
			})?;

			let block = client
				.get_block_by_number(block_number)
				.await
				.map_err(|e| {
					MonitorExecutionError::execution_error(
						format!("Failed to get block {}: {}", block_number, e),
						None,
						None,
					)
				})?;

			let block = block.ok_or_else(|| {
				MonitorExecutionError::execution_error(
					format!("Block {} not found", block_number),
					None,
					None,
				)
			})?;

			filter_service
				.filter_block(&*client, &network, &block, &[monitor.clone()])
				.await
				.map_err(|e| {
					MonitorExecutionError::execution_error(
						format!("Failed to filter block: {}", e),
						None,
						None,
					)
				})?
		}

		BlockChainType::Stellar => {
			let client = client_pool
				.get_stellar_client(&network)
				.await
				.map_err(|e| {
					MonitorExecutionError::execution_error(
						format!("Failed to get Stellar client: {}", e),
						None,
						None,
					)
				})?;

			let block = client
				.get_block_by_number(block_number)
				.await
				.map_err(|e| {
					MonitorExecutionError::execution_error(
						format!("Failed to get block {}: {}", block_number, e),
						None,
						None,
					)
				})?;

			let block = block.ok_or_else(|| {
				MonitorExecutionError::execution_error(
					format!("Block {} not found", block_number),
					None,
					None,
				)
			})?;

			filter_service
				.filter_block(&*client, &network, &block, &[monitor.clone()])
				.await
				.map_err(|e| {
					MonitorExecutionError::execution_error(
						format!("Failed to filter block: {}", e),
						None,
						None,
					)
				})?
		}
		BlockChainType::Midnight => {
			return Err(MonitorExecutionError::execution_error(
				"Midnight network not supported",
				None,
				None,
			))
		}
		BlockChainType::Solana => {
			return Err(MonitorExecutionError::execution_error(
				"Solana network not supported",
				None,
				None,
			))
		}
	};

	let json_matches = serde_json::to_string(&matches).unwrap();
	Ok(json_matches)
}
