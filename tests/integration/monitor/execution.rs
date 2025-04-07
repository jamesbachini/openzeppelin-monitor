use crate::integration::{
	filters::common::{load_test_data, setup_network_service},
	mocks::{
		create_test_network, MockClientPool, MockEvmClientTrait, MockNetworkRepository,
		MockStellarClientTrait,
	},
};
use openzeppelin_monitor::{
	models::{BlockChainType, EVMTransactionReceipt},
	repositories::NetworkService,
	services::filter::FilterService,
	utils::monitor::execution::execute_monitor,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

fn setup_mocked_networks(
	network_name: &str,
	network_slug: &str,
	block_chain_type: BlockChainType,
) -> NetworkService<MockNetworkRepository> {
	let mut mocked_networks = HashMap::new();
	mocked_networks.insert(
		network_slug.to_string(),
		create_test_network(network_name, network_slug, block_chain_type),
	);
	setup_network_service(mocked_networks)
}

#[tokio::test]
async fn test_execute_monitor_evm() {
	let test_data = load_test_data("evm");
	let mock_network_service =
		setup_mocked_networks("Ethereum", "ethereum_mainnet", BlockChainType::EVM);
	let mut mock_pool = MockClientPool::new();
	let mut mock_client = MockEvmClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Ok(Some(test_data.blocks[0].clone())));

	let receipts = test_data.receipts.clone();
	let receipt_map: std::collections::HashMap<String, EVMTransactionReceipt> = receipts
		.iter()
		.map(|r| (format!("0x{:x}", r.transaction_hash), r.clone()))
		.collect();

	let receipt_map = Arc::new(receipt_map);
	mock_client
		.expect_get_transaction_receipt()
		.returning(move |hash| {
			let receipt_map = Arc::clone(&receipt_map);
			Ok(receipt_map
				.get(&hash)
				.cloned()
				.unwrap_or_else(|| panic!("Receipt not found for hash: {}", hash)))
		});

	mock_pool
		.expect_get_evm_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 21305050;

	let result = execute_monitor(
		&test_data.monitor.name,
		"ethereum_mainnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(
		result.is_ok(),
		"Monitor execution failed: {:?}",
		result.err()
	);

	// Parse the JSON result and add more specific assertions based on expected matches
	let matches: Vec<serde_json::Value> = serde_json::from_str(&result.unwrap()).unwrap();
	assert!(matches.len() == 1);
}

