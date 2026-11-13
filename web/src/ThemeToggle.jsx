import { useState } from 'react'

function aplicar(t) { try { document.documentElement.dataset.theme = t; localStorage.setItem('melihub_tema', t) } catch { /* ignore */ } }

export default function ThemeToggle({ className = 'btn btn-sm' }) {
  const [tema, setTema] = useState(() => { try { return localStorage.getItem('melihub_tema') || 'dark' } catch { return 'dark' } })
  const toggle = () => { const n = tema === 'dark' ? 'light' : 'dark'; setTema(n); aplicar(n) }
  return (
    <button className={className} title={tema === 'dark' ? 'Cambiar a claro' : 'Cambiar a oscuro'} aria-label="Cambiar tema" onClick={toggle}>
      {tema === 'dark' ? '☀️' : '🌙'}
    </button>
  )
}
