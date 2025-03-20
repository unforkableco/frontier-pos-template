#!/usr/bin/env python3

import argparse
import json
import logging
import os
import sys
import subprocess
from decimal import Decimal

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('combine_wallet_values.log')
    ]
)
logger = logging.getLogger(__name__)

def load_wallet_data(file_path):
    """
    Load wallet data from a JSON file.
    
    Args:
        file_path: Path to the JSON file
        
    Returns:
        Dictionary containing the wallet data
    """
    try:
        logger.info(f"Loading wallet data from {file_path}")
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        # Validate the data structure
        if "metadata" not in data or "wallets" not in data:
            logger.error(f"Invalid data structure in {file_path}")
            return None
            
        logger.info(f"Successfully loaded data for {len(data['wallets'])} wallets")
        return data
    except Exception as e:
        logger.error(f"Error loading wallet data from {file_path}: {str(e)}")
        return None

def get_token_price(token_symbol):
    """
    Get the current price of a token using fetch_cxs_price.py script.
    
    Args:
        token_symbol: Symbol of the token (CXS or NEXTEP)
        
    Returns:
        Current price in USD as a Decimal
    """
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

def combine_wallet_values(cxs_data, nextep_data, cxs_price, nextep_price):
    """
    Combine wallet values from CXS and NEXTEP data and calculate USD values.
    
    Args:
        cxs_data: Dictionary containing CXS wallet data
        nextep_data: Dictionary containing NEXTEP wallet data
        cxs_price: Current CXS price in USD
        nextep_price: Current NEXTEP price in USD
        
    Returns:
        Dictionary mapping addresses to their combined USD values
    """
    combined_wallets = {}
    
    # Process CXS wallets
    for address, wallet_data in cxs_data["wallets"].items():
        balance = Decimal(wallet_data.get("balance", "0"))
        usd_value = balance * cxs_price
        
        combined_wallets[address] = {
            "cxs_balance": balance,
            "cxs_usd_value": usd_value,
            "nextep_balance": Decimal("0"),
            "nextep_usd_value": Decimal("0"),
            "total_usd_value": usd_value
        }
    
    # Process NEXTEP wallets
    for address, wallet_data in nextep_data["wallets"].items():
        balance = Decimal(wallet_data.get("balance", "0"))
        usd_value = balance * nextep_price
        
        if address in combined_wallets:
            # Update existing wallet
            combined_wallets[address]["nextep_balance"] = balance
            combined_wallets[address]["nextep_usd_value"] = usd_value
            combined_wallets[address]["total_usd_value"] += usd_value
        else:
            # Add new wallet
            combined_wallets[address] = {
                "cxs_balance": Decimal("0"),
                "cxs_usd_value": Decimal("0"),
                "nextep_balance": balance,
                "nextep_usd_value": usd_value,
                "total_usd_value": usd_value
            }
    
    return combined_wallets

def calculate_totals(data):
    """
    Calculate total balances and values from wallet data.
    
    Args:
        data: Dictionary mapping addresses to wallet data
        
    Returns:
        Dictionary containing total balances and values
    """
    totals = {
        "total_wallets": len(data),
        "total_cxs_balance": Decimal("0"),
        "total_nextep_balance": Decimal("0"),
        "total_cxs_usd_value": Decimal("0"),
        "total_nextep_usd_value": Decimal("0"),
        "total_usd_value": Decimal("0")
    }
    
    for address, wallet_data in data.items():
        totals["total_cxs_balance"] += wallet_data["cxs_balance"]
        totals["total_nextep_balance"] += wallet_data["nextep_balance"]
        totals["total_cxs_usd_value"] += wallet_data["cxs_usd_value"]
        totals["total_nextep_usd_value"] += wallet_data["nextep_usd_value"]
        totals["total_usd_value"] += wallet_data["total_usd_value"]
    
    return totals

