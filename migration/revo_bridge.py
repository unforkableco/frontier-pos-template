#!/usr/bin/env python3

import argparse
import json
import logging
import os
import signal
import sys
import time
from datetime import datetime, timedelta
import requests
from decimal import Decimal, getcontext
import subprocess
from web3 import Web3
import hashlib
import base64

# Set decimal precision
getcontext().prec = 28

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('revo_bridge.log')
    ]
)
logger = logging.getLogger(__name__)

# Constants
CXS_TOKEN_ADDRESS = "0x0000000000000000000000000000000000000000"  # Native token
NEXTEP_TOKEN_ADDRESS = "0x432e4997060f2385bdb32cdc8be815c6b22a8a61"
CXS_DECIMALS = 18
NEXTEP_DECIMALS = 18
REVO_DECIMALS = 18
BRIDGE_STATE_FILE = "bridge_state.json"
DEFAULT_POLL_INTERVAL = 60  # seconds
DEFAULT_CONFIRMATION_BLOCKS = 12

# Global variables for graceful shutdown
shutdown_requested = False

# Signal handler for graceful shutdown
def signal_handler(sig, frame):
    global shutdown_requested
    logger.info("Shutdown requested. Completing current operations and saving state...")
    shutdown_requested = True

# Register signal handlers
signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

class RateLimiter:
    """Simple rate limiter to avoid overloading APIs."""
    
    def __init__(self, requests_per_minute):
        self.requests_per_minute = requests_per_minute
        self.interval = 60 / requests_per_minute
        self.last_request_time = 0
    
    def wait_if_needed(self):
        """Wait if needed to respect the rate limit."""
        current_time = time.time()
        time_since_last_request = current_time - self.last_request_time
        
        if time_since_last_request < self.interval:
            sleep_time = self.interval - time_since_last_request
            time.sleep(sleep_time)
        
        self.last_request_time = time.time()

class BridgeState:
    """Class to manage the bridge state."""
    
    def __init__(self, state_file=BRIDGE_STATE_FILE):
        self.state_file = state_file
        self.last_block_processed = 0
        self.processed_txs = {}
        self.load_state()
    
    def load_state(self):
        """Load bridge state from file."""
        try:
            if os.path.exists(self.state_file):
                with open(self.state_file, 'r') as f:
                    state = json.load(f)
                    self.last_block_processed = state.get("last_block_processed", 0)
                    self.processed_txs = state.get("processed_txs", {})
                logger.info(f"Loaded bridge state: last block processed = {self.last_block_processed}")
            else:
                logger.info("No existing state file found. Starting from scratch.")
        except Exception as e:
            logger.error(f"Error loading bridge state: {str(e)}")
    
    def save_state(self):
        """Save bridge state to file."""
        try:
            state = {
                "last_block_processed": self.last_block_processed,
                "processed_txs": self.processed_txs
            }
            with open(self.state_file, 'w') as f:
                json.dump(state, f, indent=2)
            logger.info(f"Saved bridge state: last block processed = {self.last_block_processed}")
        except Exception as e:
            logger.error(f"Error saving bridge state: {str(e)}")
    
    def update_last_block(self, block_number):
        """Update the last processed block number."""
        self.last_block_processed = block_number
    
    def is_tx_processed(self, tx_hash):
        """Check if a transaction has already been processed."""
        return tx_hash in self.processed_txs
    
    def mark_tx_processed(self, tx_hash, details):
        """Mark a transaction as processed."""
        self.processed_txs[tx_hash] = {
            "timestamp": datetime.now().isoformat(),
            "details": details
        }

def connect_to_nxchain(rpc_url):
    """
    Connect to nxchain RPC endpoint.
    
    Args:
        rpc_url: RPC endpoint URL
        
    Returns:
        Web3 instance
    """
    try:
        w3 = Web3(Web3.HTTPProvider(rpc_url))
        # Check connection by trying to get the block number
        block_number = w3.eth.block_number
        logger.info(f"Connected to nxchain RPC endpoint: {rpc_url} (current block: {block_number})")
        return w3
    except Exception as e:
        logger.error(f"Error connecting to nxchain RPC endpoint: {str(e)}")
        return None

