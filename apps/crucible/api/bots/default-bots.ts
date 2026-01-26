/**
 * Default Bot Configurations
 * Network-specific bot configurations and factory functions
 */

import type { Address } from 'viem'

export interface BotStrategy {
  type: string
  enabled: boolean
  minProfitBps?: number
  maxGasGwei?: number
  maxSlippageBps?: number
  cooldownMs?: number
}

export interface ChainConfig {
  chainId: number
  name: string
  nativeSymbol: string
  rpcUrl: string
}

export interface DefaultBotConfig {
  name: string
  description: string
  strategies: BotStrategy[]
  initialFunding: string
}

export interface TradingBotOptions {
  agentId: bigint
  name: string
  strategies: BotStrategy[]
  chains: ChainConfig[]
  maxConcurrentExecutions: number
  useFlashbots: boolean
  treasuryAddress?: Address
}

const LOCALNET_BOTS: DefaultBotConfig[] = [
  {
    name: 'Test Arbitrage Bot',
    description: 'Local testing bot for DEX arbitrage',
    strategies: [
      {
        type: 'DEX_ARBITRAGE',
        enabled: true,
        minProfitBps: 10,
        maxGasGwei: 100,
        maxSlippageBps: 50,
        cooldownMs: 5000,
      },
    ],
    initialFunding: '1.0',
  },
]

const TESTNET_BOTS: DefaultBotConfig[] = [
  {
    name: 'Testnet Arbitrage Bot',
    description: 'Testnet bot for DEX arbitrage testing',
    strategies: [
      {
        type: 'DEX_ARBITRAGE',
        enabled: true,
        minProfitBps: 20,
        maxGasGwei: 50,
        maxSlippageBps: 100,
        cooldownMs: 30000,
      },
    ],
    initialFunding: '0.5',
  },
]

const MAINNET_BOTS: DefaultBotConfig[] = []

// Combined default bots for iteration/testing
export const DEFAULT_BOTS: DefaultBotConfig[] = [...LOCALNET_BOTS, ...TESTNET_BOTS]

// Chain configurations by network
export const DEFAULT_CHAINS: Record<string, { chainId: number; name: string; rpcUrl: string; blockTime: number }> = {
  localnet: { chainId: 31337, name: 'Localnet', rpcUrl: 'http://127.0.0.1:6546', blockTime: 1 },
  testnet: { chainId: 420690, name: 'Jeju Testnet', rpcUrl: 'https://testnet-rpc.jejunetwork.org', blockTime: 2 },
  mainnet: { chainId: 420691, name: 'Jeju Mainnet', rpcUrl: 'https://rpc.jejunetwork.org', blockTime: 2 },
}

export function getDefaultBotsForNetwork(network: string): DefaultBotConfig[] {
  switch (network) {
    case 'localnet':
      return LOCALNET_BOTS
    case 'testnet':
      return TESTNET_BOTS
    case 'mainnet':
      return MAINNET_BOTS
    default:
      return []
  }
}

export function createTradingBotOptions(
  config: DefaultBotConfig,
  agentId: bigint,
  network: string,
  treasuryAddress?: Address,
): TradingBotOptions {
  const chains: ChainConfig[] = []

  switch (network) {
    case 'localnet':
      chains.push({
        chainId: 31337,
        name: 'Localnet',
        nativeSymbol: 'ETH',
        rpcUrl: 'http://127.0.0.1:6546',
      })
      break
    case 'testnet':
      chains.push({
        chainId: 420690,
        name: 'Jeju Testnet',
        nativeSymbol: 'ETH',
        rpcUrl: 'https://testnet-rpc.jejunetwork.org',
      })
      break
    case 'mainnet':
      chains.push({
        chainId: 420691,
        name: 'Jeju Mainnet',
        nativeSymbol: 'ETH',
        rpcUrl: 'https://rpc.jejunetwork.org',
      })
      break
  }

  return {
    agentId,
    name: config.name,
    strategies: config.strategies,
    chains,
    maxConcurrentExecutions: 5,
    useFlashbots: network === 'mainnet',
    treasuryAddress,
  }
}
