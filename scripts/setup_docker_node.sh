#!/bin/bash

set -e  # Exit immediately if any command fails
export DEBIAN_FRONTEND=noninteractive

echo "🚀 Starting Docker Node Setup..."

# Update system packages
echo "🔄 Updating system packages..."
apt-get update && apt-get upgrade -y

# Install dependencies
echo "📦 Installing dependencies..."
apt-get install -y apt-transport-https ca-certificates curl software-properties-common jq git

# Install Docker
echo "🐳 Installing Docker..."
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Clone the repository
echo "📥 Cloning the repository..."
mkdir -p /home/ubuntu
cd /home/ubuntu
git clone https://github.com/unforkableco/frontier-pos-template.git substrate-node
cd substrate-node

# Create directories for node data
echo "📁 Creating directories for node data..."
mkdir -p db/alice db/bob

# Create a systemd service to start Docker Compose on boot
echo "⚙️ Creating systemd service for Docker Compose..."
cat > /etc/systemd/system/substrate-docker.service <<EOF
[Unit]
Description=Substrate Docker Node
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/ubuntu/substrate-node
ExecStart=/usr/bin/docker compose up -d
ExecStop=/usr/bin/docker compose down
User=ubuntu

[Install]
WantedBy=multi-user.target
EOF

# Set permissions
chown -R ubuntu:ubuntu /home/ubuntu/substrate-node

# Create a script to monitor and clean up disk space
cat > /home/ubuntu/monitor_disk_space.sh <<EOF
#!/bin/bash

# Check disk usage
USAGE=\$(df / | tail -1 | awk '{print \$5}' | tr -d '%')

# If usage is over 85%, prune old blockchain data
if [ \$USAGE -gt 85 ]; then
  echo "\$(date) - Disk usage is at \${USAGE}%. Cleaning up old blockchain data."
  
  # Stop the service
  systemctl stop substrate-docker.service
  
  # Remove old blockchain data
  rm -rf /home/ubuntu/substrate-node/db/alice/chains/frontier-testnet/paritydb/*
  rm -rf /home/ubuntu/substrate-node/db/bob/chains/frontier-testnet/paritydb/*
  
  # Start the service again
  systemctl start substrate-docker.service
  
  echo "\$(date) - Cleanup completed. Service restarted."
fi
EOF

chmod +x /home/ubuntu/monitor_disk_space.sh
chown ubuntu:ubuntu /home/ubuntu/monitor_disk_space.sh

# Add cron job to run the script daily
(crontab -l 2>/dev/null; echo "0 0 * * * /home/ubuntu/monitor_disk_space.sh >> /home/ubuntu/disk_monitor.log 2>&1") | crontab -u ubuntu -

# Enable and start the service
systemctl daemon-reload
systemctl enable substrate-docker.service
systemctl start substrate-docker.service

echo "✅ Docker Node setup complete!"
echo "Substrate nodes are now running!"
echo "Alice RPC endpoint: ws://localhost:9944"
echo "Bob RPC endpoint: ws://localhost:8545" 