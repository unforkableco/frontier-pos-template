const { ethers } = require('ethers');

// Usage: node submit_evm_tx.js <http_rpc_url> <sender_private_key> <to_address> <value_wei> <data_hex>
// Example: node submit_evm_tx.js http://localhost:8545 0xPRIVATE_KEY 0xPRECOMPILE_ADDRESS 0 0xa9059cbb...

async function main() {
    const args = process.argv.slice(2);
    if (args.length !== 5) {
        console.error('Usage: node submit_evm_tx.js <http_rpc_url> <sender_private_key> <to_address> <value_wei> <data_hex>');
        process.exit(1);
    }

    const httpRpcUrl = args[0];
    const senderPrivateKey = args[1];
    const toAddress = args[2];
    const valueWei = args[3];
    const dataHex = args[4];

    const provider = new ethers.JsonRpcProvider(httpRpcUrl);
    const wallet = new ethers.Wallet(senderPrivateKey, provider);

    // console.log(`Using EVM sender: ${wallet.address}`);

    try {
        const txRequest = {
            to: toAddress,
            value: BigInt(valueWei), // Ensure value is BigInt
            data: dataHex,
            // Let provider estimate gas price and limit
        };

        // console.log('Sending EVM transaction:', txRequest);
        const txResponse = await wallet.sendTransaction(txRequest);
        // console.log(`EVM Transaction submitted, hash: ${txResponse.hash}`);

        // Wait for the transaction to be included in a block (optional, but good practice for validation scripts)
        // const receipt = await txResponse.wait(); 
        // if (receipt.status === 0) {
        //     console.error('EVM Transaction failed!', receipt);
        //     process.exit(1);
        // }
        // console.log(`Transaction confirmed in block: ${receipt.blockNumber}`);
        
        // Output only the hash upon successful submission
        console.log(txResponse.hash); 

    } catch (error) {
        console.error('Error submitting EVM transaction:', error);
        process.exit(1);
    }
}

main(); 