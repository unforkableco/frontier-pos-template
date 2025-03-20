#!/usr/bin/env python3

import argparse
import json
import logging
import os
import signal
import sys
import time
from datetime import datetime
import requests
from tqdm import tqdm
import binascii
from decimal import Decimal
from web3 import Web3

# Constants
TRANSACTIONS_API_URL = "https://api.tryethernal.com/api/transactions"
MULTISEND_CONTRACT_ADDRESS = "0xF42fd95790C20F930E56847b8e29a9b5273052E7"
CXS_DECIMALS = 18

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('active_wallets_fetch.log')
    ]
)
logger = logging.getLogger(__name__)

# Global variables for graceful shutdown
shutdown_requested = False
current_page = 1

# Signal handler for graceful shutdown
def signal_handler(sig, frame):
    global shutdown_requested
    logger.info("Shutdown requested. Completing current batch and saving progress...")
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

def fetch_transactions(firebase_user_id, workspace, items_per_page=1000, page=1, order="desc", rpm=30):
    """
    Fetch transactions from the API.
    
    Args:
        firebase_user_id: The Firebase user ID for authentication
        workspace: The workspace name
        items_per_page: Number of items per page
        page: Page number
        order: Order of transactions ("asc" or "desc")
        rpm: Requests per minute
        
    Returns:
        List of transaction objects
    """
    rate_limiter = RateLimiter(rpm)
    rate_limiter.wait_if_needed()
    
    params = {
        "firebaseUserId": firebase_user_id,
        "workspace": workspace,
        "page": page,
        "itemsPerPage": items_per_page,
        "orderBy": "blockNumber",
        "order": order,
        "withCount": "false"
    }
    
    try:
        logger.info(f"Fetching transactions (page {page})...")
        response = requests.get(TRANSACTIONS_API_URL, params=params)
        response.raise_for_status()
        data = response.json()
        
        transactions = data.get("items", [])
        logger.info(f"Fetched {len(transactions)} transactions from page {page}")
        return transactions
    except Exception as e:
        logger.error(f"Error fetching transactions: {str(e)}")
        return []

def fetch_all_transactions(firebase_user_id, workspace, items_per_page=1000, max_pages=None, rpm=30):
    """
    Fetch all transactions from the API.
    
    Args:
        firebase_user_id: The Firebase user ID for authentication
        workspace: The workspace name
        items_per_page: Number of items per page
        max_pages: Maximum number of pages to fetch (None for all)
        rpm: Requests per minute
        
    Returns:
        List of all transaction objects
    """
    global current_page
    all_transactions = []
    page = 1
    
    while True:
        if max_pages and page > max_pages:
            logger.info(f"Reached maximum number of pages ({max_pages})")
            break
            
        if shutdown_requested:
            logger.info("Shutdown requested. Stopping transaction fetch.")
            break
            
        current_page = page
        transactions = fetch_transactions(
            firebase_user_id,
            workspace,
            items_per_page,
            page,
            "desc",
            rpm
        )
        
        if not transactions:
            logger.info(f"No more transactions found after page {page-1}")
            break
            
        all_transactions.extend(transactions)
        
        if len(transactions) < items_per_page:
            logger.info(f"Last page reached ({page})")
            break
            
        page += 1
    
    logger.info(f"Fetched a total of {len(all_transactions)} transactions")
    return all_transactions

