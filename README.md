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

### Approve Safe Transaction Hash with Hardware Wallet

To approve a Safe transaction hash using a hardware wallet:

#### Approve with Trezor

```bash
cargo run --bin cli approve-hash --tx output/tx_0.json --trezor
```

#### Approve with Ledger

```bash
cargo run --bin cli approve-hash --tx output/tx_0.json --ledger
```

This command will:

1. Connect to your hardware wallet
2. Display the transaction details for verification
3. Request approval on the hardware wallet
4. Submit the approval transaction to the network
5. Return a block explorer URL to track the transaction

You must specify either `--trezor` (-t) or `--ledger` (-l) to indicate which hardware wallet to use.

Make sure your hardware wallet is:

- Connected and unlocked
- On the Ethereum app
- Using the correct derivation path (set in .env)

## Configuration

Product configurations are managed through `config.toml`. Each product can have network-specific settings and defaults.

> ### Nonce 38: Propose Timelock to add Aave

SafeTxHash: `0x973bc1b24f52e81df238f9a7c451e9f3987deda5fceda45e4fcd67aa3f585f14`

Proposal TXN: Link

<details>
<summary>Transaction Details</summary>

```json
{
  "baseGas": 0,
  "data": "0x8f2a0bb000000000000000000000000000000000000000000000000000000000000",
  "gasPrice": 0,
  "gasToken": "0x0000000000000000000000000000000000000000",
  "nonce": 38,
  "operation": 0,
  "refundReceiver": "0x0000000000000000000000000000000000000000",
  "safeTxGas": 0,
  "to": "0xFb6ec7CCBd77a42922a35D22A94fdF7fd54EE4BC",
  "value": 0
}
```

</details>

- Set Root: `0x89a526fb2b69815032c7c59b737cef4f7275105b4e02cd4c6cc09317876cb406` for `0xB26AEb430b5Bf6Be55763b42095E82DB9a1838B8` and `0xE89CeE9837e6Fce3b1Ebd8E1C779b76fd6E20136`
  Diff: here
