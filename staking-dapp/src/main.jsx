import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.jsx'
import TransferTest from './TransferTest.jsx'
import './index.css'

// Décommentez l'une des deux lignes suivantes pour choisir l'interface à afficher
ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <App />
    {/* <TransferTest /> */}
  </React.StrictMode>,
)
