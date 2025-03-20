#!/usr/bin/env python3

import argparse
import json
import logging
import os
import sys
from decimal import Decimal, getcontext

# Set decimal precision
getcontext().prec = 28

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('revo_distribution.log')
    ]
)
logger = logging.getLogger(__name__)

def load_combined_data(file_path):
    """
    Load combined wallet data from a JSON file.
    
    Args:
        file_path: Path to the JSON file
        
    Returns:
        Dictionary containing the combined wallet data
    """
    try:
        logger.info(f"Loading combined wallet data from {file_path}")
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        # Validate the data structure
        if "metadata" not in data or "wallets" not in data:
            logger.error(f"Invalid data structure in {file_path}")
            return None
            
        logger.info(f"Successfully loaded data for {len(data['wallets'])} wallets")
        return data
    except Exception as e:
        logger.error(f"Error loading combined wallet data from {file_path}: {str(e)}")
        return None

def calculate_revo_distribution(combined_data, revo_price):
    """
    Calculate REVO token distribution based on USD values.
    
    Args:
        combined_data: Dictionary containing combined wallet data
        revo_price: Price of REVO token in USD
        
    Returns:
        Dictionary mapping addresses to their REVO token allocation
    """
    if revo_price <= 0:
        logger.error("REVO price must be greater than zero")
        return None
        
    revo_distribution = {}
    total_usd_value = Decimal("0")
    total_revo_tokens = Decimal("0")
    
    # Calculate REVO tokens for each wallet
    for address, wallet_data in combined_data["wallets"].items():
        usd_value = Decimal(wallet_data["usd_value"])
        revo_tokens = usd_value / Decimal(str(revo_price))
        
        revo_distribution[address] = {
            "usd_value": str(usd_value),
            "revo_tokens": str(revo_tokens)
        }
        
        total_usd_value += usd_value
        total_revo_tokens += revo_tokens
    
    logger.info(f"Calculated REVO distribution for {len(revo_distribution)} wallets")
    logger.info(f"Total USD value: ${total_usd_value}")
    logger.info(f"Total REVO tokens: {total_revo_tokens}")
    
    return {
        "metadata": {
            "total_wallets": len(revo_distribution),
            "total_usd_value": str(total_usd_value),
            "revo_price_usd": str(revo_price),
            "total_revo_tokens": str(total_revo_tokens)
        },
        "wallets": revo_distribution
    }

def save_revo_distribution(revo_data, output_file):
    """
    Save REVO distribution data to a JSON file.
    
    Args:
        revo_data: Dictionary containing REVO distribution data
        output_file: Output file path
    """
    try:
        with open(output_file, 'w') as f:
            json.dump(revo_data, f, indent=2)
        logger.info(f"Saved REVO distribution data to {output_file}")
        return True
    except Exception as e:
        logger.error(f"Error saving REVO distribution data: {str(e)}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Calculate REVO token distribution based on USD values")
    
    # Required arguments
    parser.add_argument("--input", required=True, help="Path to the combined wallet data file")
    parser.add_argument("--output", required=True, help="Output file path for REVO distribution data")
    parser.add_argument("--revo-price", type=float, required=True, help="Price of REVO token in USD")
    
    # Optional arguments
    parser.add_argument("--min-usd", type=float, default=0, help="Minimum USD value to include in distribution (default: 0)")
    
    args = parser.parse_args()
    
    # Validate REVO price
    if args.revo_price <= 0:
        logger.error("REVO price must be greater than zero")
        return 1
    
    # Load combined wallet data
    combined_data = load_combined_data(args.input)
    if not combined_data:
        logger.error("Failed to load combined wallet data")
        return 1
    
    # Filter wallets by minimum USD value if specified
    if args.min_usd > 0:
        original_count = len(combined_data["wallets"])
        filtered_wallets = {}
        
        for address, wallet_data in combined_data["wallets"].items():
            usd_value = Decimal(wallet_data["usd_value"])
            if usd_value >= Decimal(str(args.min_usd)):
                filtered_wallets[address] = wallet_data
        
        combined_data["wallets"] = filtered_wallets
        logger.info(f"Filtered wallets by minimum USD value (${args.min_usd}): {original_count} -> {len(filtered_wallets)}")
    
    # Calculate REVO distribution
    revo_data = calculate_revo_distribution(combined_data, args.revo_price)
    if not revo_data:
        logger.error("Failed to calculate REVO distribution")
        return 1
    
    # Print summary
    print(f"REVO Price: ${args.revo_price}")
    print(f"Total Wallets: {revo_data['metadata']['total_wallets']}")
    print(f"Total USD Value: ${revo_data['metadata']['total_usd_value']}")
    print(f"Total REVO Tokens: {revo_data['metadata']['total_revo_tokens']}")
    
    # Save REVO distribution data
    if not save_revo_distribution(revo_data, args.output):
        logger.error("Failed to save REVO distribution data")
        return 1
    
    logger.info("REVO distribution calculation completed successfully")
    return 0

if __name__ == "__main__":
    sys.exit(main()) 