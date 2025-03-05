# Boring Bureaucracy

A CLI tool for managing administrative transactions for boring products.

## Overview

This tool helps manage and simulate administrative transactions for boring products, particularly focusing on:

- Generating transactions for updating merkle roots
- Simulating administrative transactions through Tenderly
- Handling timelock-protected operations
- Managing multi-signature transactions

The tool integrates with Tenderly's simulation API to validate transactions before they are executed on-chain.

## Building

1. Clone the repository

   ```bash
   git clone https://github.com/Veda-Labs/boring-bureaucracy.git
   cd boring-bureaucracy
   ```

2. Copy the sample environment file

   ```bash
   cp sample.env .env
   ```

3. Fill in the required environment variables in `.env`:

- `TENDERLY_ACCESS_KEY`: Your Tenderly API access key
- `TENDERLY_ACCOUNT_SLUG`: Your Tenderly account slug
- `TENDERLY_PROJECT_SLUG`: Your Tenderly project slug
- `MAINNET_RPC_URL`: Ethereum mainnet RPC URL
- `SONIC_RPC_URL`: Sonic RPC URL
- `BOB_RPC_URL`: BOB RPC URL

4. Build the project

```bash
cargo build
```

## CLI Examples

### Generating Root Update Transactions

Generate transactions to update a product's merkle root:

```bash
cargo run --bin cli update-root \
--root 0x89a526fb2b69815032c7c59b737cef4f7275105b4e02cd4c6cc09317876cb406 \
--product liquid_eth \
--network 1 \
--nonce 34
```

This will generate transaction files in the `output` directory:

- For products without timelock: generates one transaction file
- For products with timelock: generates two transaction files (propose and execute)

### Simulating a Single Transaction

Simulate a single administrative transaction:

```bash
cargo run --bin cli simulate --tx output/tx_0.json
```

This will return the unique safe hash for this tx, and a tenderly simulation url.

### Simulating Timelock Transactions

Simulate a pair of timelock transactions (propose and execute):

```bash
cargo run --bin cli simulate-timelock \
--propose output/tx_0.json \
--execute output/tx_1.json
```

This will return the unique safe hash for the propose and the execute txs, and a tenderly vnet url.

## Configuration

Product configurations are managed through `config.toml`. Each product can have network-specific settings and defaults.
