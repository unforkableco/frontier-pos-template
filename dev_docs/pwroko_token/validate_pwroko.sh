#!/bin/bash

# Automated Validation Script for pallet-pwroko and its EVM Precompile

set -e # Exit immediately if a command exits with a non-zero status.
set -o pipefail # Causes pipelines to fail on the first command that fails

# --- Configuration ---
NODE_BINARY="./target/release/substrate" # Adjust path if needed
NODE_ARGS="--database auto --alice --dev --tmp" 
NODE_LOG="node.log"
NODE_SUBSTRATE_WS_URL="ws://localhost:9944" # Substrate WS RPC (Check your node's port, default is 9944)
NODE_SUBSTRATE_HTTP_URL="http://localhost:9933" # Substrate HTTP RPC
NODE_EVM_RPC_URL="http://localhost:8545" # EVM RPC (check your node setup)
PRECOMPILE_ADDRESS="0x0000000000000000000000000000000000000500"

# --- WARNING: Hardcoding private keys/seeds is insecure for non-development environments! --- 
ALICE_SEED="//Alice"
# Standard derivation for //Alice EVM key
ALICE_EVM_PRIVATE_KEY="0x5e98cce00ca00d01e85d1197224565513248011efe6c5345080537599660d155"
# ------------------------------------------------------------------------------------------

ALICE_SUBSTRATE="5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" # Default dev address
ALICE_SS58_HEX="0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d" # hex representation for storage keys
ALICE_EVM="0xf24FF3a9CF04c71DBC94D0b566f7A27B94566cac" # Default dev EVM address derived from Alice Substrate key
BOB_SUBSTRATE="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty" # Default dev address
BOB_SS58_HEX="0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"
BOB_EVM="0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0" # Default dev EVM address derived from Bob Substrate key

# Amounts need to be adjusted based on your chain's token decimals
# Assuming 12 decimals for ROKO/pwRoko based on common Substrate defaults
DECIMALS=12
LOCK_AMOUNT_BASE=100
TRANSFER_AMOUNT_BASE=30
LOCK_AMOUNT=$(echo "$LOCK_AMOUNT_BASE * 10^$DECIMALS" | bc)
TRANSFER_AMOUNT=$(echo "$TRANSFER_AMOUNT_BASE * 10^$DECIMALS" | bc)

# --- Helper Functions ---
info() {
    echo "[INFO] $1"
}

error() {
    echo "[ERROR] $1" >&2
}

# Function to query Substrate storage via RPC
# Usage: query_substrate_storage <method> <storage_key_hex> <expected_value_hex_or_null>
query_substrate_storage() {
    local method=$1
    local key=$2
    local expected=$3
    local resp=$(curl -s -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\":\"$method\", \"params\":[\"$key\"]}" $NODE_SUBSTRATE_HTTP_URL)
    local result=$(echo $resp | jq -r '.result')
    info "Query $method $key -> Got $result (Expected: $expected)"
    if [ "$result" != "$expected" ]; then
        error "Substrate storage query failed! Key: $key. Expected: $expected, Got: $result"
        echo "Full response: $resp"
        return 1
    fi
    return 0
}

# Function to query EVM state via RPC
# Usage: query_evm <method> <params_json_array> <expected_value_hex_or_jq_expr>
query_evm() {
    local method=$1
    local params=$2
    local expected=$3
    local resp=$(curl -s -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\":\"$method\", \"params\":$params}" $NODE_EVM_RPC_URL)
    local result=$(echo $resp | jq -r '.result')
    info "Query $method $params -> Got $result (Expected: $expected)"
    
    # If expected starts with '.', treat it as a jq expression for complex checks
    if [[ "$expected" == .* ]]; then 
        if ! echo $resp | jq -e "$expected" > /dev/null; then
            error "EVM query check failed! Method: $method, Params: $params. Expected jq expr: $expected"
            echo "Full response: $resp"
            return 1
        fi
    elif [ "$result" != "$expected" ]; then
        error "EVM query failed! Method: $method, Params: $params. Expected: $expected, Got: $result"
        echo "Full response: $resp"
        return 1
    fi
    return 0
}

cleanup() {
    info "Cleaning up..."
    if [ ! -z "$NODE_PID" ]; then
        info "Stopping node (PID: $NODE_PID)..."
        kill $NODE_PID || echo "Node already stopped."
    fi
    rm -f $NODE_LOG # Clean up log file
    info "Cleanup complete."
}