def get_token_price(token_symbol, price_override=None):
    """
    Get the current price of a token.
    
    Args:
        token_symbol: Symbol of the token (CXS or NEXTEP)
        price_override: Override price (if provided)
        
    Returns:
        Current price in USD as a Decimal
    """
    if price_override is not None:
        logger.info(f"Using override price for {token_symbol}: ${price_override}")
        return Decimal(str(price_override))
        
    try:
        logger.info(f"Fetching current price for {token_symbol}")
        
        # Run the fetch_cxs_price.py script with the token symbol
        result = subprocess.run(
            ["python", "fetch_cxs_price.py", "--token", token_symbol],
            capture_output=True,
            text=True,
            check=True
        )
        
        # Parse the output to get the price
        output = result.stdout.strip()
        price_line = [line for line in output.split('\n') if "USD price" in line]
        
        if not price_line:
            logger.error(f"Could not find price information in the output: {output}")
            return Decimal('0')
            
        # Extract the price from the line
        price_str = price_line[0].split(':')[1].strip()
        price = Decimal(price_str)
        
        logger.info(f"Current {token_symbol} price: ${price}")
        return price
    except subprocess.CalledProcessError as e:
        logger.error(f"Error running fetch_cxs_price.py: {e.stderr}")
        return Decimal('0')
    except Exception as e:
        logger.error(f"Error getting {token_symbol} price: {str(e)}")
        return Decimal('0')

def scan_for_deposits(w3, bridge_address, start_block, end_block, token_addresses):
    """
    Scan for token deposits to the bridge address.
    
    Args:
        w3: Web3 instance
        bridge_address: Bridge address to monitor
        start_block: Starting block number
        end_block: Ending block number
        token_addresses: List of token addresses to monitor
        
    Returns:
        List of deposit transactions
    """
    logger.info(f"Scanning for deposits from block {start_block} to {end_block}")
    
    deposits = []
    
    # Check for native CXS transfers
    for block_number in range(start_block, end_block + 1):
        if shutdown_requested:
            break
            
        try:
            block = w3.eth.get_block(block_number, full_transactions=True)
            
            for tx in block.transactions:
                # Check if this is a transaction to the bridge address
                if tx.to and tx.to.lower() == bridge_address.lower():
                    # Check if this is a native CXS transfer
                    if tx.value > 0:
                        deposits.append({
                            "tx_hash": tx.hash.hex(),
                            "from_address": tx["from"].lower(),
                            "token_address": CXS_TOKEN_ADDRESS,
                            "amount": tx.value,
                            "block_number": block_number,
                            "timestamp": block.timestamp
                        })
                        logger.info(f"Found CXS deposit: {tx.value / 10**CXS_DECIMALS} CXS from {tx['from']} (tx: {tx.hash.hex()})")
                    
                    # Check if this is a token transfer (ERC20 method call)
                    input_data = tx.input
                    if len(input_data) >= 10:
                        method_id = input_data[:10]
                        
                        # Check for ERC20 transfer method
                        if method_id == "0xa9059cbb":  # transfer(address,uint256)
                            try:
                                # Extract token contract address (tx.to)
                                token_address = tx.to.lower()
                                
                                # Check if this is a token we're monitoring
                                if token_address in token_addresses:
                                    # Extract recipient address and amount from the input data
                                    recipient = "0x" + input_data[34:74].lower()
                                    
                                    # Check if the recipient is the bridge address
                                    if recipient.lower() == bridge_address.lower():
                                        # Extract amount (last 32 bytes of the input data)
                                        amount_hex = input_data[74:138]
                                        amount = int(amount_hex, 16)
                                        
                                        deposits.append({
                                            "tx_hash": tx.hash.hex(),
                                            "from_address": tx["from"].lower(),
                                            "token_address": token_address,
                                            "amount": amount,
                                            "block_number": block_number,
                                            "timestamp": block.timestamp
                                        })
                                        
                                        token_name = "NEXTEP" if token_address.lower() == NEXTEP_TOKEN_ADDRESS.lower() else "Unknown"
                                        token_decimals = NEXTEP_DECIMALS if token_address.lower() == NEXTEP_TOKEN_ADDRESS.lower() else 18
                                        logger.info(f"Found {token_name} deposit: {amount / 10**token_decimals} {token_name} from {tx['from']} (tx: {tx.hash.hex()})")
                            except Exception as e:
                                logger.error(f"Error parsing transfer data: {str(e)}")
        except Exception as e:
            logger.error(f"Error processing block {block_number}: {str(e)}")
    
    return deposits