def decode_multisend_data(data):
    """
    Decode MultiSend contract data to extract recipient addresses and amounts.
    
    The MultiSend contract has a function:
    function send(uint[] memory _amounts, address[] memory _addresses)
    
    Args:
        data: Transaction input data
        
    Returns:
        List of recipient addresses
    """
    if not data or len(data) < 10:
        return []
    
    try:
        # Remove the function selector (first 10 characters)
        data = data[10:]
        
        # The data format for arrays is:
        # 1. Offset to the first array (amounts) - 32 bytes
        # 2. Offset to the second array (addresses) - 32 bytes
        # 3. First array length - 32 bytes
        # 4. First array elements - 32 bytes each
        # 5. Second array length - 32 bytes
        # 6. Second array elements - 32 bytes each
        
        # Get the offset to the second array (addresses)
        offset_to_addresses = int(data[64:128], 16) * 2  # Convert to bytes offset
        
        # The offset is from the start of the data (after function selector)
        # So we need to find where the addresses array starts in the data
        addresses_start_pos = offset_to_addresses
        
        # Get the length of the addresses array
        addresses_length_hex = data[addresses_start_pos:addresses_start_pos+64]
        addresses_length = int(addresses_length_hex, 16)
        
        # Extract each address
        addresses = []
        for i in range(addresses_length):
            # Each address is 32 bytes (64 hex chars), starting after the length field
            addr_start = addresses_start_pos + 64 + (i * 64)
            addr_end = addr_start + 64
            
            if addr_end <= len(data):
                # Addresses are padded to 32 bytes, so we need to take the last 20 bytes (40 hex chars)
                padded_addr = data[addr_start:addr_end]
                # Extract the actual address (last 40 characters) and add 0x prefix
                address = "0x" + padded_addr[-40:].lower()
                addresses.append(address)
        
        return addresses
    except Exception as e:
        logger.error(f"Error decoding MultiSend data: {str(e)}")
        return []

def extract_active_wallets(transactions):
    """
    Extract active wallet addresses from transactions.
    
    Args:
        transactions: List of transaction objects
        
    Returns:
        Set of unique wallet addresses
    """
    active_wallets = set()
    multisend_txs_count = 0
    multisend_addresses_count = 0
    
    logger.info("Extracting active wallet addresses from transactions...")
    
    for tx in tqdm(transactions, desc="Processing transactions"):
        if shutdown_requested:
            break
            
        # Add from address
        from_addr = tx.get("from", "")
        if from_addr:
            active_wallets.add(from_addr.lower())
        
        # Add to address
        to_addr = tx.get("to", "")
        if to_addr:
            active_wallets.add(to_addr.lower())
            
            # Check if this is a transaction to the MultiSend contract
            if to_addr.lower() == MULTISEND_CONTRACT_ADDRESS.lower():
                data = tx.get("input", "") or tx.get("data", "")
                if data:
                    # Decode MultiSend data to extract recipient addresses
                    multisend_addresses = decode_multisend_data(data)
                    if multisend_addresses:
                        multisend_txs_count += 1
                        multisend_addresses_count += len(multisend_addresses)
                        for addr in multisend_addresses:
                            if addr and len(addr) >= 42:  # Valid Ethereum address length
                                active_wallets.add(addr.lower())
            
        # Check for internal transactions in receipt logs
        receipt = tx.get("receipt", {})
        logs = receipt.get("logs", [])
        
        for log in logs:
            # Check for ERC20 Transfer events
            if log.get("topics") and len(log.get("topics", [])) >= 3:
                # Topic[1] is the from address, Topic[2] is the to address
                # Remove the padding and convert to address format
                try:
                    if log.get("topics")[1].startswith("0x"):
                        from_log = "0x" + log.get("topics")[1][26:].lower()
                        active_wallets.add(from_log)
                    
                    if log.get("topics")[2].startswith("0x"):
                        to_log = "0x" + log.get("topics")[2][26:].lower()
                        active_wallets.add(to_log)
                except (IndexError, AttributeError):
                    continue
        
        # Check for ERC20 transfers in transaction data
        data = tx.get("input", "") or tx.get("data", "")
        
        # Check if this is a transfer method call (transfer or transferFrom)
        if data.startswith("0xa9059cbb"):  # transfer(address,uint256)
            try:
                # Extract recipient address from transfer data
                # Format: 0xa9059cbb + 32 bytes (address padded) + 32 bytes (amount)
                recipient = "0x" + data[34:74].lower()
                active_wallets.add(recipient)
            except Exception as e:
                logger.error(f"Error parsing transfer data: {str(e)}")
        
        elif data.startswith("0x23b872dd"):  # transferFrom(address,address,uint256)
            try:
                # Extract sender and recipient addresses from transferFrom data
                # Format: 0x23b872dd + 32 bytes (from addr) + 32 bytes (to addr) + 32 bytes (amount)
                sender = "0x" + data[34:74].lower()
                recipient = "0x" + data[98:138].lower()
                active_wallets.add(sender)
                active_wallets.add(recipient)
            except Exception as e:
                logger.error(f"Error parsing transferFrom data: {str(e)}")
        
        # Check method details for decoded transfers
        method_details = tx.get("methodDetails", {})
        if method_details and method_details.get("name") == "transfer":
            # Try to extract addresses from the decoded method details
            try:
                label = method_details.get("label", "")
                if "dst:" in label:
                    # Extract address after "dst:"
                    dst_part = label.split("dst:")[1].split(",")[0].strip()
                    if dst_part.startswith("0x"):
                        active_wallets.add(dst_part.lower())
            except Exception as e:
                logger.error(f"Error parsing method details: {str(e)}")
    
    # Remove null address and contract addresses
    addresses_to_remove = [
        "0x",
        "0x0",
        "0x0000000000000000000000000000000000000000"
    ]
    
    for addr in addresses_to_remove:
        if addr in active_wallets:
            active_wallets.remove(addr)
    
    logger.info(f"Found {len(active_wallets)} unique active wallet addresses")
    if multisend_txs_count > 0:
        logger.info(f"Processed {multisend_txs_count} MultiSend transactions with {multisend_addresses_count} recipient addresses")
    
    return active_wallets

