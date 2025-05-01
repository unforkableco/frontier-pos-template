#!/bin/bash

# Validation script for renaming the native coin to ROKO (18 decimals)

NODE_BINARY="./target/release/substrate"
NODE_ARGS="--database auto --alice --dev"
RPC_URL="http://localhost:9944"
EXPECTED_SYMBOL="ROKO"
EXPECTED_DECIMALS=18

echo "Starting node in background..."
$NODE_BINARY $NODE_ARGS &
NODE_PID=$!
echo "Node started with PID: $NODE_PID"

# Allow time for the node to start
echo "Waiting 10 seconds for node initialization..."
sleep 10

echo "Querying RPC for system properties..."
# Query RPC using curl
# Adding --connect-timeout and --max-time for robustness
# Adding -s for silent mode (no progress meter), -S to show errors
PROPERTIES_JSON=$(curl -s -S --connect-timeout 5 --max-time 10 -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_properties", "params":[]}' $RPC_URL)
CURL_EXIT_CODE=$?

# Check if curl command was successful
if [ $CURL_EXIT_CODE -ne 0 ]; then
    echo "Error: curl command failed with exit code $CURL_EXIT_CODE."
    echo "Failed to query RPC endpoint $RPC_URL."
    echo "Attempting to kill node process $NODE_PID..."
    kill $NODE_PID
    wait $NODE_PID 2>/dev/null
    echo "Node process killed."
    exit 1
fi

echo "Received properties JSON: $PROPERTIES_JSON"

# Check if jq is installed
if ! command -v jq &> /dev/null
then
    echo "Error: jq is not installed. Please install jq to parse JSON response."
    echo "Attempting to kill node process $NODE_PID..."
    kill $NODE_PID
    wait $NODE_PID 2>/dev/null
    echo "Node process killed."
    exit 1
fi

# Parse JSON and check values using jq
ACTUAL_SYMBOL=$(echo $PROPERTIES_JSON | jq -r '.result.tokenSymbol')
ACTUAL_DECIMALS=$(echo $PROPERTIES_JSON | jq -r '.result.tokenDecimals')

echo "Extracted Symbol: $ACTUAL_SYMBOL"
echo "Extracted Decimals: $ACTUAL_DECIMALS"

# Validation
VALIDATION_PASSED=true

if [ "$ACTUAL_SYMBOL" != "$EXPECTED_SYMBOL" ]; then
    echo "Validation FAILED: Expected tokenSymbol '$EXPECTED_SYMBOL', but got '$ACTUAL_SYMBOL'"
    VALIDATION_PASSED=false
else
    echo "Validation SUCCESS: tokenSymbol is '$EXPECTED_SYMBOL' as expected."
fi

if [ "$ACTUAL_DECIMALS" != "$EXPECTED_DECIMALS" ]; then
    echo "Validation FAILED: Expected tokenDecimals '$EXPECTED_DECIMALS', but got '$ACTUAL_DECIMALS'"
    VALIDATION_PASSED=false
else
    echo "Validation SUCCESS: tokenDecimals is '$EXPECTED_DECIMALS' as expected."
fi

# Kill the node process
echo "Shutting down node process $NODE_PID..."
kill $NODE_PID
# Wait for the process to terminate gracefully, suppressing errors if already killed
wait $NODE_PID 2>/dev/null
echo "Node process killed."

# Final result
if [ "$VALIDATION_PASSED" = true ]; then
    echo "----------------------------"
    echo "Overall Validation: SUCCESS"
    echo "----------------------------"
    exit 0
else
    echo "----------------------------"
    echo "Overall Validation: FAILED"
    echo "----------------------------"
    exit 1
fi 