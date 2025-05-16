const ConnectButton = ({ onConnect }) => {
  return (
    <button 
      onClick={onConnect}
      className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded transition-colors"
    >
      Connecter le portefeuille
    </button>
  );
};

export default ConnectButton; 