def fetch_cxs_balances(addresses, rpc_url, rpm=30, save_interval=100, output_prefix=None):
    """
    Fetch CXS balances for a list of addresses from the RPC endpoint.
    
    Args:
        addresses: List of addresses to fetch balances for
        rpc_url: RPC endpoint URL
        rpm: Requests per minute
        save_interval: Save intermediate results every N addresses
        output_prefix: Prefix for intermediate output files
        
    Returns:
        Dictionary mapping addresses to their balances
    """
    if not rpc_url:
        logger.error("RPC URL is required to fetch balances")
        return {}
        
    logger.info(f"Fetching CXS balances for {len(addresses)} addresses from {rpc_url}")
    
    # Connect to the RPC endpoint
    try:
        w3 = Web3(Web3.HTTPProvider(rpc_url))
        # Check connection by trying to get the block number
        block_number = w3.eth.block_number
        logger.info(f"Connected to RPC endpoint: {rpc_url} (current block: {block_number})")
    except Exception as e:
        logger.error(f"Error connecting to RPC endpoint: {str(e)}")
        return {}
    
    # Setup rate limiter
    rate_limiter = RateLimiter(rpm)
    
    # Prepare output prefix
    if output_prefix is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_prefix = f"active_wallets_with_balances_{timestamp}"
    
    # Fetch balances
    balances = {}
    addresses_with_balance = 0
    total_balance = Decimal(0)
    
    for i, address in enumerate(tqdm(addresses, desc="Fetching CXS balances")):
        if shutdown_requested:
            logger.info("Shutdown requested. Saving current progress...")
            break
            
        try:
            # Wait if needed to respect rate limit
            rate_limiter.wait_if_needed()
            
            # Fetch balance
            balance_wei = w3.eth.get_balance(Web3.toChecksumAddress(address))
            balance_eth = Decimal(balance_wei) / Decimal(10 ** CXS_DECIMALS)
            
            # Store balance
            balances[address] = {
                "balance_wei": str(balance_wei),
                "balance": str(balance_eth)
            }
            
            # Update stats
            if balance_wei > 0:
                addresses_with_balance += 1
                total_balance += balance_eth
                
        except Exception as e:
            logger.error(f"Error fetching balance for {address}: {str(e)}")
            balances[address] = {
                "balance_wei": "0",
                "balance": "0",
                "error": str(e)
            }
            
        # Save intermediate results
        if save_interval > 0 and (i + 1) % save_interval == 0:
            intermediate_file = f"{output_prefix}_intermediate_{i+1}.json"
            save_balances_to_file(addresses[:i+1], balances, intermediate_file)
            logger.info(f"Saved intermediate results for {i+1}/{len(addresses)} addresses to {intermediate_file}")
    
    # Log stats
    logger.info(f"Fetched balances for {len(balances)}/{len(addresses)} addresses")
    logger.info(f"Found {addresses_with_balance} addresses with non-zero balances")
    logger.info(f"Total CXS balance: {total_balance}")
    
    return balances