trap cleanup EXIT ERR

# --- Prerequisite Checks ---
if ! command -v curl &> /dev/null; then error "curl not found."; exit 1; fi
if ! command -v jq &> /dev/null; then error "jq not found."; exit 1; fi
if ! command -v bc &> /dev/null; then error "bc not found."; exit 1; fi
if ! command -v node &> /dev/null; then error "node not found."; exit 1; fi
if ! command -v npm &> /dev/null; then error "npm not found."; exit 1; fi

# --- Validation Steps ---

# 1. Install Helper Dependencies
info "Installing Node.js helper script dependencies..."
(cd dev_docs/pwroko_token/helpers && npm install)
if [ $? -ne 0 ]; then error "npm install failed."; exit 1; fi

# 2. Build Node (Optional - uncomment if needed)
# info "Building node..."
# cargo build --release --features=testnet # Or mainnet
# if [ $? -ne 0 ]; then error "Node build failed."; exit 1; fi

# 3. Start Node
info "Starting node in background... Log: $NODE_LOG"
$NODE_BINARY $NODE_ARGS &> $NODE_LOG &
NODE_PID=$!
sleep 5 # Give node time to start

# 4. Wait for RPC
info "Waiting for node RPC..."
MAX_WAIT=30
COUNT=0
while ! curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' $NODE_SUBSTRATE_HTTP_URL | jq -e .result > /dev/null; do
    sleep 2
    COUNT=$((COUNT+2))
    if [ $COUNT -ge $MAX_WAIT ]; then
        error "Node RPC did not become available after $MAX_WAIT seconds."
        cat $NODE_LOG
        exit 1
    fi
    echo -n "."
done
info "Node RPC is up!"

# Construct Storage Keys (Hex)
# Use tools like subkey or polkadot-js to inspect storage and find correct prefixes
# Example prefixes (likely incorrect, MUST be verified for your specific runtime):
BALANCES_ACCOUNT_PREFIX="0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9" # System.Account prefix
PWROKO_LOCKED_PREFIX="0x$(echo -n "PwRoko" | xxd -p)2aae6c72759993346cca58904c931175" # PwRoko.LockedBalances prefix - NEEDS VERIFICATION
PWROKO_BALANCES_PREFIX="0x$(echo -n "PwRoko" | xxd -p)c081db052d53dd8597eb587383db9a47" # PwRoko.Balances prefix - NEEDS VERIFICATION
PWROKO_TOTAL_PREFIX="0x$(echo -n "PwRoko" | xxd -p)f2756dba016e70147985bf1428d598e5" # PwRoko.TotalSupply prefix - NEEDS VERIFICATION

ALICE_BALANCES_KEY="$BALANCES_ACCOUNT_PREFIX$(echo -n $ALICE_SS58_HEX | sed 's/0x//')"
ALICE_PWROKO_LOCKED_KEY="$PWROKO_LOCKED_PREFIX$(echo -n $ALICE_SS58_HEX | sed 's/0x//')"
ALICE_PWROKO_BALANCES_KEY="$PWROKO_BALANCES_PREFIX$(echo -n $ALICE_SS58_HEX | sed 's/0x//')"

# Construct EVM calldata
BALANCEOF_ALICE_DATA="0x70a08231$(echo $ALICE_EVM | sed 's/0x//' | awk '{ printf "%064s", $0 }' | sed 's/ /0/g')"
BALANCEOF_BOB_DATA="0x70a08231$(echo $BOB_EVM | sed 's/0x//' | awk '{ printf "%064s", $0 }' | sed 's/ /0/g')"

# Convert amounts to hex (right-padded)
LOCK_AMOUNT_HEX=$(printf '%064x' $LOCK_AMOUNT)
TRANSFER_AMOUNT_HEX=$(printf '%064x' $TRANSFER_AMOUNT)