def calculate_revo_amount(token_address, token_amount, cxs_price, nextep_price, revo_price):
    """
    Calculate the amount of REVO tokens to mint based on the deposited token.
    
    Args:
        token_address: Address of the deposited token
        token_amount: Amount of the deposited token (in base units)
        cxs_price: CXS price in USD
        nextep_price: NEXTEP price in USD
        revo_price: REVO price in USD
        
    Returns:
        Amount of REVO tokens to mint (in base units)
    """
    if revo_price <= 0:
        logger.error("REVO price must be greater than zero")
        return 0
        
    token_address = token_address.lower()
    
    if token_address == CXS_TOKEN_ADDRESS.lower():
        # Convert CXS to USD
        token_decimals = CXS_DECIMALS
        token_price = cxs_price
    elif token_address == NEXTEP_TOKEN_ADDRESS.lower():
        # Convert NEXTEP to USD
        token_decimals = NEXTEP_DECIMALS
        token_price = nextep_price
    else:
        logger.error(f"Unsupported token address: {token_address}")
        return 0
    
    # Calculate token amount in its native units
    token_amount_decimal = Decimal(token_amount) / Decimal(10 ** token_decimals)
    
    # Calculate USD value
    usd_value = token_amount_decimal * token_price
    
    # Calculate REVO amount
    revo_amount = usd_value / Decimal(str(revo_price))
    
    # Convert to base units
    revo_amount_base_units = int(revo_amount * Decimal(10 ** REVO_DECIMALS))
    
    logger.info(f"Calculated REVO amount: {revo_amount} REVO (${usd_value})")
    
    return revo_amount_base_units

def mint_revo_tokens(recipient_address, amount, private_key, revo_rpc_url, chain_id, binary="revod"):
    """
    Mint REVO tokens on the new chain.
    
    Args:
        recipient_address: Address to receive the minted tokens
        amount: Amount of tokens to mint (in base units)
        private_key: Private key with minting privileges
        revo_rpc_url: RPC endpoint URL for the REVO chain
        chain_id: Chain ID for the REVO chain
        binary: Name of the blockchain binary
        
    Returns:
        Transaction hash if successful, None otherwise
    """
    try:
        logger.info(f"Minting {amount / 10**REVO_DECIMALS} REVO to {recipient_address}")
        
        # Create a temporary key file
        key_file = "temp_mint_key.json"
        with open(key_file, 'w') as f:
            json.dump({"private_key": private_key}, f)
        
        # Build the mint command
        mint_cmd = [
            binary,
            "tx", "bank", "send",
            "--from", private_key,  # This should be the address derived from the private key
            "--to", recipient_address,
            "--amount", f"{amount}arevo",
            "--chain-id", chain_id,
            "--node", revo_rpc_url,
            "--gas", "auto",
            "--gas-adjustment", "1.4",
            "--gas-prices", "0.025arevo",
            "--yes"
        ]
        
        # Execute the mint command
        result = subprocess.run(
            mint_cmd,
            capture_output=True,
            text=True,
            check=True
        )
        
        # Parse the output to get the transaction hash
        output = result.stdout.strip()
        tx_hash_line = [line for line in output.split('\n') if "txhash" in line]
        
        if not tx_hash_line:
            logger.error(f"Could not find transaction hash in the output: {output}")
            return None
            
        # Extract the transaction hash
        tx_hash = tx_hash_line[0].split(':')[1].strip()
        
        logger.info(f"Successfully minted {amount / 10**REVO_DECIMALS} REVO to {recipient_address} (tx: {tx_hash})")
        
        # Clean up the temporary key file
        os.remove(key_file)
        
        return tx_hash
    except subprocess.CalledProcessError as e:
        logger.error(f"Error minting REVO tokens: {e.stderr}")
        return None
    except Exception as e:
        logger.error(f"Error minting REVO tokens: {str(e)}")
        return None
    finally:
        # Make sure the key file is removed
        if os.path.exists(key_file):
            os.remove(key_file)

