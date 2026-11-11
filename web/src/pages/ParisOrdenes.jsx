import { useEffect, useState } from 'react'
import { get } from '../api'

export default function ParisOrdenes({ cuenta }) {
  const [data, setData] = useState(null)
  const [error, setError] = useState(null)

  useEffect(() => {
    setData(null); setError(null)
    get(`/paris/cuentas/${cuenta.id}/ordenes`).then(setData).catch((e) => setError(e.message))
  }, [cuenta.id])

  const ordenes = Array.isArray(data) ? data : (data?.data || [])

  if (error) return <div className="alert alert-bad">{error}</div>
  if (data == null) return <div className="muted">Cargando órdenes…</div>
  if (ordenes.length === 0) return <div className="muted sm">Sin órdenes por ahora.</div>

  return (
    <div className="table-wrap">
      <table className="table">
        <thead><tr><th>N° Orden</th><th>Cliente</th><th>Fecha</th><th>Estado</th></tr></thead>
        <tbody>
          {ordenes.map((o, i) => (
            <tr key={o.id || o.originOrderNumber || i}>
              <td className="num">{o.originOrderNumber || o.id || '—'}</td>
              <td>{o.customer?.name || '—'}</td>
              <td className="sm muted">{(o.originOrderDate || o.createdAt || '').slice(0, 10)}</td>
              <td>{o.status || o.itemStatus || '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
