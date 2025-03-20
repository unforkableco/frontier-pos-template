#!/usr/bin/env python3
"""
Script to fetch all holders of the CXS coin and their balances from the NX Chain RPC.
Includes rate limiting to avoid overusing the RPC endpoint.
"""

import json
import time
import os
import requests
from web3 import Web3
from decimal import Decimal
import argparse
import logging
from datetime import datetime
import signal
import sys

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("cxs_holders_fetch.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# RPC endpoint
RPC_URL = "https://rpc.nxchainscan.com/"

# CXS token has 18 decimals
CXS_DECIMALS = 18

# Rate limiting settings
DEFAULT_REQUESTS_PER_MINUTE = 30
DEFAULT_BATCH_SIZE = 100
DEFAULT_PAUSE_SECONDS = 5

# Global variables for graceful shutdown
shutdown_requested = False
current_accounts = {}
output_file = None

def signal_handler(sig, frame):
    """Handle Ctrl+C and other termination signals."""
    global shutdown_requested
    logger.info("Shutdown requested. Saving current progress...")
    shutdown_requested = True

class RateLimiter:
    """Simple rate limiter to control requests to the RPC endpoint."""
    
    def __init__(self, requests_per_minute):
        self.requests_per_minute = requests_per_minute
        self.request_times = []
        
    def wait_if_needed(self):
        """Wait if we've exceeded our rate limit."""
        current_time = time.time()
        
        # Remove request times older than 1 minute
        self.request_times = [t for t in self.request_times if current_time - t < 60]
        
        # If we've made too many requests in the last minute, wait
        if len(self.request_times) >= self.requests_per_minute:
            oldest_request = self.request_times[0]
            sleep_time = 60 - (current_time - oldest_request)
            if sleep_time > 0:
                logger.info(f"Rate limit reached. Sleeping for {sleep_time:.2f} seconds")
                time.sleep(sleep_time)
        
        # Record this request
        self.request_times.append(time.time())

class CXSHoldersFetcher:
    """Fetches all CXS holders and their balances."""
    
    def __init__(self, rpc_url, requests_per_minute, batch_size, pause_seconds):
        self.web3 = Web3(Web3.HTTPProvider(rpc_url))
        self.rate_limiter = RateLimiter(requests_per_minute)
        self.batch_size = batch_size
        self.pause_seconds = pause_seconds
        
        if not self.web3.is_connected():
            raise ConnectionError(f"Failed to connect to RPC endpoint: {rpc_url}")
        
        logger.info(f"Connected to RPC endpoint: {rpc_url}")
        logger.info(f"Chain ID: {self.web3.eth.chain_id}")
        
    def get_latest_block(self):
        """Get the latest block number."""
        self.rate_limiter.wait_if_needed()
        return self.web3.eth.block_number
    
    def get_accounts_with_balance(self):
        """
        Fetch all accounts with non-zero CXS balance.
        Returns a dictionary of account addresses and their balances.
        """
        global current_accounts, shutdown_requested
        
        latest_block = self.get_latest_block()
        logger.info(f"Latest block: {latest_block}")
        
        accounts = {}
        processed_count = 0
        
        # We'll scan blocks in batches to find transactions and extract addresses
        start_block = 1  # Start from the first block
        
        while start_block <= latest_block and not shutdown_requested:
            end_block = min(start_block + self.batch_size - 1, latest_block)
            logger.info(f"Processing blocks {start_block} to {end_block}")
            
            # Process each block in the batch
            for block_num in range(start_block, end_block + 1):
                if shutdown_requested:
                    break
                    
                self.rate_limiter.wait_if_needed()
                
                try:
                    block = self.web3.eth.get_block(block_num, full_transactions=True)
                    
                    # Process all transactions in the block
                    for tx in block.transactions:
                        # Add 'from' address
                        from_addr = tx['from']
                        if from_addr not in accounts:
                            accounts[from_addr] = None
                        
                        # Add 'to' address if it exists (not contract creation)
                        if tx['to'] is not None:
                            to_addr = tx['to']
                            if to_addr not in accounts:
                                accounts[to_addr] = None
                    
                    processed_count += 1
                    if processed_count % 10 == 0:
                        logger.info(f"Processed {processed_count} blocks, found {len(accounts)} unique addresses")
                        # Update global accounts for graceful shutdown
                        current_accounts = accounts.copy()
                
                except Exception as e:
                    logger.error(f"Error processing block {block_num}: {str(e)}")
            
            # Save intermediate results every 1000 blocks
            if processed_count % 1000 == 0 and output_file:
                self.save_intermediate_results(accounts, f"{output_file}.intermediate")
            
            # Pause between batches to avoid overwhelming the RPC
            if end_block < latest_block and not shutdown_requested:
                logger.info(f"Pausing for {self.pause_seconds} seconds before next batch")
                time.sleep(self.pause_seconds)
            
            start_block = end_block + 1
        
        # If shutdown was requested, return current accounts
        if shutdown_requested:
            logger.info("Shutdown requested during block processing. Returning current accounts.")
            return accounts
        
        # Now fetch balances for all addresses
        logger.info(f"Fetching balances for {len(accounts)} addresses")
        
        address_count = 0
        for address in list(accounts.keys()):
            if shutdown_requested:
                break
                
            self.rate_limiter.wait_if_needed()
            
            try:
                balance_wei = self.web3.eth.get_balance(address)
                balance_cxs = Decimal(balance_wei) / Decimal(10 ** CXS_DECIMALS)
                
                # Only keep addresses with non-zero balance
                if balance_wei > 0:
                    accounts[address] = balance_cxs
                else:
                    del accounts[address]
                
                address_count += 1
                if address_count % 100 == 0:
                    logger.info(f"Fetched balances for {address_count}/{len(accounts)} addresses")
                    # Update global accounts for graceful shutdown
                    current_accounts = accounts.copy()
                
                # Save intermediate results every 1000 addresses
                if address_count % 1000 == 0 and output_file:
                    self.save_intermediate_results(accounts, f"{output_file}.intermediate")
            
            except Exception as e:
                logger.error(f"Error fetching balance for {address}: {str(e)}")
                accounts[address] = None
        
        # Filter out addresses with None balances (errors)
        accounts = {addr: bal for addr, bal in accounts.items() if bal is not None}
        
        return accounts
    
    def save_to_file(self, accounts, output_file):
        """Save the accounts and balances to a JSON file."""
        # Convert Decimal objects to strings for JSON serialization
        serializable_accounts = {addr: str(bal) for addr, bal in accounts.items()}
        
        with open(output_file, 'w') as f:
            json.dump(serializable_accounts, f, indent=2)
        
        logger.info(f"Saved {len(accounts)} accounts to {output_file}")
    
    def save_intermediate_results(self, accounts, output_file):
        """Save intermediate results to a file."""
        # Convert Decimal objects to strings for JSON serialization
        serializable_accounts = {addr: str(bal) if bal is not None else None for addr, bal in accounts.items()}
        
        with open(output_file, 'w') as f:
            json.dump(serializable_accounts, f, indent=2)
        
        logger.info(f"Saved intermediate results ({len(accounts)} accounts) to {output_file}")

def main():
    global output_file, current_accounts
    
    # Set up signal handlers for graceful shutdown
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    parser = argparse.ArgumentParser(description='Fetch CXS holders and their balances')
    parser.add_argument('--rpm', type=int, default=DEFAULT_REQUESTS_PER_MINUTE,
                        help=f'Requests per minute (default: {DEFAULT_REQUESTS_PER_MINUTE})')
    parser.add_argument('--batch', type=int, default=DEFAULT_BATCH_SIZE,
                        help=f'Batch size for block processing (default: {DEFAULT_BATCH_SIZE})')
    parser.add_argument('--pause', type=int, default=DEFAULT_PAUSE_SECONDS,
                        help=f'Pause seconds between batches (default: {DEFAULT_PAUSE_SECONDS})')
    parser.add_argument('--output', type=str, default=None,
                        help='Output file path (default: cxs_holders_TIMESTAMP.json)')
    parser.add_argument('--resume', type=str, default=None,
                        help='Resume from an intermediate file')
    
    args = parser.parse_args()
    
    # Generate default output filename with timestamp if not provided
    if args.output is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        args.output = f"cxs_holders_{timestamp}.json"
    
    output_file = args.output
    
    logger.info("Starting CXS holders fetch")
    logger.info(f"Rate limit: {args.rpm} requests per minute")
    logger.info(f"Batch size: {args.batch} blocks")
    logger.info(f"Pause between batches: {args.pause} seconds")
    
    try:
        fetcher = CXSHoldersFetcher(
            RPC_URL, 
            requests_per_minute=args.rpm,
            batch_size=args.batch,
            pause_seconds=args.pause
        )
        
        # If resuming from an intermediate file
        if args.resume and os.path.exists(args.resume):
            logger.info(f"Resuming from intermediate file: {args.resume}")
            with open(args.resume, 'r') as f:
                accounts_data = json.load(f)
                # Convert string balances back to Decimal
                accounts = {addr: Decimal(bal) if bal is not None else None for addr, bal in accounts_data.items()}
                logger.info(f"Loaded {len(accounts)} accounts from intermediate file")
                current_accounts = accounts
        else:
            accounts = fetcher.get_accounts_with_balance()
        
        # If shutdown was requested, use the current_accounts
        if shutdown_requested:
            accounts = current_accounts
        
        # Filter out addresses with None balances (errors)
        accounts = {addr: bal for addr, bal in accounts.items() if bal is not None}
        
        fetcher.save_to_file(accounts, args.output)
        
        logger.info(f"Found {len(accounts)} accounts with non-zero CXS balance")
        
        # Print some statistics
        balances = [float(bal) for bal in accounts.values()]
        if balances:
            total_cxs = sum(balances)
            avg_cxs = total_cxs / len(balances)
            max_cxs = max(balances)
            min_cxs = min(balances)
            
            logger.info(f"Total CXS: {total_cxs}")
            logger.info(f"Average CXS per holder: {avg_cxs}")
            logger.info(f"Max CXS: {max_cxs}")
            logger.info(f"Min CXS: {min_cxs}")
        
        logger.info("CXS holders fetch completed successfully")
        
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        
        # If we have current_accounts, try to save them
        if current_accounts and output_file:
            try:
                logger.info("Saving current progress before exit...")
                # Filter out addresses with None balances (errors)
                accounts = {addr: bal for addr, bal in current_accounts.items() if bal is not None}
                
                # Save to a recovery file
                recovery_file = f"{output_file}.recovery"
                fetcher.save_to_file(accounts, recovery_file)
                logger.info(f"Saved recovery data to {recovery_file}")
            except Exception as save_error:
                logger.error(f"Error saving recovery data: {str(save_error)}")
        
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main()) 