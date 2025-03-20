# REVO Bridge

This document explains how to set up and run the REVO bridge, which allows users to transfer their CXS and NEXTEP tokens from nxchain to the new REVO blockchain.

## Overview

The bridge works as follows:

1. A public address on nxchain is designated as the bridge address
2. Users send their CXS or NEXTEP tokens to this bridge address
3. The bridge script monitors for these deposits
4. When a deposit is detected, the script mints an equivalent amount of REVO tokens on the new chain
5. The REVO tokens are sent to the same address that sent the original tokens

## Prerequisites

- Python 3.6+
- Web3.py (`pip install web3`)
- Requests (`pip install requests`)
- Access to an nxchain RPC endpoint
- Access to a REVO chain RPC endpoint
- A private key with minting privileges on the REVO chain

## Setup

1. Create a public address on nxchain to serve as the bridge address
   - This can be any valid address that you don't need to control
   - Users will send their tokens to this address

2. Ensure you have a private key with minting privileges on the REVO chain
   - This key must be authorized to mint new REVO tokens
   - Keep this key secure and never share it

3. Set up the `fetch_cxs_price.py` script to fetch current token prices
   - The bridge uses this script to determine the USD value of deposits

## Running the Bridge

```bash
python revo_bridge.py \
  --bridge-address 0x123456789abcdef123456789abcdef123456789a \
  --private-key YOUR_PRIVATE_KEY \
  --nxchain-rpc https://rpc.nxchain.example.com \
  --revo-rpc https://rpc.revo.example.com \
  --revo-price 0.1 \
  --chain-id revo-1
```

### Required Parameters

- `--bridge-address`: The address on nxchain to monitor for deposits
- `--private-key`: Private key with minting privileges on the REVO chain
- `--nxchain-rpc`: RPC endpoint URL for nxchain
- `--revo-rpc`: RPC endpoint URL for the REVO chain
- `--revo-price`: Price of REVO token in USD
- `--chain-id`: Chain ID for the REVO chain

### Optional Parameters

- `--cxs-price`: Override CXS price in USD (default: fetch from API)
- `--nextep-price`: Override NEXTEP price in USD (default: fetch from API)
- `--binary`: Name of the blockchain binary (default: revod)
- `--state-file`: Path to the bridge state file (default: bridge_state.json)
- `--poll-interval`: Polling interval in seconds (default: 60)
- `--confirmations`: Number of confirmations required (default: 12)
- `--max-blocks`: Maximum number of blocks to process in one batch (default: 100)

## How It Works

### Token Conversion Process

1. When a user sends CXS or NEXTEP to the bridge address, the script detects this transaction
2. The script calculates the USD value of the deposit using current token prices
3. The USD value is converted to an equivalent amount of REVO tokens based on the REVO price
4. The script mints the calculated amount of REVO tokens on the new chain
5. The REVO tokens are sent to the same address that made the original deposit

### Formula

```
revo_amount = (token_amount * token_price_usd) / revo_price_usd
```

For example:
- If a user sends 100 CXS at a price of $0.50 per CXS
- And the REVO price is $0.10 per REVO
- They will receive 500 REVO tokens (100 * 0.50 / 0.10 = 500)

### State Management

The bridge maintains a state file (`bridge_state.json`) to track:
- The last processed block number
- Transactions that have already been processed

This ensures that:
- No transaction is processed twice
- The bridge can resume from where it left off if restarted

## Security Considerations

1. **Private Key Security**: The private key used for minting REVO tokens is highly sensitive. Store it securely and never expose it.

2. **RPC Endpoints**: Use secure, authenticated RPC endpoints whenever possible.

3. **Confirmations**: The default confirmation count is 12 blocks. Adjust this based on your security requirements.

4. **Rate Limiting**: The script includes rate limiting to avoid overloading RPC endpoints.

5. **Graceful Shutdown**: The script handles SIGINT and SIGTERM signals to ensure proper state saving on shutdown.

## Monitoring and Maintenance

- The script logs all activity to both the console and a log file (`revo_bridge.log`)
- Monitor this log file for any errors or issues
- Periodically check the state file to ensure the bridge is processing transactions correctly
- Consider setting up alerts for any critical errors

## Example User Instructions

Provide these instructions to users who want to migrate their tokens:

1. Send your CXS or NEXTEP tokens to the bridge address: `0x123456789abcdef123456789abcdef123456789a`
2. Wait for the transaction to be confirmed (at least 12 blocks)
3. The equivalent amount of REVO tokens will be automatically minted and sent to your address on the new REVO chain
4. Check your balance on the REVO chain using the explorer or wallet

## Troubleshooting

- **Missing Transactions**: If a transaction doesn't get processed, check if it has enough confirmations
- **Price Issues**: If token prices can't be fetched, use the override parameters to set prices manually
- **RPC Connection Issues**: Ensure the RPC endpoints are accessible and responding correctly 