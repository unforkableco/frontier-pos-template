# REVO Blockchain Migration Tools

This directory contains tools for migrating from CXS and NEXTEP tokens to a new blockchain with REVO as the native token.

## Overview

The migration process involves several steps:

1. Fetch active wallet addresses and balances for CXS and NEXTEP tokens
2. Combine these balances into a single USD value per wallet
3. Convert USD values to REVO token allocations
4. Generate a genesis configuration file for the new blockchain
5. Set up and initialize the REVO blockchain

## Prerequisites

- Python 3.6+
- Web3.py (`pip install web3`)
- Requests (`pip install requests`)
- tqdm (`pip install tqdm`)
- Cosmos SDK-based blockchain binary (e.g., `revod`)

## Step 1: Fetch Active Wallet Addresses and Balances

### For CXS Token

```bash
python fetch_active_wallets.py \
  --firebase-user-id YOUR_FIREBASE_ID \
  --workspace YOUR_WORKSPACE \
  --fetch-balances \
  --rpc-url YOUR_RPC_URL \
  --output cxs_wallets.json
```

### For NEXTEP Token

```bash
python fetch_active_wallets_nextep.py \
  --firebase-user-id YOUR_FIREBASE_ID \
  --workspace YOUR_WORKSPACE \
  --fetch-balances \
  --rpc-url YOUR_RPC_URL \
  --output nextep_wallets.json
```

## Step 2: Combine Wallet Values

```bash
python combine_wallet_values.py \
  --cxs-file cxs_wallets.json \
  --nextep-file nextep_wallets.json \
  --output combined_wallets.json
```

This script will:
- Load wallet data from both CXS and NEXTEP files
- Fetch current prices for both tokens
- Calculate the total USD value for each wallet
- Save the combined data to a new JSON file

## Step 3: Calculate REVO Distribution

```bash
python calculate_revo_distribution.py \
  --input combined_wallets.json \
  --output revo_distribution.json \
  --revo-price 0.1
```

This script will:
- Convert USD values to REVO token allocations based on the provided REVO price
- Optionally filter out wallets with less than a minimum amount of REVO
- Save the REVO distribution to a new JSON file

## Step 4: Generate Genesis Configuration

```bash
python generate_genesis_config.py \
  --input revo_distribution.json \
  --output genesis.json \
  --chain-id revo-1 \
  --chain-name "REVO Mainnet"
```

This script will:
- Create a genesis configuration file for a new blockchain
- Include all wallet addresses with their pre-allocated REVO balances
- Configure blockchain parameters (governance, staking, etc.)

## Step 5: Set Up and Initialize the REVO Blockchain

```bash
bash setup_revo_chain.sh \
  --chain-id revo-1 \
  --genesis-file genesis.json \
  --binary revod
```

This script will:
- Initialize a new blockchain with the specified chain ID
- Create a validator key
- Copy the genesis file to the appropriate location
- Add the validator account to genesis
- Create and collect genesis transactions
- Validate the genesis file

After running this script, you can start the blockchain node with:

```bash
revod start --home $HOME/.revo
```

## Additional Options

Each script has additional options that can be viewed by running the script with the `--help` flag.

## Example Workflow

Here's a complete example workflow:

```bash
# Step 1: Fetch wallet data
python fetch_active_wallets.py --firebase-user-id user123 --workspace main --fetch-balances --rpc-url https://rpc.example.com --output cxs_wallets.json
python fetch_active_wallets_nextep.py --firebase-user-id user123 --workspace main --fetch-balances --rpc-url https://rpc.example.com --output nextep_wallets.json

# Step 2: Combine wallet values
python combine_wallet_values.py --cxs-file cxs_wallets.json --nextep-file nextep_wallets.json --output combined_wallets.json

# Step 3: Calculate REVO distribution
python calculate_revo_distribution.py --input combined_wallets.json --output revo_distribution.json --revo-price 0.1 --min-usd 1.0

# Step 4: Generate genesis configuration
python generate_genesis_config.py --input revo_distribution.json --output genesis.json --chain-id revo-1

# Step 5: Set up and initialize the blockchain
bash setup_revo_chain.sh --chain-id revo-1 --genesis-file genesis.json --binary revod
```

## Notes

- Make sure to back up all JSON files generated during the migration process
- The blockchain binary (`revod`) must be installed and available in your PATH
- You may need to adjust parameters like token prices, minimum thresholds, etc. based on your specific requirements
- For production deployments, consider using a more secure keyring backend than the default "test" backend 