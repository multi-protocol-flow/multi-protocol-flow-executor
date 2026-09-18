import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { setupGlobalErrorHandlers } from './lib/globalErrorHandler'
import { initLayoutTransitionObserver } from './lib/layoutTransition'

setupGlobalErrorHandlers();
initLayoutTransitionObserver();
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
