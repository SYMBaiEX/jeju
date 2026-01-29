# Claude Context Documentation

**Date Created:** January 29, 2025 05:42
**Last Updated:** January 29, 2025 05:42

## Development Environment

### Local Setup
- **Machine:** MacBook
- **Repository:** prophet10x/jeju (local clone)
- **Active Branch:** `fresh-deploy`
- **Local Path:** `/Users/admin69/jeju/jeju`

### Oracle Cloud Server
- **IP Address:** `192.9.153.231`
- **User:** `ubuntu`
- **Access:** SSH required for all operations
- **Location:** Contains actual deployment of Jeju Network

## Network Architecture

### Deployed Infrastructure
- **L1 Chain:** Base/parent chain
- **L2 Chain:** Jeju Network (running on Anvil)
  - **Port:** 6546 (not standard 8545 to avoid conflicts)
  - **Chain ID:** 31337
  - **RPC URL:** `http://localhost:6546` (internal) / `http://192.9.153.231:6546` (external)
- **DApps:** Fully deployed ecosystem including:
  - Node staking system
  - Tauri desktop application
  - Contract registry
  - Token systems (JEJU, USDC)

### Critical Infrastructure Notes

#### ⚠️ ANVIL STATE MANAGEMENT
**CRITICAL:** Must save anvil state before any restart operations
- **State Location:** `~/jeju-anvil-state/anvil-state.json`
- **Auto-save Script:** `~/jeju/scripts/anvil-with-autosave.sh` (saves every 10 minutes, keeps 15 backups)
- **Manual Save Command:** `~/.foundry/bin/cast rpc anvil_dumpState --rpc-url http://localhost:6546 > ~/jeju-anvil-state/anvil-state.json`
- **Restore Command:** Start anvil with `--load-state ~/jeju-anvil-state/anvil-state.json`

#### 🔄 COMPLETE SYSTEM RESTART PROCESS
When starting from scratch (lost state, fresh deployment):

1. **Start fresh anvil:** `nohup ~/.foundry/bin/anvil --host 0.0.0.0 --port 6546 --chain-id 31337 > /tmp/anvil.log 2>&1 &`
2. **Run bootstrap:** `cd ~/jeju && ~/.bun/bin/bun run packages/deployment/scripts/bootstrap-localnet-complete.ts`
3. **Deploy ServiceStaking:** `cd ~/jeju/packages/contracts && ~/.foundry/bin/forge create --broadcast --private-key 0xac0974... --rpc-url http://localhost:6546 src/staking/ServiceStaking.sol:ServiceStaking --constructor-args [JEJU_TOKEN] [OWNER]`
4. **Update config:** Edit `apps/node/app/src-tauri/src/config.rs` with new contract addresses
5. **Build Tauri:** `cd ~/jeju/apps/node/app/src-tauri && ~/.cargo/bin/cargo build --release`
6. **Start app:** `DISPLAY=:11.0 nohup ~/jeju/apps/node/app/src-tauri/target/release/jeju-node > /tmp/jeju-node.log 2>&1 &`
7. **Save state:** Save working state immediately after successful deployment

## Current Work Areas

### Tauri Desktop Application
- **Location:** `apps/node/app/`
- **Backend:** Rust (src-tauri/)
- **Frontend:** Web components (embedded in Tauri)
- **Build Process:** Must rebuild Rust after web changes to embed new assets

### Contract Development
- **Location:** `packages/contracts/`
- **Deployment:** Uses Foundry/Forge
- **Bootstrap Script:** `packages/deployment/scripts/bootstrap-localnet-complete.ts`
- **Key Contracts:**
  - JEJU Token
  - MultiServiceStakeManager
  - ServiceStaking (newer per-service variant)

### Current Focus: Staking System
- **Issue:** UI tracking of individual service stakes for unstaking
- **Contracts:**
  - `MultiServiceStakeManager.sol` (deployed at `0xE6E340D132b5f46d1e472DebcD681B2aBc16e57E`, tracks totals only)
  - `ServiceStaking.sol` (deployed at `0x976fcd02f7C4773dd89C309fBF55D5923B4c98a1`, tracks per-service stakes)
- **Files Modified:**
  - `apps/node/app/src-tauri/src/commands/staking.rs`
  - `apps/node/web/components/Staking.tsx`
  - `apps/node/app/src-tauri/src/config.rs` (updated contract addresses)

### Current Deployed Contracts (Jan 29 2025)
- **JEJU Token:** `0x0B306BF915C4d645ff596e518fAf3F9669b97016`
- **ServiceStaking:** `0x976fcd02f7C4773dd89C309fBF55D5923B4c98a1`
- **MultiServiceStakeManager:** `0xE6E340D132b5f46d1e472DebcD681B2aBc16e57E`
- **NodeStakingManager:** `0xc5a5C42992dECbae36851359345FE25997F5C42d`
- **IdentityRegistry:** `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9`
- **BanManager:** `0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0`

## Important Paths & Commands

### Server Access
```bash
ssh ubuntu@192.9.153.231
```

### Common Operations
```bash
# Check anvil status
curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:6546

# Bootstrap full deployment
cd ~/jeju && ~/.bun/bin/bun run packages/deployment/scripts/bootstrap-localnet-complete.ts

# Build Tauri app
cd ~/jeju/apps/node/app/src-tauri && ~/.cargo/bin/cargo build --release

# Check app status
ps aux | grep jeju-node
```

### Configuration Files
- **Contract Addresses:** `packages/config/contracts.json`
- **Network Config:** `apps/node/app/src-tauri/src/config.rs`
- **Environment:** Various `.env.localnet` files

## Tools & Dependencies

### Server Environment
- **Bun:** `~/.bun/bin/bun`
- **Foundry:** `~/.foundry/bin/forge`, `~/.foundry/bin/cast`
- **Rust/Cargo:** `~/.cargo/bin/cargo`
- **Node/NPM:** For some build processes

### Development Workflow
1. Make changes locally on MacBook
2. Push to GitHub (fresh-deploy branch)
3. Pull changes on Oracle Cloud server
4. Build/deploy on server
5. Test via Tauri app or direct contract calls

## Security Notes
- Always save anvil state before infrastructure changes
- Test deployments thoroughly before production
- Keep track of contract addresses after deployments
- Monitor disk space for state backups (auto-cleanup after 20 backups)

## Troubleshooting

### Common Issues
- **Anvil Connection Refused:** Check if anvil is running on port 6546
- **Contract Not Found:** May need to redeploy after anvil restart
- **Tauri UI Not Updating:** Need to rebuild Rust after web changes
- **Transaction Failures:** Check if contracts are deployed and wallet has funds

### Recovery Procedures
- **Lost Anvil State:** Restore from `~/jeju-anvil-state/backups/`
- **Contract Issues:** Run bootstrap script to redeploy all contracts
- **Build Failures:** Check tool paths and dependencies on server

---

*This document should be updated as the project evolves and new context is discovered.*