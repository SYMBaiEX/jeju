# Jeju L2 Oracle Cloud Deployment

Deployment scripts and configuration for running Jeju L2 (OP Stack) on Oracle Cloud.

## Quick Start

### Step 1: Start L1 First

```bash
cd docker
docker compose up -d l1-geth
docker compose logs -f l1-geth  # Wait for "HTTP server started"
```

### Step 2: Initialize Genesis Configuration

```bash
chmod +x ../scripts/init-genesis.sh
../scripts/init-genesis.sh http://localhost:8545
```

This updates `rollup.json` with the actual L1 genesis hash.

### Step 3: Start L2 Stack

```bash
docker compose up -d op-geth op-node op-batcher
docker compose logs -f op-node
```

### Step 4: Verify L2 is Producing Blocks

```bash
curl -s localhost:9545 -X POST -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

You should see the block number increasing every 2 seconds.

## Components

| Service | Version | Port | Description |
|---------|---------|------|-------------|
| L1 Geth | v1.14.12 | 8545 | Ethereum L1 (--dev mode) |
| op-geth | latest | 9545 | L2 Execution Layer |
| op-node | v1.11.0 | 7545 | L2 Consensus Layer |
| op-batcher | latest | 6545 | Batch Submitter |
| Alto Bundler | v0.14.0 | 4337 | ERC-4337 Bundler (x86_64 only) |

## Chain Configuration

- **L1 Chain ID**: 1337 (Geth --dev default)
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

## ARM64 Compatibility (Oracle Cloud Ampere)

Oracle Cloud's free tier uses ARM64 (Ampere) processors. Key notes:

- **op-geth**: Use `:latest` tag (has ARM64 support)
- **op-node**: Use `v1.11.0` or later (has ARM64 support)
- **op-batcher**: Use `:latest` tag (has ARM64 support)
- **Alto Bundler**: **NO ARM64 SUPPORT** - excluded from ARM64 deployments

To run bundler on ARM64, you'll need to either:
1. Use a separate x86_64 machine
2. Build Alto from source for ARM64
3. Use an alternative bundler with ARM64 support

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
docker logs jeju-l1-geth -f
```

### Restart services
```bash
docker compose restart op-node
```

### Clean restart (reset all data)
```bash
docker compose down -v
docker compose up -d
```

### Common Issues

#### "L1 genesis block hash does not match"
The `rollup.json` L1 hash doesn't match the running L1. Re-run `init-genesis.sh`.

#### "state scheme path vs hash conflict"
The `--state.scheme=hash` flag is already added to the geth init command. If you see this error, delete the L2 data volume and restart:
```bash
docker compose down
docker volume rm jeju_l2-data
docker compose up -d
```

#### "Beacon API required for Ecotone"
This deployment uses pre-Ecotone forks (up to Canyon) to avoid requiring a Beacon node. Ecotone, Fjord, and Granite forks are disabled.

#### op-node "backing off" or not producing blocks
Ensure these flags are set in op-node command:
- `--l1.trustrpc` - Trust the L1 RPC (required for Geth --dev mode)
- `--sequencer.l1-confs=0` - Don't wait for L1 confirmations
- `--verifier.l1-confs=0` - Don't wait for L1 confirmations

## Version Compatibility Notes

This deployment uses specific versions tested to work together:

| Component | Version | Notes |
|-----------|---------|-------|
| op-node | v1.11.0 | Has ARM64 support, works with Geth L1 |
| op-geth | latest | ARM64 support, hash state scheme |
| op-batcher | latest | ARM64 support |
| Geth L1 | v1.14.12 | --dev mode with 2s block time |

### Why not Anvil?
Anvil has issues with block hash stability that cause op-node to fail verification. Geth in `--dev` mode provides stable, deterministic block hashes.

### Why no Ecotone/Fjord/Granite?
These forks require a Beacon API (L1 Beacon node). For simplicity, this deployment runs with forks up to Canyon only.
