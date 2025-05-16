import { useState } from 'react';

const TransferForm = ({ onTransfer, isPending }) => {
  const [recipient, setRecipient] = useState('');
  const [amount, setAmount] = useState('');

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!recipient || !amount) return;
    onTransfer(recipient, amount);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label htmlFor="recipient" className="block text-sm font-medium text-gray-700 mb-1">
          Adresse du destinataire
        </label>
        <input
          type="text"
          id="recipient"
          value={recipient}
          onChange={(e) => setRecipient(e.target.value)}
          placeholder="0x..."
          required
          className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>
      
      <div>
        <label htmlFor="transferAmount" className="block text-sm font-medium text-gray-700 mb-1">
          Montant (ETH)
        </label>
        <input
          type="number"
          id="transferAmount"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          min="0"
          step="0.001"
          placeholder="0.0"
          required
          className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>
      
      <button
        type="submit"
        disabled={isPending || !amount || !recipient}
        className={`w-full py-2 px-4 rounded-md font-medium ${
          isPending
            ? 'bg-gray-400 text-gray-700 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700 text-white'
        }`}
      >
        {isPending ? 'Transaction en cours...' : 'Transférer'}
      </button>
    </form>
  );
};

export default TransferForm; 