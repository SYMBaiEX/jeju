//! Contract bindings and provider for interacting with Jeju Network contracts
//!
//! Uses alloy for type-safe contract interactions.

use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::sol;
use alloy::transports::http::{Client, Http};
use std::str::FromStr;
use std::sync::Arc;

// Generate type-safe bindings for NodeStakingManager
sol! {
    #[sol(rpc)]
    interface INodeStakingManager {
        struct NodeStake {
            address operator;
            bytes32 nodeId;
            address stakingToken;
            uint256 stakedAmount;
            uint256 stakedValueUSD;
            address rewardToken;
            uint256 pendingRewards;
            uint256 claimedRewards;
            uint256 lastClaimTime;
            uint256 registeredAt;
            string rpcUrl;
            uint8 region;
            bool isActive;
        }

        function getNodeStake(bytes32 nodeId) external view returns (NodeStake memory);
        function getOperatorNodes(address operator) external view returns (bytes32[] memory);
        function registerNode(
            address stakingToken,
            uint256 stakeAmount,
            address rewardToken,
            string calldata rpcUrl,
            uint8 region
        ) external returns (bytes32 nodeId);
        function addStake(bytes32 nodeId, uint256 amount) external;
        function initiateUnstake(bytes32 nodeId, uint256 amount) external;
        function completeUnstake(bytes32 nodeId) external;
        function claimRewards(bytes32 nodeId) external returns (uint256 amount);
        function getPendingRewards(bytes32 nodeId) external view returns (uint256);
        function getTotalStakedUSD() external view returns (uint256);
    }

    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function transfer(address to, uint256 amount) external returns (bool);
    }

    #[sol(rpc)]
    interface IIdentityRegistry {
        struct AgentInfo {
            address owner;
            uint256 agentId;
            string tokenURI;
            uint256 reputation;
            bool isBanned;
            uint256 banExpiry;
            string banReason;
        }

        function register(string calldata tokenURI, uint256 stakeAmount) external returns (uint256 agentId);
        function getAgentInfo(uint256 agentId) external view returns (AgentInfo memory);
        function getAgentByOwner(address owner) external view returns (uint256 agentId);
        function getBanStatus(uint256 agentId) external view returns (bool banned, uint256 expiry, string memory reason);
    }

    #[sol(rpc)]
    interface IBanManager {
        function isBanned(uint256 agentId) external view returns (bool);
        function isOnNotice(uint256 agentId) external view returns (bool);
        function isPermanentlyBanned(uint256 agentId) external view returns (bool);
        function getBanInfo(uint256 agentId) external view returns (
            bool banned,
            uint256 expiry,
            string memory reason,
            bool canAppeal
        );
    }

    #[sol(rpc)]
    interface IComputeRegistry {
        function register(string calldata name, string calldata endpoint, bytes32 attestationHash) external payable;
        function registerWithAgent(string calldata name, string calldata endpoint, bytes32 attestationHash, uint256 agentId) external payable;
        function addCapability(string calldata model, uint256 pricePerInputToken, uint256 pricePerOutputToken, uint256 maxContextLength) external;
        function deactivate() external;
        function reactivate() external;
        function getProvider(address provider) external view returns (
            address owner,
            string memory name,
            string memory endpoint,
            bytes32 attestationHash,
            uint256 stake,
            uint256 registeredAt,
            uint256 agentId,
            bytes32 serviceType,
            bool active
        );
        function getProviderByAgent(uint256 agentId) external view returns (address);
        function minProviderStake() external view returns (uint256);
    }

}

// Separate sol! block for IIdentityRegistryV2 to avoid naming conflicts with IIdentityRegistry
sol! {
    #[sol(rpc)]
    interface IIdentityRegistryV2 {
        function register(string calldata tokenURI) external returns (uint256 agentId);
        function agentExists(uint256 agentId) external view returns (bool);
        function ownerOf(uint256 agentId) external view returns (address);
    }
}

/// Client for interacting with Jeju Network contracts
pub struct ContractClient {
    provider: Arc<RootProvider<Http<Client>>>,
    addresses: ContractAddresses,
}

/// Contract addresses for a specific network
#[derive(Clone)]
pub struct ContractAddresses {
    pub node_staking_manager: Address,
    pub identity_registry: Address,
    pub ban_manager: Address,
    pub jeju_token: Address,
    pub compute_staking: Address,
    pub compute_registry: Address,
}

