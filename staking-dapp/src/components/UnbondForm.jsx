import { useState } from 'react';

const UnbondForm = ({ onUnbond, isPending }) => {
  const [amount, setAmount] = useState('');

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!amount) return;
    onUnbond(amount);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label htmlFor="unbondAmount" className="block text-sm font-medium text-gray-700 mb-1">
          Montant (ETH)
        </label>
        <input
          type="number"
          id="unbondAmount"
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
        disabled={isPending || !amount}
        className={`w-full py-2 px-4 rounded-md font-medium ${
          isPending
            ? 'bg-gray-400 text-gray-700 cursor-not-allowed'
            : 'bg-orange-600 hover:bg-orange-700 text-white'
        }`}
      >
        {isPending ? 'Transaction en cours...' : 'Débloquer'}
      </button>
    </form>
  );
};

export default UnbondForm; 