def save_balances_to_file(addresses, balances, output_file):
    """
    Save addresses with balances to a JSON file.
    
    Args:
        addresses: List of addresses
        balances: Dictionary mapping addresses to balances
        output_file: Output file path
    """
    # Calculate stats
    addresses_with_balance = sum(1 for addr in balances if balances.get(addr, {}).get("balance_wei", "0") != "0")
    total_balance = sum(Decimal(balances.get(addr, {}).get("balance", "0")) for addr in balances)
    
    # Prepare data for saving
    data = {
        "metadata": {
            "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            "total_wallets": len(addresses),
            "wallets_with_balance": addresses_with_balance,
            "total_balance": str(total_balance)
        },
        "wallets": {}
    }
    
    # Add wallet data
    for addr in addresses:
        if addr in balances:
            data["wallets"][addr] = balances[addr]
        else:
            data["wallets"][addr] = {
                "balance_wei": "0",
                "balance": "0"
            }
    
    # Save to file
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)
    
    return output_file

def save_to_file(active_wallets, output_file=None, balances=None):
    """
    Save active wallet addresses to a JSON file.
    
    Args:
        active_wallets: Set of active wallet addresses
        output_file: Output file path
        balances: Dictionary mapping addresses to balances
    """
    if output_file is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_file = f"active_wallets_{timestamp}.json"
    
    # Convert set to list for JSON serialization
    wallet_list = list(active_wallets)
    
    # Prepare data for saving
    if balances:
        # Calculate stats
        addresses_with_balance = sum(1 for addr in wallet_list if balances.get(addr, {}).get("balance_wei", "0") != "0")
        total_balance = sum(Decimal(balances.get(addr, {}).get("balance", "0")) for addr in wallet_list)
        
        data = {
            "metadata": {
                "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
                "total_wallets": len(active_wallets),
                "wallets_with_balance": addresses_with_balance,
                "total_balance": str(total_balance)
            },
            "wallets": {}
        }
        
        # Add wallet data with balances
        for addr in wallet_list:
            if addr in balances:
                data["wallets"][addr] = balances[addr]
            else:
                data["wallets"][addr] = {
                    "balance_wei": "0",
                    "balance": "0"
                }
    else:
        # Simple format without balances
        data = {
            "metadata": {
                "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
                "total_wallets": len(active_wallets)
            },
            "wallets": wallet_list
        }
    
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)
    
    if balances:
        logger.info(f"Saved {len(active_wallets)} active wallet addresses with balances to {output_file}")
    else:
        logger.info(f"Saved {len(active_wallets)} active wallet addresses to {output_file}")
    
    return output_file

def save_progress(transactions, active_wallets=None):
    """Save progress to a file."""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"active_wallets_progress_{timestamp}.json"
    
    data = {
        "metadata": {
            "timestamp": timestamp,
            "current_page": current_page,
            "transactions_fetched": len(transactions)
        },
        "transactions": transactions
    }
    
    if active_wallets:
        data["active_wallets"] = list(active_wallets)
    
    with open(filename, 'w') as f:
        json.dump(data, f, indent=2)
    
    logger.info(f"Saved progress to {filename}")