impl ContractAddresses {
    /// Create from config
    pub fn from_config(config: &crate::config::ContractsConfig) -> Result<Self, String> {
        Ok(Self {
            node_staking_manager: Address::from_str(&config.node_staking_manager)
                .map_err(|e| format!("Invalid node_staking_manager address: {}", e))?,
            identity_registry: Address::from_str(&config.identity_registry)
                .map_err(|e| format!("Invalid identity_registry address: {}", e))?,
            ban_manager: Address::from_str(&config.ban_manager)
                .map_err(|e| format!("Invalid ban_manager address: {}", e))?,
            jeju_token: Address::from_str(&config.jeju_token)
                .map_err(|e| format!("Invalid jeju_token address: {}", e))?,
            compute_staking: Address::from_str(&config.compute_staking)
                .map_err(|e| format!("Invalid compute_staking address: {}", e))?,
            compute_registry: Address::from_str(&config.compute_registry)
                .map_err(|e| format!("Invalid compute_registry address: {}", e))?,
        })
    }

    /// Get contract addresses for localnet (chainId 31337)
    /// These must match packages/config/contracts.json
    pub fn localnet() -> Self {
        Self::from_config(&crate::config::ContractsConfig::localnet())
            .expect("valid localnet addresses")
    }

    /// Get contract addresses based on chain ID (fallback if no config provided)
    pub fn for_chain(chain_id: u64) -> Self {
        Self::from_config(&crate::config::ContractsConfig::for_chain(chain_id))
            .expect("valid addresses for chain")
    }
}

impl ContractClient {
    /// Create a new contract client with config
    pub async fn new_with_config(
        rpc_url: &str,
        contracts_config: &crate::config::ContractsConfig,
    ) -> Result<Self, String> {
        let provider = ProviderBuilder::new().on_http(
            rpc_url
                .parse()
                .map_err(|e| format!("Invalid RPC URL: {}", e))?,
        );

        Ok(Self {
            provider: Arc::new(provider),
            addresses: ContractAddresses::from_config(contracts_config)?,
        })
    }

    /// Create a new contract client (legacy, uses chain_id to determine addresses)
    pub async fn new(rpc_url: &str, chain_id: u64) -> Result<Self, String> {
        let provider = ProviderBuilder::new().on_http(
            rpc_url
                .parse()
                .map_err(|e| format!("Invalid RPC URL: {}", e))?,
        );

        Ok(Self {
            provider: Arc::new(provider),
            addresses: ContractAddresses::for_chain(chain_id),
        })
    }

    /// Get ETH balance for an address
    pub async fn get_eth_balance(&self, address: Address) -> Result<U256, String> {
        self.provider
            .get_balance(address)
            .await
            .map_err(|e| format!("Failed to get balance: {}", e))
    }

