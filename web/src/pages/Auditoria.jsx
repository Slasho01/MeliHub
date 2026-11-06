import { useEffect, useState } from 'react'
import { get } from '../api'

const ACCION = {
  precio_ml: 'Cambió precio en ML',
  estado_ml: 'Pausó/activó publicación',
  equipo_invitar: 'Invitó a un miembro',
  equipo_eliminar: 'Eliminó un miembro',
  equipo_rol: 'Cambió rol de un miembro',
  apikey_crear: 'Creó una API key',
  apikey_revocar: 'Revocó una API key',
}

export default function Auditoria() {
  const [items, setItems] = useState(null)
  const [error, setError] = useState(null)
  useEffect(() => { get('/auditoria').then(setItems).catch((e) => setError(e.message)) }, [])

  if (error) return <div className="alert alert-bad">{error}</div>
  if (!items) return <div className="empty">Cargando…</div>
  return (
    <div className="stack">
      <div><h1>Actividad</h1><p className="muted sm" style={{ margin: 0 }}>Acciones recientes de tu cuenta y tu equipo.</p></div>
      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {items.length === 0 ? <div className="empty">Sin actividad registrada todavía.</div> : (
          <table style={{ minWidth: 640 }}>
            <thead><tr><th>Fecha</th><th>Quién</th><th>Acción</th><th>Detalle</th></tr></thead>
            <tbody>{items.map((a) => (
              <tr key={a.id}>
                <td className="muted sm">{new Date(a.created_at + 'Z').toLocaleString('es-CL')}</td>
                <td>{a.actor || '—'}</td>
                <td>{ACCION[a.accion] || a.accion}</td>
                <td className="muted sm">{a.detalle || ''}</td>
              </tr>
            ))}</tbody>
          </table>
        )}
      </div>
    </div>
  )
}
