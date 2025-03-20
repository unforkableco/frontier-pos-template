#!/usr/bin/env python3
"""
Script to fetch CXS price in USD from the provided API endpoint.
"""

import json
import requests
import logging
from datetime import datetime
import argparse
from decimal import Decimal
import os

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("cxs_price_fetch.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# Price API endpoint
PRICE_API_URL = "https://2prhxgnjz3tt2lqu4c45rnikbu0jjega.lambda-url.us-east-1.on.aws/"

def fetch_prices():
    """
    Fetch CXS and NEXTEP prices from the API.
    Returns a dictionary with the prices.
    """
    try:
        logger.info(f"Fetching prices from {PRICE_API_URL}")
        response = requests.get(PRICE_API_URL)
        response.raise_for_status()  # Raise an exception for HTTP errors
        
        data = response.json()
        logger.info(f"Successfully fetched prices: {data}")
        
        return data
    except requests.exceptions.RequestException as e:
        logger.error(f"Error fetching prices: {str(e)}")
        raise

def format_price(price_str):
    """Format price for display with appropriate decimal places."""
    price = Decimal(price_str)
    
    if price < Decimal('0.0001'):
        # For very small prices, show more decimal places
        return f"${price:.10f}"
    elif price < Decimal('0.01'):
        return f"${price:.6f}"
    elif price < Decimal('1'):
        return f"${price:.4f}"
    else:
        return f"${price:.2f}"

def save_to_file(prices, output_file):
    """Save the prices to a JSON file with timestamp."""
    prices_with_timestamp = {
        **prices,
        "timestamp": datetime.now().isoformat()
    }
    
    with open(output_file, 'w') as f:
        json.dump(prices_with_timestamp, f, indent=2)
    
    logger.info(f"Saved prices to {output_file}")

def main():
    parser = argparse.ArgumentParser(description='Fetch CXS price in USD')
    parser.add_argument('--output', type=str, default=None,
                        help='Output file path (default: cxs_price_TIMESTAMP.json)')
    parser.add_argument('--save', action='store_true',
                        help='Save prices to a file')
    
    args = parser.parse_args()
    
    # Generate default output filename with timestamp if saving is enabled
    if args.save and args.output is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        args.output = f"cxs_price_{timestamp}.json"
    
    try:
        prices = fetch_prices()
        
        # Display prices
        print("\n=== Current Prices ===")
        print(f"CXS: {format_price(prices['cxs_price_usd'])} USD")
        print(f"NEXTEP: {format_price(prices['nextep_price_usd'])} USD")
        print("=====================\n")
        
        # Calculate some example values
        cxs_price = Decimal(prices['cxs_price_usd'])
        
        print("Example CXS Holdings Value:")
        for amount in [100, 1000, 10000, 100000]:
            value = amount * cxs_price
            print(f"{amount:,} CXS = ${value:.2f} USD")
        
        # Save to file if requested
        if args.save:
            save_to_file(prices, args.output)
        
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main()) 