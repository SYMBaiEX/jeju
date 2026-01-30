// SPDX-License-Identifier: MIT
pragma solidity ^0.8.33;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IBoardGovernance} from "./interfaces/IBoardGovernance.sol";

/**
 * @title BoardGovernance
 * @author Jeju Network
 * @notice Singleton contract for managing DAO proposals across all DAOs
 * @dev Implements IBoardGovernance for on-chain proposal storage and lifecycle
 *
 * Design decisions:
 * - Singleton per network (not per-DAO) for simpler deployment
 * - Quality score stored off-chain (API/SQLit) - on-chain is manipulable
 * - No staking in v1 - can be added later
 * - Autocrat operator controls status transitions
 */
contract BoardGovernance is IBoardGovernance, Ownable, ReentrancyGuard {
    // ============ State Variables ============

    /// @notice All proposals by ID
    mapping(bytes32 => Proposal) private _proposals;

    /// @notice Proposal IDs by DAO
    mapping(bytes32 => bytes32[]) private _daoProposals;

    /// @notice Check if proposal exists
    mapping(bytes32 => bool) private _proposalExists;

    /// @notice Autocrat operator address (can update statuses)
    address public autocratOperator;

    /// @notice Voting period duration
    uint256 public votingPeriod = 3 days;

    /// @notice Grace period duration
    uint256 public gracePeriod = 1 days;

    /// @notice Proposal counter for unique IDs
    uint256 private _proposalCounter;

    // ============ Errors ============

    error ProposalNotFound();
    error ProposalAlreadyExists();
    error NotAuthorized();
    error InvalidStatus();
    error InvalidProposalType();
    error GracePeriodNotComplete();

    // ============ Modifiers ============

    modifier onlyAutocrat() {
        if (msg.sender != autocratOperator && msg.sender != owner()) revert NotAuthorized();
        _;
    }

    modifier proposalExists(bytes32 proposalId) {
        if (!_proposalExists[proposalId]) revert ProposalNotFound();
        _;
    }

    // ============ Constructor ============

    constructor(address initialOwner, address _autocratOperator) Ownable(initialOwner) {
        autocratOperator = _autocratOperator;
    }

    // ============ Write Functions ============

    /**
     * @notice Submit a new proposal
     * @param daoId The DAO this proposal belongs to
     * @param proposalType Type of proposal (0=Treasury, 1=Parameter, etc.)
     * @param contentHash IPFS hash of proposal content
     * @param targetContract Contract to call if approved
     * @param callData Encoded function call
     * @param value ETH value to send
     * @return proposalId Unique proposal identifier
     */
    function submitProposal(
        bytes32 daoId,
        uint8 proposalType,
        bytes32 contentHash,
        address targetContract,
        bytes calldata callData,
        uint256 value
    ) external nonReentrant returns (bytes32 proposalId) {
        // Generate unique proposal ID
        proposalId = keccak256(abi.encodePacked(daoId, contentHash, block.timestamp, _proposalCounter++));

        if (_proposalExists[proposalId]) revert ProposalAlreadyExists();

        uint256 currentTime = block.timestamp;

        _proposals[proposalId] = Proposal({
            proposalId: proposalId,
            proposer: msg.sender,
            proposerAgentId: 0, // Set by Autocrat API if needed
            proposalType: proposalType,
            status: ProposalStatus.SUBMITTED,
            qualityScore: 0, // Stored off-chain
            createdAt: currentTime,
            autocratVoteEnd: currentTime + votingPeriod,
            gracePeriodEnd: currentTime + votingPeriod + gracePeriod,
            contentHash: contentHash,
            targetContract: targetContract,
            callData: callData,
            value: value,
            totalStaked: 0, // No staking in v1
            totalReputation: 0,
            backerCount: 0,
            hasResearch: false,
            researchHash: bytes32(0),
            directorApproved: false,
            directorDecisionHash: bytes32(0)
        });

        _proposalExists[proposalId] = true;
        _daoProposals[daoId].push(proposalId);

        emit ProposalSubmitted(daoId, proposalId, msg.sender, proposalType, contentHash);
    }

    /**
     * @notice Update proposal status (Autocrat only)
     */
    function updateProposalStatus(bytes32 proposalId, ProposalStatus status)
        external
        onlyAutocrat
        proposalExists(proposalId)
    {
        ProposalStatus oldStatus = _proposals[proposalId].status;
        _proposals[proposalId].status = status;
        emit ProposalStatusChanged(proposalId, oldStatus, status);
    }

    /**
     * @notice Record Director's decision (Autocrat only)
     */
    function setDirectorApproval(bytes32 proposalId, bool approved, bytes32 decisionHash)
        external
        onlyAutocrat
        proposalExists(proposalId)
    {
        _proposals[proposalId].directorApproved = approved;
        _proposals[proposalId].directorDecisionHash = decisionHash;

        ProposalStatus oldStatus = _proposals[proposalId].status;
        ProposalStatus newStatus = approved ? ProposalStatus.APPROVED : ProposalStatus.REJECTED;
        _proposals[proposalId].status = newStatus;

        emit DirectorDecision(proposalId, approved, decisionHash);
        emit ProposalStatusChanged(proposalId, oldStatus, newStatus);
    }

    /**
     * @notice Mark proposal as executing
     */
    function markExecuting(bytes32 proposalId) external onlyAutocrat proposalExists(proposalId) {
        Proposal storage p = _proposals[proposalId];
        if (p.status != ProposalStatus.APPROVED) revert InvalidStatus();
        if (block.timestamp < p.gracePeriodEnd) revert GracePeriodNotComplete();

        ProposalStatus oldStatus = p.status;
        p.status = ProposalStatus.EXECUTING;
        emit ProposalStatusChanged(proposalId, oldStatus, ProposalStatus.EXECUTING);
    }

    /**
     * @notice Mark proposal as completed
     */
    function markCompleted(bytes32 proposalId) external onlyAutocrat proposalExists(proposalId) {
        Proposal storage p = _proposals[proposalId];
        if (p.status != ProposalStatus.EXECUTING) revert InvalidStatus();

        ProposalStatus oldStatus = p.status;
        p.status = ProposalStatus.COMPLETED;
        emit ProposalStatusChanged(proposalId, oldStatus, ProposalStatus.COMPLETED);
    }

    /**
     * @notice Mark proposal as failed
     */
    function markFailed(bytes32 proposalId, string calldata /* reason */ )
        external
        onlyAutocrat
        proposalExists(proposalId)
    {
        Proposal storage p = _proposals[proposalId];
        ProposalStatus oldStatus = p.status;
        p.status = ProposalStatus.REJECTED;
        emit ProposalStatusChanged(proposalId, oldStatus, ProposalStatus.REJECTED);
    }

    // ============ Read Functions ============

    function isProposalApproved(bytes32 proposalId) external view returns (bool) {
        return _proposals[proposalId].status == ProposalStatus.APPROVED
            || _proposals[proposalId].status == ProposalStatus.EXECUTING
            || _proposals[proposalId].status == ProposalStatus.COMPLETED;
    }

    function isGracePeriodComplete(bytes32 proposalId) external view returns (bool) {
        return block.timestamp >= _proposals[proposalId].gracePeriodEnd;
    }

    function getProposal(bytes32 proposalId) external view returns (Proposal memory) {
        return _proposals[proposalId];
    }

    function getProposalsByDAO(bytes32 daoId) external view returns (bytes32[] memory) {
        return _daoProposals[daoId];
    }

    function proposalCount(bytes32 daoId) external view returns (uint256) {
        return _daoProposals[daoId].length;
    }

    // ============ Admin Functions ============

    function setAutocratOperator(address _operator) external onlyOwner {
        autocratOperator = _operator;
    }

    function setVotingPeriod(uint256 _period) external onlyOwner {
        votingPeriod = _period;
    }

    function setGracePeriod(uint256 _period) external onlyOwner {
        gracePeriod = _period;
    }

    function version() external pure returns (string memory) {
        return "1.0.0";
    }
}