# 5. Initial State Checks (Substrate & EVM)
info "Checking initial state for Alice ($ALICE_SUBSTRATE / $ALICE_EVM)..."
# Note: Balances query checks the whole AccountInfo struct (nonce, consumers, providers, sufficients, data{free,reserved,frozen})
# We only check the existence and expect reserved/frozen to be 0 initially for simple test cases.
query_substrate_storage "state_getStorage" $ALICE_BALANCES_KEY '.result | contains({reserved: "0x00000000000000000000000000000000"})' 
query_substrate_storage "state_getStorage" $ALICE_PWROKO_LOCKED_KEY "null" # Expect null as ValueQuery default is 0, which isn't stored
query_substrate_storage "state_getStorage" $ALICE_PWROKO_BALANCES_KEY "null"
query_substrate_storage "state_getStorage" $PWROKO_TOTAL_PREFIX "null"
query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_ALICE_DATA'"}, "latest"]' "0x0000000000000000000000000000000000000000000000000000000000000000"
info "Initial state checks passed."

# 6. Lock ROKO (via EVM Precompile)
info "Submitting lock($LOCK_AMOUNT) transaction via EVM..."
LOCK_CALL_DATA="0xe14a1a54${LOCK_AMOUNT_HEX}"
LOCK_TX_HASH=$(node dev_docs/pwroko_token/helpers/submit_evm_tx.js "$NODE_EVM_RPC_URL" "$ALICE_EVM_PRIVATE_KEY" "$PRECOMPILE_ADDRESS" 0 "$LOCK_CALL_DATA")
if [ $? -ne 0 ] || [[ "$LOCK_TX_HASH" != 0x* ]]; then
    error "Failed to submit lock transaction. See script output above."
    exit 1
fi
info "Lock transaction submitted, tx hash: $LOCK_TX_HASH. Waiting for block..."
sleep 6 # Wait for block finalization (adjust based on block time)

# 7. Post-Lock State Checks (Substrate & EVM)
info "Checking post-lock state..."
LOCK_AMOUNT_STORAGE_HEX="0x$(printf '%x' $LOCK_AMOUNT | tac -rs.. | tr -d '\n')" # Little-endian for direct storage value
LOCK_AMOUNT_BALANCE_HEX="0x$(printf '%032x' $LOCK_AMOUNT)" # Correct padding for balance query
LOCK_AMOUNT_EVM_HEX="0x$(printf '%064x' $LOCK_AMOUNT)" # EVM encoding 
query_substrate_storage "state_getStorage" $ALICE_BALANCES_KEY '.result | contains({reserved: "'$LOCK_AMOUNT_BALANCE_HEX'"})' 
query_substrate_storage "state_getStorage" $ALICE_PWROKO_LOCKED_KEY $LOCK_AMOUNT_STORAGE_HEX
query_substrate_storage "state_getStorage" $ALICE_PWROKO_BALANCES_KEY $LOCK_AMOUNT_STORAGE_HEX
query_substrate_storage "state_getStorage" $PWROKO_TOTAL_PREFIX $LOCK_AMOUNT_STORAGE_HEX
query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_ALICE_DATA'"}, "latest"]' $LOCK_AMOUNT_EVM_HEX
info "Post-lock state checks passed."

# 8. Transfer pwRoko (EVM Precompile)
info "Transferring $TRANSFER_AMOUNT pwRoko from Alice ($ALICE_EVM) to Bob ($BOB_EVM) via EVM..."
# Construct calldata for transfer(address to, uint256 amount)
TRANSFER_CALL_DATA="0xa9059cbb$(echo $BOB_EVM | sed 's/0x//' | awk '{ printf "%064s", $0 }' | sed 's/ /0/g')${TRANSFER_AMOUNT_HEX}"
TRANSFER_TX_HASH=$(node dev_docs/pwroko_token/helpers/submit_evm_tx.js "$NODE_EVM_RPC_URL" "$ALICE_EVM_PRIVATE_KEY" "$PRECOMPILE_ADDRESS" 0 "$TRANSFER_CALL_DATA")
if [ $? -ne 0 ] || [[ "$TRANSFER_TX_HASH" != 0x* ]]; then
    error "Failed to submit EVM transfer. See script output above."
    exit 1
fi
info "EVM transfer submitted, tx hash: $TRANSFER_TX_HASH. Waiting for block..."
sleep 6

