# tx-spammer

A CLI tool for spamming ERC20 transactions against an EVM-compatible network.

## Build

```bash
cargo build --release
```

The binary will be at `target/release/tx-spammer`.

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `RPC_URL` | No | `http://localhost:8547` | HTTP RPC endpoint of the target node |
| `PRIVATE_KEY` | For `deploy` and `spamm` | — | Hex-encoded private key used to sign transactions |

## Commands

### `deploy`

Deploys the `MyToken` ERC20 contract to the network.

```bash
tx-spammer deploy
```

Reads the compiled artifact from `out/MyToken.sol/MyToken.json` (produced by Forge). Prints the deployed contract address on success.

**Requires:** `PRIVATE_KEY`

---

### `spamm <token_address> <tps>`

Sends ERC20 `transfer` transactions to random addresses at the specified rate.

```bash
tx-spammer spamm <token_address> <tps>
```

| Argument | Description |
|---|---|
| `token_address` | Address of the deployed ERC20 token contract |
| `tps` | Target transactions per second (must be > 0) |

Each transaction transfers 1 token unit to a randomly generated address. Gas fees are dynamically calculated from the latest block's base fee.

**Requires:** `PRIVATE_KEY`

---

### `balance <token_address> <owner_address>`

Queries the ERC20 token balance of an address (read-only, no key required).

```bash
tx-spammer balance <token_address> <owner_address>
```

| Argument | Description |
|---|---|
| `token_address` | Address of the ERC20 token contract |
| `owner_address` | Address to query the balance of |

---

### `block <block_number>`

Prints details about a specific block (read-only, no key required).

```bash
tx-spammer block <block_number>
```

| Argument | Description |
|---|---|
| `block_number` | The block number to inspect |

Outputs: transaction count, difficulty, gas used, gas limit, blob gas used, excess blob gas, and base fee per gas.

## Example

```bash
# Deploy the token contract
export RPC_URL=http://localhost:8545
export PRIVATE_KEY=0xabc123...

tx-spammer deploy
# Contract deployed at: 0xDeAdBeEf...

# Spam at 50 TPS
tx-spammer spamm 0xDeAdBeEf... 50

# Check balance
tx-spammer balance 0xDeAdBeEf... 0xYourAddress...

# Inspect block 42
tx-spammer block 42
```
