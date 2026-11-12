import { useEffect, useState } from 'react'
import { NavLink } from 'react-router-dom'
import { get } from '../api'

const PASOS = [
  { key: 'ml_conectado', label: 'Conecta tu cuenta de Mercado Libre', to: '/', cta: 'Conectar' },
  { key: 'productos_sincronizados', label: 'Sincroniza tus publicaciones', to: '/inventario', cta: 'Inventario' },
  { key: 'costos_cargados', label: 'Carga costos para ver tu ganancia real', to: '/inventario', cta: 'Cargar costos' },
  { key: 'otro_canal', label: 'Conecta otro canal (Falabella / Paris)', to: '/falabella', cta: 'Conectar' },
]

export default function Onboarding() {
  const [st, setSt] = useState(null)
  const [oculto, setOculto] = useState(() => { try { return localStorage.getItem('melihub_onboarding_done') === '1' } catch { return false } })

  useEffect(() => { get('/onboarding').then(setSt).catch(() => {}) }, [])

  if (!st || oculto) return null
  const hechos = PASOS.filter((p) => st[p.key]).length
  if (hechos === PASOS.length) { try { localStorage.setItem('melihub_onboarding_done', '1') } catch { /* ignore */ } return null }

  return (
    <div className="card" style={{ borderLeft: '4px solid var(--accent)' }}>
      <div className="row between" style={{ alignItems: 'center' }}>
        <b>🚀 Puesta en marcha · {hechos}/{PASOS.length}</b>
        <button className="link-btn" onClick={() => setOculto(true)}>ocultar</button>
      </div>
      <div className="stack" style={{ gap: 6, marginTop: 10 }}>
        {PASOS.map((p) => (
          <div key={p.key} className="row between" style={{ gap: 10, flexWrap: 'wrap' }}>
            <span className="sm" style={{ color: st[p.key] ? 'var(--muted)' : undefined }}>{st[p.key] ? '✅' : '⬜'} {p.label}</span>
            {!st[p.key] && <NavLink to={p.to} className="btn btn-sm">{p.cta}</NavLink>}
          </div>
        ))}
      </div>
    </div>
  )
}