#[tokio::test]
async fn test_execute_monitor_evm_wrong_network() {
	let test_data = load_test_data("evm");
	let mut mock_pool = MockClientPool::new();
	let mock_network_service =
		setup_mocked_networks("Ethereum", "ethereum_mainnet", BlockChainType::EVM);
	let mock_client = MockEvmClientTrait::new();

	mock_pool
		.expect_get_evm_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 22197425;

	let result = execute_monitor(
		&test_data.monitor.name,
		"ethereum_goerli",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_evm_wrong_block_number() {
	let test_data = load_test_data("evm");
	let mut mock_pool = MockClientPool::new();
	let mock_network_service =
		setup_mocked_networks("Ethereum", "ethereum_mainnet", BlockChainType::EVM);
	let mut mock_client = MockEvmClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Ok(None));

	mock_pool
		.expect_get_evm_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 1;

	let result = execute_monitor(
		&test_data.monitor.name,
		"ethereum_mainnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_evm_failed_to_get_block_by_number() {
	let test_data = load_test_data("evm");
	let mut mock_pool = MockClientPool::new();
	let mock_network_service =
		setup_mocked_networks("Ethereum", "ethereum_mainnet", BlockChainType::EVM);
	let mut mock_client = MockEvmClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Err(anyhow::anyhow!("Failed to get block by number")));

	mock_pool
		.expect_get_evm_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 1;

	let result = execute_monitor(
		&test_data.monitor.name,
		"ethereum_mainnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_evm_failed_to_get_evm_client() {
	let test_data = load_test_data("evm");
	let mut mock_pool = MockClientPool::new();
	let mock_network_service =
		setup_mocked_networks("Ethereum", "ethereum_mainnet", BlockChainType::EVM);

	mock_pool
		.expect_get_evm_client()
		.return_once(move |_| Err(anyhow::anyhow!("Failed to get evm client")));

	let block_number = 1;

	let result = execute_monitor(
		&test_data.monitor.name,
		"ethereum_mainnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_stellar() {
	let test_data = load_test_data("stellar");
	let mock_network_service =
		setup_mocked_networks("Stellar", "stellar_testnet", BlockChainType::Stellar);

	let mut mock_pool = MockClientPool::new();
	let mut mock_client = MockStellarClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Ok(Some(test_data.blocks[0].clone())));
	mock_client
		.expect_get_transactions()
		.return_once(move |_, _| Ok(test_data.stellar_transactions.clone()));
	mock_client
		.expect_get_events()
		.return_once(move |_, _| Ok(test_data.stellar_events.clone()));

	mock_pool
		.expect_get_stellar_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 172627;

	let result = execute_monitor(
		&test_data.monitor.name,
		"stellar_testnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(
		result.is_ok(),
		"Monitor execution failed: {:?}",
		result.err()
	);

	// Parse the JSON result and add more specific assertions based on expected matches
	let matches: Vec<serde_json::Value> = serde_json::from_str(&result.unwrap()).unwrap();
	assert!(matches.len() == 1);
}

#[tokio::test]
async fn test_execute_monitor_not_found() {
	let test_data = load_test_data("stellar");
	let mock_network_service =
		setup_mocked_networks("Stellar", "stellar_testnet", BlockChainType::Stellar);
	let mut mock_pool = MockClientPool::new();
	let mock_client = MockStellarClientTrait::new();
	mock_pool
		.expect_get_stellar_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));
	let block_number = 172627;

	let result = execute_monitor(
		"wrong_monitor",
		"stellar_testnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_failed_to_get_block() {
	let test_data = load_test_data("stellar");
	let mock_network_service =
		setup_mocked_networks("Stellar", "stellar_testnet", BlockChainType::Stellar);
	let mut mock_pool = MockClientPool::new();
	let mut mock_client = MockStellarClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Ok(None));

	mock_pool
		.expect_get_stellar_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 172627;

	let result = execute_monitor(
		&test_data.monitor.name,
		"stellar_testnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_failed_to_get_stellar_client() {
	let test_data = load_test_data("stellar");
	let mock_network_service =
		setup_mocked_networks("Stellar", "stellar_testnet", BlockChainType::Stellar);
	let mut mock_pool = MockClientPool::new();

	mock_pool
		.expect_get_stellar_client()
		.return_once(move |_| Err(anyhow::anyhow!("Failed to get stellar client")));

	let block_number = 172627;

	let result = execute_monitor(
		&test_data.monitor.name,
		"stellar_testnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_monitor_failed_to_get_block_by_number() {
	let test_data = load_test_data("stellar");
	let mock_network_service =
		setup_mocked_networks("Stellar", "stellar_testnet", BlockChainType::Stellar);
	let mut mock_pool = MockClientPool::new();
	let mut mock_client = MockStellarClientTrait::new();

	mock_client
		.expect_get_block_by_number()
		.return_once(move |_| Err(anyhow::anyhow!("Failed to get block by number")));

	mock_pool
		.expect_get_stellar_client()
		.return_once(move |_| Ok(Arc::new(mock_client)));

	let block_number = 172627;

	let result = execute_monitor(
		&test_data.monitor.name,
		"stellar_testnet",
		&block_number,
		vec![test_data.monitor.clone()],
		Arc::new(Mutex::new(mock_network_service)),
		Arc::new(FilterService::new()),
		mock_pool,
	)
	.await;
	assert!(result.is_err());
}
