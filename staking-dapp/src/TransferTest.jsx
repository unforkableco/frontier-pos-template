import { useState } from 'react';
import { ethers } from 'ethers';

// Adresse du précompile de balances
const BALANCES_PRECOMPILE_ADDRESS = '0x0000000000000000000000000000000000000800';

function TransferTest() {
  const [result, setResult] = useState('');
  const [loading, setLoading] = useState(false);
  
  // Méthode 1: Envoi direct avec value et data
  const testMethod1 = async () => {
    setLoading(true);
    setResult('');
    
    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      await provider.send("eth_requestAccounts", []);
      const signer = provider.getSigner();
      const fromAddress = await signer.getAddress();
      
      const toAddress = '0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac';
      const amount = ethers.utils.parseEther('1.0');
      
      // Format d'adresse
      const cleanAddress = toAddress.slice(2).toLowerCase();
      const paddedAddress = '000000000000000000000000' + cleanAddress;
      const hexAmount = amount.toHexString().slice(2).padStart(64, '0');
      
      // Data
      const data = `0x00000003${paddedAddress}${hexAmount}`;
      
      console.log('Method 1:');
      console.log('From:', fromAddress);
      console.log('To (precompile):', BALANCES_PRECOMPILE_ADDRESS);
      console.log('Data:', data);
      console.log('Amount:', amount.toString());
      
      const tx = await signer.sendTransaction({
        to: BALANCES_PRECOMPILE_ADDRESS,
        data: data,
        value: amount,
        gasLimit: 1000000,
      });
      
      const receipt = await tx.wait();
      setResult(`Méthode 1 - Succès! Hash: ${receipt.transactionHash}`);
    } catch (error) {
      console.error('Method 1 error:', error);
      setResult(`Méthode 1 - Erreur: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };
  
  // Méthode 2: Envoi avec value seulement
  const testMethod2 = async () => {
    setLoading(true);
    setResult('');
    
    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      await provider.send("eth_requestAccounts", []);
      const signer = provider.getSigner();
      
      const toAddress = '0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac';
      const amount = ethers.utils.parseEther('1.0');
      
      console.log('Method 2:');
      console.log('Transferring directly to:', toAddress);
      console.log('Amount:', amount.toString());
      
      const tx = await signer.sendTransaction({
        to: toAddress,
        value: amount,
        gasLimit: 1000000,
      });
      
      const receipt = await tx.wait();
      setResult(`Méthode 2 - Succès! Hash: ${receipt.transactionHash}`);
    } catch (error) {
      console.error('Method 2 error:', error);
      setResult(`Méthode 2 - Erreur: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };
  
  return (
    <div className="p-6 max-w-lg mx-auto bg-white rounded-lg shadow-md mt-10">
      <h1 className="text-2xl font-bold mb-6">Test de Transfert</h1>
      
      <div className="space-y-4">
        <button
          onClick={testMethod1}
          disabled={loading}
          className="w-full p-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-blue-300"
        >
          Méthode 1: Utiliser le précompile balance_transfer
        </button>
        
        <button
          onClick={testMethod2}
          disabled={loading}
          className="w-full p-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-green-300"
        >
          Méthode 2: Transfert direct ETH/ROKO
        </button>
        
        {loading && <p className="text-center text-gray-700">Traitement en cours...</p>}
        
        {result && (
          <div className="mt-4 p-3 bg-gray-100 rounded">
            <pre className="whitespace-pre-wrap">{result}</pre>
          </div>
        )}
      </div>
    </div>
  );
}

export default TransferTest; 