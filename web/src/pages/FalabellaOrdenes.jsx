import { useEffect, useState } from 'react'
import { get } from '../api'

const money = (v) => {
  const n = Number(v)
  return isNaN(n) ? (v ?? '—') : new Intl.NumberFormat('es-CL', { style: 'currency', currency: 'CLP', maximumFractionDigits: 0 }).format(n)
}
const fecha = (s) => {
  const d = new Date(s)
  return isNaN(d) ? (s || '') : d.toLocaleString('es-CL', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' })
}
const estados = (o) => {
  let s = o?.Statuses?.Status ?? o?.Status
  s = Array.isArray(s) ? s : s ? [s] : []
  return [...new Set(s.map(String))].join(', ') || '—'
}

export default function FalabellaOrdenes({ cuenta }) {
  const [rows, setRows] = useState(null)
  const [error, setError] = useState(null)
  const [dias, setDias] = useState(30)
  const [cargando, setCargando] = useState(false)

  const cargar = () => {
    if (!cuenta) return
    setCargando(true)
    setError(null)
    get(`/falabella/ordenes?cuenta=${cuenta.id}&dias=${dias}`)
      .then((d) => {
        let o = d?.SuccessResponse?.Body?.Orders?.Order
        o = Array.isArray(o) ? o : o ? [o] : []
        setRows(o)
      })
      .catch((e) => setError(e.message))
      .finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta, dias]) // eslint-disable-line react-hooks/exhaustive-deps

  if (!cuenta) return null

  return (
    <div className="card stack" style={{ gap: 10 }}>
      <div className="row between" style={{ flexWrap: 'wrap', gap: 8 }}>
        <div className="strong">Ventas en Falabella</div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={dias} onChange={(e) => setDias(Number(e.target.value))}>
            <option value={7}>Últimos 7 días</option>
            <option value={30}>Últimos 30 días</option>
            <option value={90}>Últimos 90 días</option>
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : rows.length === 0 ? (
        <div className="muted sm">Sin ventas en Falabella en el período.</div>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%' }}>
            <thead>
              <tr><th>N° orden</th><th>Fecha</th><th>Cliente</th><th>Estado</th><th>Ítems</th><th style={{ textAlign: 'right' }}>Total</th></tr>
            </thead>
            <tbody>
              {rows.map((o) => (
                <tr key={o.OrderId || o.OrderNumber}>
                  <td className="strong">{o.OrderNumber || o.OrderId}</td>
                  <td className="sm">{fecha(o.CreatedAt)}</td>
                  <td className="sm">{[o.CustomerFirstName, o.CustomerLastName].filter(Boolean).join(' ') || '—'}</td>
                  <td><span className="tag">{estados(o)}</span></td>
                  <td className="sm">{o.ItemsCount ?? '—'}</td>
                  <td className="sm" style={{ textAlign: 'right' }}>{money(o.Price)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
