/**
 * ProposalService - Handles on-chain proposal submission via BoardGovernance contract
 */

import { getChainId, getRpcUrl } from '@jejunetwork/config'
import {
  type Address,
  type Hash,
  createPublicClient,
  createWalletClient,
  http,
  toHex,
} from 'viem'
import { privateKeyToAccount } from 'viem/accounts'
import { anvil, type Chain } from 'viem/chains'

// BoardGovernance ABI - minimal for proposal submission
const BOARD_GOVERNANCE_ABI = [
  {
    type: 'function',
    name: 'submitProposal',
    inputs: [
      { name: 'daoId', type: 'bytes32' },
      { name: 'proposalType', type: 'uint8' },
      { name: 'contentHash', type: 'bytes32' },
      { name: 'targetContract', type: 'address' },
      { name: 'callData', type: 'bytes' },
      { name: 'value', type: 'uint256' },
    ],
    outputs: [{ name: 'proposalId', type: 'bytes32' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function',
    name: 'getProposal',
    inputs: [{ name: 'proposalId', type: 'bytes32' }],
    outputs: [
      {
        name: '',
        type: 'tuple',
        components: [
          { name: 'proposalId', type: 'bytes32' },
          { name: 'proposer', type: 'address' },
          { name: 'proposerAgentId', type: 'uint256' },
          { name: 'proposalType', type: 'uint8' },
          { name: 'status', type: 'uint8' },
          { name: 'qualityScore', type: 'uint8' },
          { name: 'createdAt', type: 'uint256' },
          { name: 'autocratVoteEnd', type: 'uint256' },
          { name: 'gracePeriodEnd', type: 'uint256' },
          { name: 'contentHash', type: 'bytes32' },
          { name: 'targetContract', type: 'address' },
          { name: 'callData', type: 'bytes' },
          { name: 'value', type: 'uint256' },
          { name: 'totalStaked', type: 'uint256' },
          { name: 'totalReputation', type: 'uint256' },
          { name: 'backerCount', type: 'uint256' },
          { name: 'hasResearch', type: 'bool' },
          { name: 'researchHash', type: 'bytes32' },
          { name: 'directorApproved', type: 'bool' },
          { name: 'directorDecisionHash', type: 'bytes32' },
        ],
      },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function',
    name: 'getProposalsByDAO',
    inputs: [{ name: 'daoId', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bytes32[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function',
    name: 'proposalCount',
    inputs: [{ name: 'daoId', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'event',
    name: 'ProposalSubmitted',
    inputs: [
      { name: 'daoId', type: 'bytes32', indexed: true },
      { name: 'proposalId', type: 'bytes32', indexed: true },
      { name: 'proposer', type: 'address', indexed: true },
      { name: 'proposalType', type: 'uint8', indexed: false },
      { name: 'contentHash', type: 'bytes32', indexed: false },
    ],
  },
] as const

// Localnet BoardGovernance address - deployed above
const BOARD_GOVERNANCE_ADDRESS: Address =
  '0x8F4ec854Dd12F1fe79500a1f53D0cbB30f9b6134'

// Default operator key (localnet only)
const DEFAULT_OPERATOR_KEY =
  '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'

export interface ProposalSubmission {
  daoId: string
  proposalType: number
  contentHash: string
  targetContract?: Address
  callData?: `0x${string}`
  value?: bigint
}

export interface OnChainProposal {
  proposalId: string
  proposer: Address
  proposalType: number
  status: number
  createdAt: number
  contentHash: string
  targetContract: Address
  value: string
}

function getChain(): Chain {
  const chainId = getChainId()
  if (chainId === 31337) {
    return anvil
  }
  // Add other chains as needed
  return anvil
}

class ProposalService {
  private publicClient
  private walletClient
  private contractAddress: Address

  constructor() {
    const rpcUrl = getRpcUrl()
    const chain = getChain()
    const operatorKey = process.env.OPERATOR_PRIVATE_KEY ?? DEFAULT_OPERATOR_KEY
    const account = privateKeyToAccount(operatorKey as `0x${string}`)

    this.publicClient = createPublicClient({
      chain,
      transport: http(rpcUrl),
    })

    this.walletClient = createWalletClient({
      account,
      chain,
      transport: http(rpcUrl),
    })

    this.contractAddress = BOARD_GOVERNANCE_ADDRESS
  }

  /**
   * Submit a proposal on-chain
   */
  async submitProposal(params: ProposalSubmission): Promise<{
    txHash: Hash
    proposalId: string
  }> {
    // Convert daoId to bytes32 if it's not already
    const daoIdBytes = params.daoId.startsWith('0x')
      ? (params.daoId as `0x${string}`)
      : toHex(params.daoId, { size: 32 })

    // Convert contentHash to bytes32
    const contentHashBytes = params.contentHash.startsWith('0x')
      ? (params.contentHash as `0x${string}`)
      : toHex(params.contentHash, { size: 32 })

    const { request, result } = await this.publicClient.simulateContract({
      address: this.contractAddress,
      abi: BOARD_GOVERNANCE_ABI,
      functionName: 'submitProposal',
      args: [
        daoIdBytes,
        params.proposalType,
        contentHashBytes,
        params.targetContract ?? '0x0000000000000000000000000000000000000000',
        params.callData ?? '0x',
        params.value ?? 0n,
      ],
      account: this.walletClient.account,
    })

    const txHash = await this.walletClient.writeContract(request)

    // Wait for confirmation
    await this.publicClient.waitForTransactionReceipt({ hash: txHash })

    return {
      txHash,
      proposalId: result as string,
    }
  }

  /**
   * Get proposal by ID
   */
  async getProposal(proposalId: string): Promise<OnChainProposal | null> {
    try {
      const proposalIdBytes = proposalId.startsWith('0x')
        ? (proposalId as `0x${string}`)
        : toHex(proposalId, { size: 32 })

      const result = await this.publicClient.readContract({
        address: this.contractAddress,
        abi: BOARD_GOVERNANCE_ABI,
        functionName: 'getProposal',
        args: [proposalIdBytes],
      })

      // Check if proposal exists (proposalId != 0)
      if (result.proposalId === '0x' + '0'.repeat(64)) {
        return null
      }

      return {
        proposalId: result.proposalId,
        proposer: result.proposer,
        proposalType: result.proposalType,
        status: result.status,
        createdAt: Number(result.createdAt),
        contentHash: result.contentHash,
        targetContract: result.targetContract,
        value: result.value.toString(),
      }
    } catch {
      return null
    }
  }

  /**
   * Get all proposal IDs for a DAO
   */
  async getProposalsByDAO(daoId: string): Promise<string[]> {
    const daoIdBytes = daoId.startsWith('0x')
      ? (daoId as `0x${string}`)
      : toHex(daoId, { size: 32 })

    const result = await this.publicClient.readContract({
      address: this.contractAddress,
      abi: BOARD_GOVERNANCE_ABI,
      functionName: 'getProposalsByDAO',
      args: [daoIdBytes],
    })

    return result as string[]
  }

  /**
   * Get proposal count for a DAO
   */
  async getProposalCount(daoId: string): Promise<number> {
    const daoIdBytes = daoId.startsWith('0x')
      ? (daoId as `0x${string}`)
      : toHex(daoId, { size: 32 })

    const result = await this.publicClient.readContract({
      address: this.contractAddress,
      abi: BOARD_GOVERNANCE_ABI,
      functionName: 'proposalCount',
      args: [daoIdBytes],
    })

    return Number(result)
  }
}

// Singleton instance
let proposalServiceInstance: ProposalService | null = null

export function getProposalService(): ProposalService {
  if (!proposalServiceInstance) {
    proposalServiceInstance = new ProposalService()
  }
  return proposalServiceInstance
}
