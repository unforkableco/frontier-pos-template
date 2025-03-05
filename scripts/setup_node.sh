#!/bin/bash

set -e  # Exit immediately if any command fails
export DEBIAN_FRONTEND=noninteractive

# Variables
GITHUB_REPO="unforkableco/frontier-pos-template"
NODE_BINARY="main-node"
CHAIN_SPEC="custom"  # Change to your chain's spec if needed
RPC_PORT=9944
WS_PORT=9944

echo "🚀 Starting Node Setup..."

# Update system packages
echo "🔄 Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install dependencies
echo "📦 Installing dependencies..."
sudo apt install -y curl jq tmux systemd

# Fetch latest GitHub release binary
echo "⬇️ Downloading the Substrate node binary..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | jq -r '.tag_name')
BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$LATEST_RELEASE/$NODE_BINARY"

wget -O /tmp/${NODE_BINARY} ${BINARY_URL}
sudo mv /tmp/${NODE_BINARY} /usr/local/bin/substrate-node
sudo chmod +x /usr/local/bin/substrate-node

# Create a systemd service
echo "⚙️ Configuring systemd service..."
sudo tee /etc/systemd/system/substrate-node.service > /dev/null <<EOF
[Unit]
Description=Substrate Node
After=network.target

[Service]
User=ubuntu
ExecStart=/usr/local/bin/substrate-node --name "Test network" --rpc-external --rpc-port $RPC_PORT --pruning=archive --rpc-cors all
Restart=always
RestartSec=10
LimitNOFILE=4096

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and start the node
echo "🚀 Starting Substrate node service..."
sudo systemctl daemon-reload
sudo systemctl enable substrate-node
sudo systemctl start substrate-node

# Check status
echo "✅ Node setup complete! Checking status..."
sleep 5
sudo systemctl status substrate-node --no-pager
