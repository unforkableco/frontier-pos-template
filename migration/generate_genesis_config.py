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
        logging.FileHandler('genesis_generation.log')
    ]
)
logger = logging.getLogger(__name__)

# Default chain parameters
DEFAULT_CHAIN_ID = "revo-1"
DEFAULT_CHAIN_NAME = "REVO Mainnet"
DEFAULT_CONSENSUS_PARAMS = {
    "block": {
        "max_bytes": "22020096",
        "max_gas": "10000000",
        "time_iota_ms": "1000"
    },
    "evidence": {
        "max_age_num_blocks": "100000",
        "max_age_duration": "172800000000000",
        "max_bytes": "1048576"
    },
    "validator": {
        "pub_key_types": ["ed25519"]
    },
    "version": {}
}

def load_revo_distribution(file_path):
    """
    Load REVO distribution data from a JSON file.
    
    Args:
        file_path: Path to the JSON file
        
    Returns:
        Dictionary containing the REVO distribution data
    """
    try:
        logger.info(f"Loading REVO distribution data from {file_path}")
        with open(file_path, 'r') as f:
            data = json.load(f)
        
        # Validate the data structure
        if "metadata" not in data or "wallets" not in data:
            logger.error(f"Invalid data structure in {file_path}")
            return None
            
        logger.info(f"Successfully loaded data for {len(data['wallets'])} wallets")
        return data
    except Exception as e:
        logger.error(f"Error loading REVO distribution data from {file_path}: {str(e)}")
        return None

def convert_to_base_units(amount, decimals=18):
    """
    Convert token amount to base units (wei/atto).
    
    Args:
        amount: Token amount as a string or Decimal
        decimals: Number of decimal places for the token
        
    Returns:
        String representation of the amount in base units
    """
    amount_decimal = Decimal(amount)
    multiplier = Decimal(10) ** Decimal(decimals)
    base_units = amount_decimal * multiplier
    
    # Convert to integer string (no decimal point)
    return str(int(base_units))

def generate_genesis_config(revo_data, chain_id, chain_name, consensus_params, decimals=18):
    """
    Generate a genesis configuration for a new blockchain with pre-allocated REVO balances.
    
    Args:
        revo_data: Dictionary containing REVO distribution data
        chain_id: Chain ID for the new blockchain
        chain_name: Name of the new blockchain
        consensus_params: Consensus parameters for the blockchain
        decimals: Number of decimal places for the REVO token
        
    Returns:
        Dictionary containing the genesis configuration
    """
    logger.info(f"Generating genesis configuration for chain: {chain_id}")
    
    # Initialize genesis structure
    genesis = {
        "genesis_time": "2023-01-01T00:00:00.000000000Z",  # Placeholder, will be updated at launch
        "chain_id": chain_id,
        "initial_height": "1",
        "consensus_params": consensus_params,
        "app_hash": "",
        "app_state": {
            "auth": {
                "params": {
                    "max_memo_characters": "256",
                    "tx_sig_limit": "7",
                    "tx_size_cost_per_byte": "10",
                    "sig_verify_cost_ed25519": "590",
                    "sig_verify_cost_secp256k1": "1000"
                },
                "accounts": []
            },
            "bank": {
                "params": {
                    "send_enabled": True,
                    "default_send_enabled": True
                },
                "balances": [],
                "supply": [],
                "denom_metadata": [
                    {
                        "description": "The native token of the REVO blockchain",
                        "denom_units": [
                            {
                                "denom": "arevo",  # Base unit (like wei in Ethereum)
                                "exponent": 0,
                                "aliases": ["attorevo"]
                            },
                            {
                                "denom": "revo",  # Main unit
                                "exponent": decimals,
                                "aliases": []
                            }
                        ],
                        "base": "arevo",
                        "display": "revo",
                        "name": "REVO",
                        "symbol": "REVO"
                    }
                ]
            },
            "staking": {
                "params": {
                    "unbonding_time": "1814400000000000",  # 3 weeks in nanoseconds
                    "max_validators": 100,
                    "max_entries": 7,
                    "historical_entries": 10000,
                    "bond_denom": "arevo"
                },
                "validators": [],
                "delegations": [],
                "unbonding_delegations": [],
                "redelegations": [],
                "exported": False
            },
            "distribution": {
                "params": {
                    "community_tax": "0.020000000000000000",  # 2%
                    "base_proposer_reward": "0.010000000000000000",  # 1%
                    "bonus_proposer_reward": "0.040000000000000000",  # 4%
                    "withdraw_addr_enabled": True
                },
                "fee_pool": {
                    "community_pool": []
                },
                "delegator_withdraw_infos": [],
                "previous_proposer": "",
                "outstanding_rewards": [],
                "validator_accumulated_commissions": [],
                "validator_historical_rewards": [],
                "validator_current_rewards": [],
                "delegator_starting_infos": [],
                "validator_slash_events": []
            },
            "gov": {
                "starting_proposal_id": "1",
                "deposits": [],
                "votes": [],
                "proposals": [],
                "deposit_params": {
                    "min_deposit": [
                        {
                            "denom": "arevo",
                            "amount": convert_to_base_units("10", decimals)  # 10 REVO
                        }
                    ],
                    "max_deposit_period": "172800000000000"  # 2 days in nanoseconds
                },
                "voting_params": {
                    "voting_period": "172800000000000"  # 2 days in nanoseconds
                },
                "tally_params": {
                    "quorum": "0.334000000000000000",  # 33.4%
                    "threshold": "0.500000000000000000",  # 50%
                    "veto_threshold": "0.334000000000000000"  # 33.4%
                }
            }
        }
    }
    
    # Add accounts and balances
    total_supply = Decimal("0")
    accounts = []
    balances = []
    
    for address, wallet_data in revo_data["wallets"].items():
        revo_tokens = Decimal(wallet_data["revo_tokens"])
        if revo_tokens > 0:
            # Convert to base units (arevo)
            base_units = convert_to_base_units(revo_tokens, decimals)
            total_supply += revo_tokens
            
            # Add account
            account = {
                "address": address,
                "account_number": "0",  # Will be assigned sequentially at genesis
                "sequence": "0"
            }
            accounts.append(account)
            
            # Add balance
            balance = {
                "address": address,
                "coins": [
                    {
                        "denom": "arevo",
                        "amount": base_units
                    }
                ]
            }
            balances.append(balance)
    
    # Update genesis with accounts and balances
    genesis["app_state"]["auth"]["accounts"] = accounts
    genesis["app_state"]["bank"]["balances"] = balances
    
    # Add total supply
    genesis["app_state"]["bank"]["supply"] = [
        {
            "denom": "arevo",
            "amount": convert_to_base_units(total_supply, decimals)
        }
    ]
    
    logger.info(f"Genesis configuration generated with {len(accounts)} accounts")
    logger.info(f"Total REVO supply: {total_supply}")
    
    return genesis

