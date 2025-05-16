import { useState } from 'react';

const BondForm = ({ onBond, isPending }) => {
  const [amount, setAmount] = useState('');
  const [payeeType, setPayeeType] = useState(0); // Default: Staked

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!amount) return;
    onBond(amount, parseInt(payeeType));
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label htmlFor="amount" className="block text-sm font-medium text-gray-700 mb-1">
          Montant (ETH)
        </label>
        <input
          type="number"
          id="amount"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          min="0"
          step="0.001"
          placeholder="0.0"
          required
          className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>
      
      <div>
        <label htmlFor="payeeType" className="block text-sm font-medium text-gray-700 mb-1">
          Destination des récompenses
        </label>
        <select
          id="payeeType"
          value={payeeType}
          onChange={(e) => setPayeeType(e.target.value)}
          className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="0">Staked (réinvestir)</option>
          <option value="1">Stash (compte principal)</option>
          <option value="2">Controller (compte contrôleur)</option>
        </select>
      </div>
      
      <button
        type="submit"
        disabled={isPending || !amount}
        className={`w-full py-2 px-4 rounded-md font-medium ${
          isPending
            ? 'bg-gray-400 text-gray-700 cursor-not-allowed'
            : 'bg-green-600 hover:bg-green-700 text-white'
        }`}
      >
        {isPending ? 'Transaction en cours...' : 'Staker'}
      </button>
    </form>
  );
};

export default BondForm; 