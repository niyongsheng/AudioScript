import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@/shared/theme.css'
import { initLocale } from '@/shared/locale'
import { App } from './App'

void initLocale()

const root = document.getElementById('root')
if (!root) throw new Error('#root 未找到')

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