    /// Get JEJU token balance for an address
    pub async fn get_jeju_balance(&self, address: Address) -> Result<U256, String> {
        let token = IERC20::new(self.addresses.jeju_token, &*self.provider);
        token
            .balanceOf(address)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| format!("Failed to get JEJU balance: {}", e))
    }

    /// Get staking info for an operator
    pub async fn get_staking_info(&self, operator: Address) -> Result<Vec<NodeStakeInfo>, String> {
        let staking =
            INodeStakingManager::new(self.addresses.node_staking_manager, &*self.provider);

        // Get all node IDs for the operator
        let node_ids = staking
            .getOperatorNodes(operator)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| format!("Failed to get operator nodes: {}", e))?;

        let mut stakes = Vec::new();
        for node_id in node_ids {
            let stake = staking
                .getNodeStake(node_id)
                .call()
                .await
                .map(|r| r._0)
                .map_err(|e| format!("Failed to get node stake: {}", e))?;

            stakes.push(NodeStakeInfo {
                node_id: format!("0x{}", hex::encode(node_id)),
                staked_amount: stake.stakedAmount.to_string(),
                staked_value_usd: stake.stakedValueUSD.to_string(),
                pending_rewards: stake.pendingRewards.to_string(),
                staking_token: format!("{:?}", stake.stakingToken),
            });
        }

        Ok(stakes)
    }

    /// Get agent info by ID
    pub async fn get_agent_info(&self, agent_id: u64) -> Result<AgentInfoResult, String> {
        let registry = IIdentityRegistry::new(self.addresses.identity_registry, &*self.provider);
        let info = registry
            .getAgentInfo(U256::from(agent_id))
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| format!("Failed to get agent info: {}", e))?;

        Ok(AgentInfoResult {
            owner: format!("{:?}", info.owner),
            token_uri: info.tokenURI,
            reputation: info.reputation.to_string(),
            is_banned: info.isBanned,
            ban_reason: info.banReason,
        })
    }

    /// Get agent ID for an owner address
    pub async fn get_agent_by_owner(&self, owner: Address) -> Result<Option<u64>, String> {
        let registry = IIdentityRegistry::new(self.addresses.identity_registry, &*self.provider);
        let agent_id = registry
            .getAgentByOwner(owner)
            .call()
            .await
            .map(|r| r.agentId)
            .map_err(|e| format!("Failed to get agent by owner: {}", e))?;

        if agent_id == U256::ZERO {
            Ok(None)
        } else {
            Ok(Some(agent_id.to::<u64>()))
        }
    }

    /// Check ban status for an agent
    pub async fn get_ban_status(&self, agent_id: u64) -> Result<BanStatusResult, String> {
        let ban_manager = IBanManager::new(self.addresses.ban_manager, &*self.provider);
        let (banned, expiry, reason, can_appeal) = ban_manager
            .getBanInfo(U256::from(agent_id))
            .call()
            .await
            .map(|r| (r.banned, r.expiry, r.reason, r.canAppeal))
            .map_err(|e| format!("Failed to get ban info: {}", e))?;

        let is_permanent = ban_manager
            .isPermanentlyBanned(U256::from(agent_id))
            .call()
            .await
            .map(|r| r._0)
            .unwrap_or(false);

        let on_notice = ban_manager
            .isOnNotice(U256::from(agent_id))
            .call()
            .await
            .map(|r| r._0)
            .unwrap_or(false);

        Ok(BanStatusResult {
            is_banned: banned,
            is_permanent,
            is_on_notice: on_notice,
            expiry: expiry.to::<u64>(),
            reason,
            can_appeal,
        })
    }

    /// Get compute provider info
    pub async fn get_compute_provider(
        &self,
        provider: Address,
    ) -> Result<Option<ComputeProviderResult>, String> {
        let registry = IComputeRegistry::new(self.addresses.compute_registry, &*self.provider);

        let result = registry
            .getProvider(provider)
            .call()
            .await
            .map_err(|e| format!("Failed to get provider: {}", e))?;

        // Check if provider is registered (registeredAt > 0)
        if result.registeredAt == U256::ZERO {
            return Ok(None);
        }

        Ok(Some(ComputeProviderResult {
            address: format!("{:?}", provider),
            name: result.name,
            endpoint: result.endpoint,
            agent_id: result.agentId.to::<u64>(),
            stake: result.stake.to_string(),
            is_active: result.active,
            registered_at: result.registeredAt.to::<u64>(),
        }))
    }

    /// Get minimum stake required for compute registration
    pub async fn get_min_compute_stake(&self) -> Result<String, String> {
        let registry = IComputeRegistry::new(self.addresses.compute_registry, &*self.provider);

        let min_stake = registry
            .minProviderStake()
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| format!("Failed to get min stake: {}", e))?;

        Ok(min_stake.to_string())
    }
}

/// Result structure for node stake info
#[derive(Debug, Clone)]
pub struct NodeStakeInfo {
    pub node_id: String,
    pub staked_amount: String,
    pub staked_value_usd: String,
    pub pending_rewards: String,
    pub staking_token: String,
}

/// Result structure for agent info
#[derive(Debug, Clone)]
pub struct AgentInfoResult {
    pub owner: String,
    pub token_uri: String,
    pub reputation: String,
    pub is_banned: bool,
    pub ban_reason: String,
}

/// Result structure for ban status
#[derive(Debug, Clone)]
pub struct BanStatusResult {
    pub is_banned: bool,
    pub is_permanent: bool,
    pub is_on_notice: bool,
    pub expiry: u64,
    pub reason: String,
    pub can_appeal: bool,
}

/// Result structure for compute provider info
#[derive(Debug, Clone)]
pub struct ComputeProviderResult {
    pub address: String,
    pub name: String,
    pub endpoint: String,
    pub agent_id: u64,
    pub stake: String,
    pub is_active: bool,
    pub registered_at: u64,
}
