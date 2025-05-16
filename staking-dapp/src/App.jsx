import { useState, useEffect } from 'react';
import { ethers } from 'ethers';
import ConnectButton from './components/ConnectButton';
import BondForm from './components/BondForm';
import UnbondForm from './components/UnbondForm';

// Adresse du précompile de staking
const STAKING_PRECOMPILE_ADDRESS = '0x0000000000000000000000000000000000000700';

// ABI du précompile
const stakingAbi = [
  {
    inputs: [
      { name: 'amount', type: 'uint256' },
      { name: 'payeeType', type: 'uint8' }
    ],
    name: 'bond',
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'payable',
    type: 'function',
    selector: '0x00000001'
  },
  {
    inputs: [
      { name: 'amount', type: 'uint256' }
    ],
    name: 'unbond',
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'nonpayable',
    type: 'function',
    selector: '0x00000002'
  }
];

function App() {
  const [account, setAccount] = useState('');
  const [stakingContract, setStakingContract] = useState(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState('');
  const [txPending, setTxPending] = useState(false);
  const [txResult, setTxResult] = useState(null);
  const [stakingInfo, setStakingInfo] = useState(null);
  const [loading, setLoading] = useState(false);

  // Connecter au portefeuille
  const connectWallet = async () => {
    setError('');
    try {
      if (window.ethereum) {
        console.log("Portefeuille Ethereum détecté");
        
        const provider = new ethers.providers.Web3Provider(window.ethereum);
        await window.ethereum.request({ method: 'eth_requestAccounts' });
        const signer = provider.getSigner();
        const address = await signer.getAddress();
        
        // Créer une instance du contrat de staking
        const stakingContractInstance = new ethers.Contract(
          STAKING_PRECOMPILE_ADDRESS,
          stakingAbi,
          signer
        );
        
        setAccount(address);
        setStakingContract(stakingContractInstance);
        setIsConnected(true);
        
        console.log(`Connecté avec l'adresse: ${address}`);
      } else {
        setError('Aucun portefeuille Ethereum détecté. Veuillez installer MetaMask.');
      }
    } catch (error) {
      console.error('Erreur de connexion:', error);
      setError(`Erreur de connexion: ${error.message}`);
    }
  };

  // Exécuter bond
  const handleBond = async (amount, payeeType) => {
    if (!stakingContract) return;
    
    setError('');
    setTxPending(true);
    setTxResult(null);
    
    try {
      const amountBN = ethers.utils.parseEther(amount.toString());
      console.log(`Bonding ${amountBN} tokens avec payeeType=${payeeType}`);
      console.log("Adresse du contrat:", STAKING_PRECOMPILE_ADDRESS);
      
      // Construire la data complète
      // Modification pour un encodage plus simple et direct
      // Format : sélecteur (8 caractères) + montant (64 caractères) + type (64 caractères)
      // Le montant doit être en hexadécimal et padded à 32 bytes
      
      // Convertir l'amount en hex et padder à 32 bytes (64 caractères)
      const hexAmount = amountBN.toHexString().slice(2).padStart(64, '0');
      // Convertir le payeeType en hex et padder à 32 bytes
      const hexPayeeType = payeeType.toString(16).padStart(64, '0');
      
      // Assembler la data
      const data = `0x00000001${hexAmount}${hexPayeeType}`;
      
      console.log("Data construite:", data);
      
      // Envoyer la transaction
      const signer = (new ethers.providers.Web3Provider(window.ethereum)).getSigner();
      const tx = await signer.sendTransaction({
        to: STAKING_PRECOMPILE_ADDRESS,
        data: data,
        value: amountBN,
        gasLimit: 1000000,
        gasPrice: ethers.utils.parseUnits("1", "gwei")
      });
      
      console.log("Transaction envoyée:", tx.hash);
      
      // Définir un timeout pour attendre la transaction
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Transaction timeout - vérifiez l'explorateur de blocs")), 60000)
      );
      
      // Attendre le premier entre la confirmation et le timeout
      const receipt = await Promise.race([
        tx.wait(),
        timeoutPromise
      ]);
      
      console.log("Transaction confirmée:", receipt);
      
      setTxResult({
        success: true,
        hash: receipt.transactionHash,
        blockNumber: receipt.blockNumber
      });
    } catch (error) {
      console.error('Erreur lors du bond:', error);
      
      // Logging détaillé de l'erreur
      if (error.code) console.error("Code d'erreur:", error.code);
      if (error.method) console.error("Méthode:", error.method);
      if (error.transaction) console.error("Transaction:", error.transaction);
      if (error.error && error.error.message) console.error("Message interne:", error.error.message);
      
      // Si c'est un timeout, on affiche un message spécifique
      if (error.message && error.message.includes("timeout")) {
        setError("La transaction a été envoyée mais attend d'être confirmée. Vérifiez votre portefeuille ou l'explorateur de blocs.");
        setTxResult({
          success: true,
          error: null,
          hash: tx.hash,
          pending: true
        });
      } else {
        setError(`Erreur: ${error.message}`);
        setTxResult({
          success: false,
          error: error.message
        });
      }
    } finally {
      setTxPending(false);
    }
  };

  // Exécuter unbond
  const handleUnbond = async (amount) => {
    if (!stakingContract) return;
    
    setError('');
    setTxPending(true);
    setTxResult(null);
    
    try {
      const amountBN = ethers.utils.parseEther(amount.toString());
      console.log(`Unbonding ${amountBN} tokens`);
      console.log("Adresse du contrat:", STAKING_PRECOMPILE_ADDRESS);
      
      // Comme pour bond, utiliser l'approche directe
      const signer = (new ethers.providers.Web3Provider(window.ethereum)).getSigner();
      
      // Convertir l'amount en hex et padder à 32 bytes (64 caractères)
      const hexAmount = amountBN.toHexString().slice(2).padStart(64, '0');
      
      // Assembler la data
      const data = `0x00000002${hexAmount}`;
      
      console.log("Data construite:", data);
      
      // Envoyer la transaction
      const tx = await signer.sendTransaction({
        to: STAKING_PRECOMPILE_ADDRESS,
        data: data,
        gasLimit: 1000000,
        gasPrice: ethers.utils.parseUnits("1", "gwei")
      });
      console.log("Transaction envoyée:", tx.hash);
      
      // Définir un timeout pour attendre la transaction
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Transaction timeout - vérifiez l'explorateur de blocs")), 60000)
      );
      
      // Attendre le premier entre la confirmation et le timeout
      const receipt = await Promise.race([
        tx.wait(),
        timeoutPromise
      ]);
      
      console.log("Transaction confirmée:", receipt);
      
      setTxResult({
        success: true,
        hash: receipt.transactionHash,
        blockNumber: receipt.blockNumber
      });
    } catch (error) {
      console.error('Erreur lors du unbond:', error);
      
      // Logging détaillé de l'erreur
      if (error.code) console.error("Code d'erreur:", error.code);
      if (error.method) console.error("Méthode:", error.method);
      if (error.transaction) console.error("Transaction:", error.transaction);
      if (error.error && error.error.message) console.error("Message interne:", error.error.message);
      
      // Si c'est un timeout, on affiche un message spécifique
      if (error.message && error.message.includes("timeout")) {
        setError("La transaction a été envoyée mais attend d'être confirmée. Vérifiez votre portefeuille ou l'explorateur de blocs.");
        setTxResult({
          success: true,
          error: null,
          hash: tx.hash,
          pending: true
        });
      } else {
        setError(`Erreur: ${error.message}`);
        setTxResult({
          success: false,
          error: error.message
        });
      }
    } finally {
      setTxPending(false);
    }
  };

  // Récupérer les informations de staking
  const getStakingInfo = async () => {
    if (!account) return;
    
    setLoading(true);
    setError('');
    
    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      
      // Récupérer le solde du compte (jetons non stakés)
      const balance = await provider.getBalance(account);
      
      // MODIFICATION: Nous retirons l'appel au précompile avec 0x00000004 qui n'existe pas
      // Le précompile ne semble pas avoir de fonction pour récupérer le montant staké
      // Affichons seulement le solde disponible
      
      setStakingInfo({
        accountAddress: account,
        balanceEth: ethers.utils.formatEther(balance),
        stakedEth: "(Fonction non disponible)", // Indiquer que cette info n'est pas disponible
        totalEth: ethers.utils.formatEther(balance) // Total = solde disponible uniquement
      });
      
      console.log('Informations de balance récupérées:', {
        balance: ethers.utils.formatEther(balance)
      });
    } catch (error) {
      console.error('Erreur lors de la récupération des informations de balance:', error);
      setError(`Erreur: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };
  
  // Rafraîchir les informations après chaque transaction réussie
  useEffect(() => {
    if (isConnected) {
      getStakingInfo();
    }
  }, [isConnected, txResult]);

  return (
    <div className="min-h-screen bg-gray-100 py-8">
      <div className="max-w-3xl mx-auto px-4">
        <header className="bg-gray-800 text-white p-4 rounded-lg mb-6 text-center">
          <h1 className="text-2xl font-bold">Substrate Staking dApp</h1>
        </header>

        <main className="space-y-6">
          <div className="bg-white p-6 rounded-lg shadow-md">
            <h2 className="text-xl font-semibold mb-4 pb-2 border-b">Portefeuille</h2>
            {!isConnected ? (
              <ConnectButton onConnect={connectWallet} />
            ) : (
              <div className="bg-gray-50 p-4 rounded">
                <p>Connecté: <span className="font-mono font-medium break-all">{account}</span></p>
                
                {stakingInfo && (
                  <div className="mt-4 space-y-2">
                    <div className="flex justify-between border-b pb-2">
                      <span className="text-gray-600">Balance non stakée:</span>
                      <span className="font-medium">{stakingInfo.balanceEth} ETH</span>
                    </div>
                    <div className="flex justify-between border-b pb-2">
                      <span className="text-gray-600">Tokens stakés:</span>
                      <span className="font-medium text-green-600">{stakingInfo.stakedEth}</span>
                    </div>
                    <div className="flex justify-between font-semibold pt-1">
                      <span>Total:</span>
                      <span>{stakingInfo.totalEth} ETH</span>
                    </div>
                    
                    <button 
                      onClick={getStakingInfo}
                      disabled={loading}
                      className="mt-2 w-full py-1 px-4 bg-gray-200 hover:bg-gray-300 rounded text-sm"
                    >
                      {loading ? 'Chargement...' : 'Actualiser'}
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>

          {isConnected && (
            <>
              <div className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4 pb-2 border-b">Bond (Staker des tokens)</h2>
                <BondForm onBond={handleBond} isPending={txPending} />
              </div>

              <div className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4 pb-2 border-b">Unbond (Débloquer des tokens)</h2>
                <UnbondForm onUnbond={handleUnbond} isPending={txPending} />
              </div>
            </>
          )}

          {error && (
            <div className="bg-red-50 border-l-4 border-red-500 p-4 rounded-lg">
              <h3 className="text-red-800 font-medium">Erreur</h3>
              <p className="text-red-700">{error}</p>
            </div>
          )}

          {txResult && (
            <div className={`p-4 rounded-lg ${
              txResult.pending 
                ? 'bg-yellow-50 border-l-4 border-yellow-500' 
                : txResult.success 
                  ? 'bg-green-50 border-l-4 border-green-500' 
                  : 'bg-red-50 border-l-4 border-red-500'
            }`}>
              <h3 className={`font-medium ${
                txResult.pending 
                  ? 'text-yellow-800' 
                  : txResult.success 
                    ? 'text-green-800' 
                    : 'text-red-800'
              }`}>
                {txResult.pending 
                  ? 'Transaction en attente' 
                  : txResult.success 
                    ? 'Transaction réussie' 
                    : 'Échec de la transaction'
                }
              </h3>
              {txResult.pending ? (
                <div className="mt-2">
                  <p>Hash: <code className="bg-gray-100 px-1 py-0.5 rounded font-mono text-sm">{txResult.hash}</code></p>
                  <p className="text-yellow-700 mt-1">La transaction a été soumise mais attend d'être confirmée. Vérifiez votre portefeuille ou l'explorateur de blocs pour plus de détails.</p>
                </div>
              ) : txResult.success ? (
                <div className="mt-2">
                  <p>Hash: <code className="bg-gray-100 px-1 py-0.5 rounded font-mono text-sm">{txResult.hash}</code></p>
                  <p>Bloc: <code className="bg-gray-100 px-1 py-0.5 rounded font-mono text-sm">{txResult.blockNumber}</code></p>
                </div>
              ) : (
                <p className="text-red-700">{txResult.error}</p>
              )}
            </div>
          )}
        </main>

        <footer className="mt-12 text-center text-gray-500 text-sm">
          <p>Substrate Staking dApp - Précompile d'exemple</p>
        </footer>
      </div>
    </div>
  );
}

export default App;