# 9. Post-Transfer State Checks (EVM & Substrate)
info "Checking post-transfer state..."
EXPECTED_ALICE_PWROKO=$(($LOCK_AMOUNT - $TRANSFER_AMOUNT))
EXPECTED_BOB_PWROKO=$TRANSFER_AMOUNT
EXPECTED_ALICE_PWROKO_EVM_HEX="0x$(printf '%064x' $EXPECTED_ALICE_PWROKO)"
EXPECTED_BOB_PWROKO_EVM_HEX="0x$(printf '%064x' $EXPECTED_BOB_PWROKO)"
EXPECTED_ALICE_PWROKO_STORAGE_HEX="0x$(printf '%x' $EXPECTED_ALICE_PWROKO | tac -rs.. | tr -d '\n')"
EXPECTED_BOB_PWROKO_STORAGE_HEX="0x$(printf '%x' $EXPECTED_BOB_PWROKO | tac -rs.. | tr -d '\n')"
BOB_PWROKO_BALANCES_KEY="$PWROKO_BALANCES_PREFIX$(echo -n $BOB_SS58_HEX | sed 's/0x//')"

query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_ALICE_DATA'"}, "latest"]' $EXPECTED_ALICE_PWROKO_EVM_HEX
query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_BOB_DATA'"}, "latest"]' $EXPECTED_BOB_PWROKO_EVM_HEX
query_substrate_storage "state_getStorage" $ALICE_PWROKO_BALANCES_KEY $EXPECTED_ALICE_PWROKO_STORAGE_HEX
query_substrate_storage "state_getStorage" $BOB_PWROKO_BALANCES_KEY $EXPECTED_BOB_PWROKO_STORAGE_HEX
query_substrate_storage "state_getStorage" $PWROKO_TOTAL_PREFIX $LOCK_AMOUNT_STORAGE_HEX # Total supply should still be original lock amount
info "Post-transfer state checks passed."

# 10. Unlock ROKO (via EVM Precompile)
info "Submitting unlock($EXPECTED_ALICE_PWROKO) transaction via EVM..."
EXPECTED_ALICE_PWROKO_HEX=$(printf '%064x' $EXPECTED_ALICE_PWROKO)
UNLOCK_CALL_DATA="0x619a363a${EXPECTED_ALICE_PWROKO_HEX}"
UNLOCK_TX_HASH=$(node dev_docs/pwroko_token/helpers/submit_evm_tx.js "$NODE_EVM_RPC_URL" "$ALICE_EVM_PRIVATE_KEY" "$PRECOMPILE_ADDRESS" 0 "$UNLOCK_CALL_DATA")
if [ $? -ne 0 ] || [[ "$UNLOCK_TX_HASH" != 0x* ]]; then
    error "Failed to submit unlock transaction. See script output above."
    exit 1
fi
info "Unlock transaction submitted, tx hash: $UNLOCK_TX_HASH. Waiting for block..."
sleep 6

# 11. Final State Checks (Substrate & EVM)
info "Checking final state..."
# Alice's free balance check is tricky due to gas fees. Check reserved is 0.
query_substrate_storage "state_getStorage" $ALICE_BALANCES_KEY '.result | contains({reserved: "0x00000000000000000000000000000000"})' 
query_substrate_storage "state_getStorage" $ALICE_PWROKO_LOCKED_KEY "null"
query_substrate_storage "state_getStorage" $ALICE_PWROKO_BALANCES_KEY "null"
query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_ALICE_DATA'"}, "latest"]' "0x0000000000000000000000000000000000000000000000000000000000000000"

# Bob should still have his balance
query_evm "eth_call" '[{"to": "'$PRECOMPILE_ADDRESS'", "data": "'$BALANCEOF_BOB_DATA'"}, "latest"]' $EXPECTED_BOB_PWROKO_EVM_HEX
query_substrate_storage "state_getStorage" $BOB_PWROKO_BALANCES_KEY $EXPECTED_BOB_PWROKO_STORAGE_HEX

# Total supply should now reflect only Bob's balance
EXPECTED_TOTAL_SUPPLY_STORAGE_HEX="0x$(printf '%x' $EXPECTED_BOB_PWROKO | tac -rs.. | tr -d '\n')"
query_substrate_storage "state_getStorage" $PWROKO_TOTAL_PREFIX $EXPECTED_TOTAL_SUPPLY_STORAGE_HEX

info "Final state checks passed."

# --- Success ---

info "✅ ✅ ✅ All pwRoko validation steps passed! ✅ ✅ ✅"

# Cleanup is handled by the trap
exit 0 