def process_deposits(deposits, bridge_state, cxs_price, nextep_price, revo_price, private_key, revo_rpc_url, chain_id, binary="revod"):
    """
    Process deposits and mint REVO tokens.
    
    Args:
        deposits: List of deposit transactions
        bridge_state: Bridge state object
        cxs_price: CXS price in USD
        nextep_price: NEXTEP price in USD
        revo_price: REVO price in USD
        private_key: Private key with minting privileges
        revo_rpc_url: RPC endpoint URL for the REVO chain
        chain_id: Chain ID for the REVO chain
        binary: Name of the blockchain binary
        
    Returns:
        Number of successfully processed deposits
    """
    processed_count = 0
    
    for deposit in deposits:
        if shutdown_requested:
            break
            
        tx_hash = deposit["tx_hash"]
        
        # Check if this transaction has already been processed
        if bridge_state.is_tx_processed(tx_hash):
            logger.info(f"Transaction {tx_hash} already processed. Skipping.")
            continue
        
        # Calculate REVO amount
        revo_amount = calculate_revo_amount(
            deposit["token_address"],
            deposit["amount"],
            cxs_price,
            nextep_price,
            revo_price
        )
        
        if revo_amount <= 0:
            logger.warning(f"Calculated REVO amount is zero or negative for transaction {tx_hash}. Skipping.")
            continue
        
        # Mint REVO tokens
        mint_tx_hash = mint_revo_tokens(
            deposit["from_address"],
            revo_amount,
            private_key,
            revo_rpc_url,
            chain_id,
            binary
        )
        
        if mint_tx_hash:
            # Mark transaction as processed
            bridge_state.mark_tx_processed(tx_hash, {
                "from_address": deposit["from_address"],
                "token_address": deposit["token_address"],
                "amount": str(deposit["amount"]),
                "revo_amount": str(revo_amount),
                "mint_tx_hash": mint_tx_hash,
                "timestamp": datetime.now().isoformat()
            })
            
            processed_count += 1
        else:
            logger.error(f"Failed to mint REVO tokens for transaction {tx_hash}. Will retry on next run.")
    
    return processed_count

