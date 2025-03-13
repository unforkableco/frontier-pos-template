# Dockerized Substrate Node AWS Deployment

This document explains how to deploy a dockerized Substrate test network to AWS using the GitHub workflow.

## Prerequisites

Before you can use the deployment workflow, you need to set up the following GitHub secrets:

1. `AWS_ACCESS_KEY_ID`: Your AWS access key ID
2. `AWS_SECRET_ACCESS_KEY`: Your AWS secret access key
3. `AWS_REGION`: The AWS region to deploy to (e.g., `us-east-1`)

## Deployment Process

1. Go to the GitHub repository's "Actions" tab
2. Select the "Deploy Dockerized Substrate Node on AWS" workflow
3. Click "Run workflow"
4. Enter the required parameter:
   - `release_version`: The version tag for this deployment (e.g., `v1.0.0`)
5. Click "Run workflow" to start the deployment

## What Happens During Deployment

The workflow performs the following steps:

1. Creates a new security group with the necessary port rules for both nodes
2. Creates a new EC2 instance with the security group
3. The EC2 instance automatically:
   - Installs Docker and Docker Compose
   - Clones the repository to get the Docker Compose configuration
   - Starts both Alice and Bob nodes using the Docker Compose file
   - Configures the nodes to restart automatically on system boot
4. Outputs the node's IP address and connection endpoints for both nodes

## Fully Automated Deployment

This deployment is fully automated and doesn't require any manual SSH connection to the instance. The entire setup is handled by the user-data script that runs when the EC2 instance is launched.

The setup script pulls the repository directly, ensuring that the deployed nodes use the exact same Docker Compose configuration that's in the repository. This makes it easier to maintain and update the configuration in the future.

## Accessing the Nodes

Once deployment is complete, you can access both nodes using the following endpoints:

### Alice Node
- RPC/WebSocket: `ws://<node-ip>:9944`
- P2P: `/ip4/<node-ip>/tcp/30333`

### Bob Node
- RPC/WebSocket: `ws://<node-ip>:8545`
- P2P: `/ip4/<node-ip>/tcp/30334`

## Security Considerations

The deployment opens the following ports to the public internet:

- Port 22 (SSH)
- Port 9944 (Alice RPC/WebSocket)
- Port 8545 (Bob RPC/WebSocket)
- Ports 30333-30334 (P2P)

For production deployments, consider restricting access to these ports to specific IP ranges.

## Troubleshooting

If the deployment fails, check the following:

1. Verify that all required secrets are correctly set up
2. Check the AWS console for any issues with the EC2 instance
3. Check the EC2 instance's system log in the AWS console for any errors in the user-data script execution
4. If needed, you can SSH into the instance and check the Docker logs:
   ```
   ssh -i <path-to-key> ubuntu@<node-ip>
   cd /home/ubuntu/substrate-node
   docker compose logs
   ``` 