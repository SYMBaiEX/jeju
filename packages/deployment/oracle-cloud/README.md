# Jeju L2 Oracle Cloud Deployment

Deployment scripts and configuration for running Jeju L2 (OP Stack) on Oracle Cloud.

## Quick Start

### Option 1: Docker Compose (Recommended)

```bash
cd docker
docker compose up -d
docker compose logs -f op-node
```

### Option 2: Shell Script

```bash
chmod +x scripts/deploy-l2.sh
./scripts/deploy-l2.sh
```

## Components

| Service | Version | Port | Description |
|---------|---------|------|-------------|
| L1 Geth | v1.14.12 | 8545 | Ethereum L1 (--dev mode) |
| op-geth | v1.101408.0 | 9545 | L2 Execution Layer |
| op-node | v1.10.1 | 7545 | L2 Consensus Layer |
| op-batcher | v1.10.1 | 6545 | Batch Submitter |
| Alto Bundler | v0.14.0 | 4337 | ERC-4337 Bundler |

## Chain Configuration

- **L1 Chain ID**: 3151908
- **L2 Chain ID**: 2151908
- **Block Time**: 2 seconds

## Test Accounts (Anvil defaults)

| Account | Address | Private Key |
|---------|---------|-------------|
| Deployer | 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 | 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 |
| Signer | 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 | 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d |
| Batcher | 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC | 0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a |
| Proposer | 0x90F79bf6EB2c4f870365E785982E1f101E93b906 | 0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6 |

## Deploy Paymaster (After L2 is Running)

```bash
chmod +x scripts/deploy-paymaster.sh
./scripts/deploy-paymaster.sh
```

This deploys:
- **ELIZAOS**: ERC-20 token
- **SimplePaymaster**: Sponsors gas for ELIZAOS holders

## Troubleshooting

### Check L2 block production
```bash
curl -s localhost:9545 -X POST -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### View logs
```bash
docker logs jeju-op-node -f
docker logs jeju-op-geth -f
```

### Restart services
```bash
docker compose restart op-node
```

## Version Compatibility Notes

This deployment uses specific versions that are compatible with each other:

- **op-node v1.10.1** works with Geth L1 (avoids Anvil block hash issues)
- **op-geth v1.101408.0** is pre-Holocene (no minBaseFee requirement)
- All forks up to Granite are enabled at genesis (time 0)