def run_bridge(args):
    """
    Run the bridge process.
    
    Args:
        args: Command line arguments
    """
    # Initialize bridge state
    bridge_state = BridgeState(args.state_file)
    
    # Connect to nxchain
    w3 = connect_to_nxchain(args.nxchain_rpc)
    if not w3:
        logger.error("Failed to connect to nxchain. Exiting.")
        return 1
    
    # Get token prices
    cxs_price = get_token_price("CXS", args.cxs_price)
    nextep_price = get_token_price("NEXTEP", args.nextep_price)
    
    if cxs_price <= 0 or nextep_price <= 0:
        logger.error("Failed to get token prices. Exiting.")
        return 1
    
    if args.revo_price <= 0:
        logger.error("REVO price must be greater than zero. Exiting.")
        return 1
    
    # Token addresses to monitor
    token_addresses = [
        CXS_TOKEN_ADDRESS.lower(),
        NEXTEP_TOKEN_ADDRESS.lower()
    ]
    
    # Main loop
    while not shutdown_requested:
        try:
            # Get current block number
            current_block = w3.eth.block_number
            
            # Determine start and end blocks
            start_block = bridge_state.last_block_processed + 1
            end_block = min(current_block - args.confirmations, start_block + args.max_blocks - 1)
            
            # Check if there are new blocks to process
            if start_block > end_block:
                logger.info(f"No new blocks to process. Waiting for {args.poll_interval} seconds...")
                time.sleep(args.poll_interval)
                continue
            
            # Scan for deposits
            deposits = scan_for_deposits(
                w3,
                args.bridge_address,
                start_block,
                end_block,
                token_addresses
            )
            
            if deposits:
                # Process deposits
                processed_count = process_deposits(
                    deposits,
                    bridge_state,
                    cxs_price,
                    nextep_price,
                    args.revo_price,
                    args.private_key,
                    args.revo_rpc,
                    args.chain_id,
                    args.binary
                )
                
                logger.info(f"Processed {processed_count}/{len(deposits)} deposits")
            else:
                logger.info(f"No deposits found in blocks {start_block} to {end_block}")
            
            # Update last processed block
            bridge_state.update_last_block(end_block)
            
            # Save bridge state
            bridge_state.save_state()
            
            # Wait for the next poll interval
            logger.info(f"Waiting for {args.poll_interval} seconds before next poll...")
            time.sleep(args.poll_interval)
            
            # Refresh token prices periodically
            if not args.cxs_price:
                cxs_price = get_token_price("CXS")
            if not args.nextep_price:
                nextep_price = get_token_price("NEXTEP")
            
        except Exception as e:
            logger.error(f"Error in bridge process: {str(e)}")
            logger.info(f"Waiting for {args.poll_interval} seconds before retry...")
            time.sleep(args.poll_interval)
    
    # Save final state before exiting
    bridge_state.save_state()
    
    logger.info("Bridge process terminated gracefully")
    return 0

def main():
    parser = argparse.ArgumentParser(description="Bridge for transferring CXS and NEXTEP tokens to REVO")
    
    # Required arguments
    parser.add_argument("--bridge-address", required=True, help="Address on nxchain to monitor for deposits")
    parser.add_argument("--private-key", required=True, help="Private key with minting privileges on REVO chain")
    parser.add_argument("--nxchain-rpc", required=True, help="RPC endpoint URL for nxchain")
    parser.add_argument("--revo-rpc", required=True, help="RPC endpoint URL for REVO chain")
    parser.add_argument("--revo-price", type=float, required=True, help="Price of REVO token in USD")
    parser.add_argument("--chain-id", required=True, help="Chain ID for the REVO chain")
    
    # Optional arguments
    parser.add_argument("--cxs-price", type=float, help="Override CXS price in USD (default: fetch from API)")
    parser.add_argument("--nextep-price", type=float, help="Override NEXTEP price in USD (default: fetch from API)")
    parser.add_argument("--binary", default="revod", help="Name of the blockchain binary (default: revod)")
    parser.add_argument("--state-file", default=BRIDGE_STATE_FILE, help=f"Path to the bridge state file (default: {BRIDGE_STATE_FILE})")
    parser.add_argument("--poll-interval", type=int, default=DEFAULT_POLL_INTERVAL, help=f"Polling interval in seconds (default: {DEFAULT_POLL_INTERVAL})")
    parser.add_argument("--confirmations", type=int, default=DEFAULT_CONFIRMATION_BLOCKS, help=f"Number of confirmations required (default: {DEFAULT_CONFIRMATION_BLOCKS})")
    parser.add_argument("--max-blocks", type=int, default=100, help="Maximum number of blocks to process in one batch (default: 100)")
    
    args = parser.parse_args()
    
    # Validate REVO price
    if args.revo_price <= 0:
        logger.error("REVO price must be greater than zero")
        return 1
    
    # Run the bridge
    return run_bridge(args)

if __name__ == "__main__":
    sys.exit(main()) 