def save_genesis_config(genesis_config, output_file):
    """
    Save genesis configuration to a JSON file.
    
    Args:
        genesis_config: Dictionary containing the genesis configuration
        output_file: Output file path
    """
    try:
        with open(output_file, 'w') as f:
            json.dump(genesis_config, f, indent=2)
        logger.info(f"Saved genesis configuration to {output_file}")
        return True
    except Exception as e:
        logger.error(f"Error saving genesis configuration: {str(e)}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Generate a genesis configuration for a new blockchain with pre-allocated REVO balances")
    
    # Required arguments
    parser.add_argument("--input", required=True, help="Path to the REVO distribution data file")
    parser.add_argument("--output", required=True, help="Output file path for genesis configuration")
    
    # Optional arguments
    parser.add_argument("--chain-id", default=DEFAULT_CHAIN_ID, help=f"Chain ID for the new blockchain (default: {DEFAULT_CHAIN_ID})")
    parser.add_argument("--chain-name", default=DEFAULT_CHAIN_NAME, help=f"Name of the new blockchain (default: {DEFAULT_CHAIN_NAME})")
    parser.add_argument("--decimals", type=int, default=18, help="Number of decimal places for the REVO token (default: 18)")
    parser.add_argument("--min-revo", type=float, default=0, help="Minimum REVO tokens to include in genesis (default: 0)")
    
    args = parser.parse_args()
    
    # Load REVO distribution data
    revo_data = load_revo_distribution(args.input)
    if not revo_data:
        logger.error("Failed to load REVO distribution data")
        return 1
    
    # Filter wallets by minimum REVO tokens if specified
    if args.min_revo > 0:
        original_count = len(revo_data["wallets"])
        filtered_wallets = {}
        
        for address, wallet_data in revo_data["wallets"].items():
            revo_tokens = Decimal(wallet_data["revo_tokens"])
            if revo_tokens >= Decimal(str(args.min_revo)):
                filtered_wallets[address] = wallet_data
        
        revo_data["wallets"] = filtered_wallets
        logger.info(f"Filtered wallets by minimum REVO tokens ({args.min_revo}): {original_count} -> {len(filtered_wallets)}")
    
    # Generate genesis configuration
    genesis_config = generate_genesis_config(
        revo_data,
        args.chain_id,
        args.chain_name,
        DEFAULT_CONSENSUS_PARAMS,
        args.decimals
    )
    
    if not genesis_config:
        logger.error("Failed to generate genesis configuration")
        return 1
    
    # Save genesis configuration
    if not save_genesis_config(genesis_config, args.output):
        logger.error("Failed to save genesis configuration")
        return 1
    
    logger.info("Genesis configuration generation completed successfully")
    return 0

if __name__ == "__main__":
    sys.exit(main()) 