def main():
    global MULTISEND_CONTRACT_ADDRESS
    
    parser = argparse.ArgumentParser(description="Fetch active wallet addresses from transaction history")
    
    # Required arguments
    parser.add_argument("--firebase-user-id", required=True, help="Firebase user ID for authentication")
    parser.add_argument("--workspace", required=True, help="Workspace name")
    
    # Optional arguments
    parser.add_argument("--rpm", type=int, default=30, help="Requests per minute (default: 30)")
    parser.add_argument("--items-per-page", type=int, default=1000, help="Number of items per page (default: 1000)")
    parser.add_argument("--max-pages", type=int, help="Maximum number of pages to fetch (default: all)")
    parser.add_argument("--output", help="Output file path (default: active_wallets_TIMESTAMP.json)")
    parser.add_argument("--resume", help="Resume from a progress file")
    parser.add_argument("--multisend-address", default=MULTISEND_CONTRACT_ADDRESS, 
                        help=f"MultiSend contract address (default: {MULTISEND_CONTRACT_ADDRESS})")
    
    # Balance fetching arguments
    parser.add_argument("--fetch-balances", action="store_true", help="Fetch CXS balances for all addresses")
    parser.add_argument("--rpc-url", help="RPC endpoint URL (required if --fetch-balances is specified)")
    parser.add_argument("--save-interval", type=int, default=100, 
                        help="Save intermediate results every N addresses during balance fetching (default: 100)")
    parser.add_argument("--resume-balances", help="Resume balance fetching from an intermediate file")
    
    args = parser.parse_args()
    
    # Check if balance fetching is enabled but RPC URL is not provided
    if args.fetch_balances and not args.rpc_url:
        logger.error("RPC URL is required when --fetch-balances is specified")
        return 1
    
    # Update MultiSend contract address if provided
    MULTISEND_CONTRACT_ADDRESS = args.multisend_address
    
    logger.info("Starting active wallets fetch")
    logger.info(f"Rate limit: {args.rpm} requests per minute")
    logger.info(f"Items per page: {args.items_per_page}")
    logger.info(f"MultiSend contract address: {MULTISEND_CONTRACT_ADDRESS}")
    if args.fetch_balances:
        logger.info(f"Will fetch CXS balances from RPC: {args.rpc_url}")
    
    try:
        # Check if resuming balance fetching
        if args.resume_balances:
            try:
                with open(args.resume_balances, 'r') as f:
                    resume_data = json.load(f)
                    if "wallets" in resume_data:
                        # Extract addresses and balances
                        addresses = list(resume_data["wallets"].keys())
                        balances = {addr: resume_data["wallets"][addr] for addr in addresses}
                        logger.info(f"Resuming with {len(addresses)} addresses and {len(balances)} balances from {args.resume_balances}")
                        
                        # Save the final result
                        output_file = args.output or f"active_wallets_with_balances_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                        save_to_file(set(addresses), output_file, balances)
                        logger.info("Balance fetch completed successfully")
                        return 0
                    else:
                        logger.warning(f"Could not find wallets in resume file. Starting from scratch.")
            except Exception as e:
                logger.error(f"Error loading balance resume file: {str(e)}")
                logger.info("Starting from scratch")
        
        transactions = []
        
        # Check if resuming transaction fetch
        if args.resume:
            try:
                with open(args.resume, 'r') as f:
                    resume_data = json.load(f)
                    if "transactions" in resume_data:
                        transactions = resume_data["transactions"]
                        logger.info(f"Resuming with {len(transactions)} transactions from {args.resume}")
                    else:
                        logger.warning(f"Could not find transactions in resume file. Starting from scratch.")
            except Exception as e:
                logger.error(f"Error loading resume file: {str(e)}")
                logger.info("Starting from scratch")
        
        # If not resuming or resume failed, fetch transactions
        if not transactions:
            transactions = fetch_all_transactions(
                args.firebase_user_id,
                args.workspace,
                args.items_per_page,
                args.max_pages,
                args.rpm
            )
        
        if not transactions:
            logger.error("No transactions found")
            return 1
        
        # Extract active wallets
        active_wallets = extract_active_wallets(transactions)
        
        if not active_wallets:
            logger.error("No active wallets found")
            return 1
        
        # Fetch balances if requested
        if args.fetch_balances:
            # Convert set to list for ordered processing
            wallet_list = list(active_wallets)
            
            # Fetch balances
            balances = fetch_cxs_balances(
                wallet_list,
                args.rpc_url,
                args.rpm,
                args.save_interval,
                args.output
            )
            
            # Save to file with balances
            if balances:
                output_file = args.output or f"active_wallets_with_balances_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                save_to_file(active_wallets, output_file, balances)
            else:
                logger.error("Failed to fetch balances")
                # Save addresses without balances as fallback
                output_file = args.output or f"active_wallets_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                save_to_file(active_wallets, output_file)
        else:
            # Save to file without balances
            save_to_file(active_wallets, args.output)
        
        logger.info("Active wallets fetch completed successfully")
        
    except KeyboardInterrupt:
        logger.info("Process interrupted by user")
        if 'transactions' in locals() and transactions:
            save_progress(transactions)
        return 1
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        if 'transactions' in locals() and transactions:
            save_progress(transactions)
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main()) 