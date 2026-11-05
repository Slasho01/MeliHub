import { useEffect, useState } from 'react'
import { get, post } from '../api'

const ICONO = { quiebre: '⚠️', sync_error: '🔌', info: 'ℹ️' }

export default function Notificaciones() {
  const [data, setData] = useState(null)
  const [error, setError] = useState(null)

  const cargar = () => get('/notificaciones').then(setData).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const marcarTodas = async () => {
    try { await post('/notificaciones/leer', {}); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
        <div><h1>Notificaciones</h1>{data && <span className="muted sm">{data.no_leidas} sin leer</span>}</div>
        {data?.no_leidas > 0 && <button className="btn btn-sm" onClick={marcarTodas}>Marcar todas como leídas</button>}
      </div>
      {error && <div className="alert alert-bad">{error}</div>}
      <div className="card" style={{ padding: 0 }}>
        {!data ? <div className="empty">Cargando…</div> : data.items.length === 0 ? (
          <div className="empty">Sin notificaciones. Aquí verás quiebres de stock y errores de sincronización.</div>
        ) : (
          <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
            {data.items.map((n) => (
              <li key={n.id} style={{ padding: '12px 14px', borderTop: '1px solid var(--line)', opacity: n.leida ? 0.55 : 1 }}>
                <div><b>{ICONO[n.tipo] || '•'} {n.titulo}</b> {!n.leida && <span className="badge" style={{ color: 'var(--accent)' }}>nuevo</span>}</div>
                {n.cuerpo && <div className="muted sm">{n.cuerpo}</div>}
                <div className="muted sm">{new Date(n.created_at + 'Z').toLocaleString('es-CL')}</div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
