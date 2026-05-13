# tx-spammer

A CLI tool for stress testing EVM-compatible networks via high-throughput transaction spam — ERC20 transfers and ZK gas stress operations.

---

## Table of Contents

- [Build](#build)
- [Configuration](#configuration)
- [Commands](#commands)
  - [deploy](#deploy)
  - [spam](#spam-token_address-tps)
  - [deploy-zkgas-stress](#deploy-zkgas-stress)
  - [spam-add](#spam-add-stress_address-iters-tps)
  - [spam-mulmod](#spam-mulmod-stress_address-iters-tps)
  - [spam-keccak](#spam-keccak-stress_address-iters-tps)
  - [spam-ecrecover](#spam-ecrecover-stress_address-iters-tps)
  - [spam-blake2f](#spam-blake2f-stress_address-iters-tps)
  - [spam-modexp](#spam-modexp-stress_address-iters-tps)
  - [balance](#balance-token_address-owner_address)
  - [block](#block-block_number)
- [Example](#example)

---

## Build

```bash
cargo build --release
```

Binary: `target/release/tx-spammer`

---

## Configuration

| Variable | Required | Default | Description |
|---|---|---|---|
| `RPC_URL` | No | `http://localhost:8547` | HTTP RPC endpoint |
| `PRIVATE_KEY` | For write commands | — | Hex-encoded private key for signing |

Read-only commands (`balance`, `block`) do not require `PRIVATE_KEY`.

---

## Commands

---

### `deploy`

Deploys the `MyToken` ERC20 contract. Reads the compiled artifact from `out/MyToken.sol/MyToken.json` (Forge output) and prints the deployed address.

```bash
tx-spammer deploy
```

---

### `spam <token_address> <tps>`

Sends ERC20 `transfer` calls to random addresses at the target rate. Each transaction transfers 1 token unit to a randomly generated address. Gas fees are derived from the latest block's base fee; transactions are batched for throughput.

```bash
tx-spammer spam 0xTokenAddress 50
```

---

### `deploy-zkgas-stress`

Deploys the `ZkGasStress` contract used by the stress spam commands below. Reads the compiled artifact from `out/ZkGasStress.sol/ZkGasStress.json` and prints the deployed address.

```bash
tx-spammer deploy-zkgas-stress
```

---

### `spam-add <stress_address> <iters> <tps>`

Stress tests the ADD opcode by spamming transactions that call `stressAdd(iters)` on the deployed `ZkGasStress` contract.

```bash
tx-spammer spam-add 0xStressAddress 10000 25
```

---

### `spam-mulmod <stress_address> <iters> <tps>`

Stress tests the MULMOD opcode by spamming transactions that call `stressMulmod(iters)`.

```bash
tx-spammer spam-mulmod 0xStressAddress 10000 25
```

---

### `spam-keccak <stress_address> <iters> <tps>`

Stress tests the KECCAK256 opcode by spamming transactions that call `stressKeccak(iters)`.

```bash
tx-spammer spam-keccak 0xStressAddress 10000 25
```

---

### `spam-ecrecover <stress_address> <iters> <tps>`

Stress tests the ECRECOVER precompile by spamming transactions that call `stressEcrecover(iters)`.

```bash
tx-spammer spam-ecrecover 0xStressAddress 1000 25
```

---

### `spam-blake2f <stress_address> <iters> <tps>`

Stress tests the BLAKE2F precompile by spamming transactions that call `stressBlake2f(iters)`.

```bash
tx-spammer spam-blake2f 0xStressAddress 1000 25
```

---

### `spam-modexp <stress_address> <iters> <tps>`

Stress tests the MODEXP precompile by spamming transactions that call `stressModexp(iters)`.

```bash
tx-spammer spam-modexp 0xStressAddress 1000 25
```

> For all `spam-*` stress commands, `iters` controls how many loop iterations each transaction executes, and `tps` sets the target transactions per second (must be > 0).

---

### `balance <token_address> <owner_address>`

Queries the ERC20 token balance of an address. Read-only, no `PRIVATE_KEY` required.

```bash
tx-spammer balance 0xTokenAddress 0xOwnerAddress
```

---

### `block <block_number>`

Prints details about a specific block: transaction count, difficulty, gas used, gas limit, blob gas used, excess blob gas, and base fee per gas. Read-only, no `PRIVATE_KEY` required.

```bash
tx-spammer block 42
```

---

## Example

```bash
export RPC_URL=http://localhost:8545
export PRIVATE_KEY=0xabc123...

# Deploy and spam ERC20 transfers at 50 TPS
tx-spammer deploy
tx-spammer spam 0xDeAdBeEf... 50

# Deploy and run a KECCAK256 stress test at 25 TPS, 10000 iters per tx
tx-spammer deploy-zkgas-stress
tx-spammer spam-keccak 0xC0ffee... 10000 25

# Check results
tx-spammer balance 0xDeAdBeEf... 0xYourAddress
tx-spammer block 100
```