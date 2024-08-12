import './polyfills'
import { configure } from 'mobx'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'

if (process.env.NODE_ENV === 'development') {
  configure({
    computedRequiresReaction: true,
    observableRequiresReaction: true,
    reactionRequiresObservable: true,
  })
}

createRoot(document.querySelector('#root')).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