def save_combined_data(combined_wallets, totals, cxs_price, nextep_price, output_file):
    """
    Save combined wallet data to a JSON file.
    
    Args:
        combined_wallets: Dictionary mapping addresses to combined wallet data
        totals: Dictionary containing total balances and values
        cxs_price: Current CXS price in USD
        nextep_price: Current NEXTEP price in USD
        output_file: Output file path
    """
    # Create a simplified version with only USD values
    simplified_wallets = {}
    for address, wallet_data in combined_wallets.items():
        simplified_wallets[address] = {
            "usd_value": str(wallet_data["total_usd_value"])
        }
    
    # Prepare data for saving
    data = {
        "metadata": {
            "total_wallets": totals["total_wallets"],
            "total_usd_value": str(totals["total_usd_value"]),
            "cxs_price_usd": str(cxs_price),
            "nextep_price_usd": str(nextep_price),
            "total_cxs_balance": str(totals["total_cxs_balance"]),
            "total_nextep_balance": str(totals["total_nextep_balance"]),
            "total_cxs_usd_value": str(totals["total_cxs_usd_value"]),
            "total_nextep_usd_value": str(totals["total_nextep_usd_value"])
        },
        "wallets": simplified_wallets
    }
    
    try:
        with open(output_file, 'w') as f:
            json.dump(data, f, indent=2)
        logger.info(f"Saved combined wallet data to {output_file}")
    except Exception as e:
        logger.error(f"Error saving combined wallet data: {str(e)}")

def main():
    parser = argparse.ArgumentParser(description="Combine CXS and NEXTEP wallet data and calculate USD values")
    
    # Required arguments
    parser.add_argument("--cxs-file", required=True, help="Path to the CXS wallet data file")
    parser.add_argument("--nextep-file", required=True, help="Path to the NEXTEP wallet data file")
    parser.add_argument("--output", required=True, help="Output file path for combined wallet data")
    
    # Optional arguments
    parser.add_argument("--cxs-price", type=float, help="Override CXS price in USD (default: fetch from API)")
    parser.add_argument("--nextep-price", type=float, help="Override NEXTEP price in USD (default: fetch from API)")
    
    args = parser.parse_args()
    
    # Load wallet data
    cxs_data = load_wallet_data(args.cxs_file)
    nextep_data = load_wallet_data(args.nextep_file)
    
    if not cxs_data or not nextep_data:
        logger.error("Failed to load wallet data")
        return 1
    
    # Print total amounts from the original files
    cxs_total = cxs_data["metadata"].get("total_balance", "0")
    nextep_total = nextep_data["metadata"].get("total_balance", "0")
    
    logger.info(f"Total CXS from {args.cxs_file}: {cxs_total}")
    logger.info(f"Total NEXTEP from {args.nextep_file}: {nextep_total}")
    
    print(f"Total CXS: {cxs_total}")
    print(f"Total NEXTEP: {nextep_total}")
    
    # Get token prices
    cxs_price = Decimal(str(args.cxs_price)) if args.cxs_price is not None else get_token_price("CXS")
    nextep_price = Decimal(str(args.nextep_price)) if args.nextep_price is not None else get_token_price("NEXTEP")
    
    if cxs_price == Decimal('0') or nextep_price == Decimal('0'):
        logger.warning("One or both token prices could not be fetched. USD values may be inaccurate.")
    
    # Combine wallet values
    combined_wallets = combine_wallet_values(cxs_data, nextep_data, cxs_price, nextep_price)
    
    # Calculate totals
    totals = calculate_totals(combined_wallets)
    
    # Print totals
    print(f"Combined wallets: {totals['total_wallets']}")
    print(f"Total CXS: {totals['total_cxs_balance']}")
    print(f"Total NEXTEP: {totals['total_nextep_balance']}")
    print(f"CXS price: ${cxs_price}")
    print(f"NEXTEP price: ${nextep_price}")
    print(f"Total USD value: ${totals['total_usd_value']}")
    
    # Save combined data
    save_combined_data(combined_wallets, totals, cxs_price, nextep_price, args.output)
    
    logger.info("Combined wallet values successfully")
    return 0

if __name__ == "__main__":
    sys.exit(main()) 