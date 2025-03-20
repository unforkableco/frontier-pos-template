#!/bin/bash

# setup_revo_chain.sh - Script to set up and initialize a new blockchain with REVO as the native token

set -e

# Default values
CHAIN_ID="revo-1"
CHAIN_DIR="$HOME/.revo"
MONIKER="revo-node"
GENESIS_FILE="genesis.json"
BINARY="revod"
KEYRING_BACKEND="test"
KEY_NAME="validator"
VALIDATOR_TOKENS="10000000000000000000000" # 10M REVO in base units (arevo)

# Print usage information
function print_usage {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --chain-id STRING        Chain ID for the new blockchain (default: $CHAIN_ID)"
    echo "  --chain-dir PATH         Directory for blockchain data (default: $CHAIN_DIR)"
    echo "  --moniker STRING         Moniker for the validator node (default: $MONIKER)"
    echo "  --genesis-file PATH      Path to the genesis file (default: $GENESIS_FILE)"
    echo "  --binary STRING          Name of the blockchain binary (default: $BINARY)"
    echo "  --keyring-backend STRING Keyring backend to use (default: $KEYRING_BACKEND)"
    echo "  --key-name STRING        Name for the validator key (default: $KEY_NAME)"
    echo "  --validator-tokens INT   Amount of tokens for the validator (default: $VALIDATOR_TOKENS)"
    echo "  --help                   Show this help message"
    echo ""
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --chain-id)
            CHAIN_ID="$2"
            shift 2
            ;;
        --chain-dir)
            CHAIN_DIR="$2"
            shift 2
            ;;
        --moniker)
            MONIKER="$2"
            shift 2
            ;;
        --genesis-file)
            GENESIS_FILE="$2"
            shift 2
            ;;
        --binary)
            BINARY="$2"
            shift 2
            ;;
        --keyring-backend)
            KEYRING_BACKEND="$2"
            shift 2
            ;;
        --key-name)
            KEY_NAME="$2"
            shift 2
            ;;
        --validator-tokens)
            VALIDATOR_TOKENS="$2"
            shift 2
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Check if the binary exists
if ! command -v $BINARY &> /dev/null; then
    echo "Error: $BINARY binary not found. Please install it first."
    exit 1
fi

# Check if the genesis file exists
if [ ! -f "$GENESIS_FILE" ]; then
    echo "Error: Genesis file not found at $GENESIS_FILE"
    exit 1
fi

echo "Setting up REVO blockchain with the following configuration:"
echo "Chain ID: $CHAIN_ID"
echo "Chain Directory: $CHAIN_DIR"
echo "Moniker: $MONIKER"
echo "Genesis File: $GENESIS_FILE"
echo "Binary: $BINARY"
echo "Keyring Backend: $KEYRING_BACKEND"
echo "Key Name: $KEY_NAME"
echo "Validator Tokens: $VALIDATOR_TOKENS"
echo ""

# Confirm with the user
read -p "Do you want to proceed? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Setup aborted."
    exit 0
fi

# Initialize the chain
echo "Initializing the chain..."
$BINARY init $MONIKER --chain-id $CHAIN_ID --home $CHAIN_DIR

# Create a key for the validator
echo "Creating validator key..."
$BINARY keys add $KEY_NAME --keyring-backend $KEYRING_BACKEND --home $CHAIN_DIR

# Get the validator address
VALIDATOR_ADDRESS=$($BINARY keys show $KEY_NAME -a --keyring-backend $KEYRING_BACKEND --home $CHAIN_DIR)
echo "Validator address: $VALIDATOR_ADDRESS"

# Copy the genesis file
echo "Copying genesis file..."
cp $GENESIS_FILE $CHAIN_DIR/config/genesis.json

# Add the validator account to genesis
echo "Adding validator account to genesis..."
$BINARY add-genesis-account $VALIDATOR_ADDRESS ${VALIDATOR_TOKENS}arevo --home $CHAIN_DIR

# Create the validator transaction
echo "Creating validator transaction..."
$BINARY gentx $KEY_NAME ${VALIDATOR_TOKENS}arevo \
    --chain-id $CHAIN_ID \
    --moniker $MONIKER \
    --commission-rate 0.1 \
    --commission-max-rate 0.2 \
    --commission-max-change-rate 0.01 \
    --min-self-delegation 1 \
    --keyring-backend $KEYRING_BACKEND \
    --home $CHAIN_DIR

# Collect genesis transactions
echo "Collecting genesis transactions..."
$BINARY collect-gentxs --home $CHAIN_DIR

# Validate genesis
echo "Validating genesis..."
$BINARY validate-genesis --home $CHAIN_DIR

echo ""
echo "REVO blockchain setup completed successfully!"
echo ""
echo "To start the node, run:"
echo "$BINARY start --home $CHAIN_DIR"
echo ""
echo "To add more validators, run the following on each validator node:"
echo "$BINARY init <moniker> --chain-id $CHAIN_ID"
echo "cp $CHAIN_DIR/config/genesis.json ~/.revo/config/"
echo "$BINARY keys add <key-name> --keyring-backend $KEYRING_BACKEND"
echo "$BINARY gentx <key-name> <amount>arevo --chain-id $CHAIN_ID --moniker <moniker> ..."
echo ""
echo "Then collect and distribute the updated genesis file." 