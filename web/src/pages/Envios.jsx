import { useEffect, useState } from 'react'
import { get, descargarArchivo } from '../api'

export default function Envios() {
  const [dia, setDia] = useState('') // vacío = últimos 15 días
  const [rows, setRows] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)

  const cargar = () => {
    setRows(null); setError(null)
    get(`/ventas/envios${dia ? `?dia=${dia}` : ''}`).then(setRows).catch((e) => setError(e.message))
  }
  useEffect(cargar, [dia]) // eslint-disable-line react-hooks/exhaustive-deps

  const qs = dia ? `?dia=${dia}` : ''
  const descargarDia = async () => {
    setBusy(true); setError(null); setMsg(null)
    try { await descargarArchivo(`/ventas/etiquetas${qs}`, `etiquetas-${dia || 'recientes'}.pdf`); setMsg('Descarga lista.') }
    catch (e) { setError(e.message) } finally { setBusy(false) }
  }
  const descargarUna = async (o) => {
    setError(null); setMsg(null)
    try { await descargarArchivo(`/ventas/${o.id}/etiqueta`, `etiqueta-${o.ml_order_id}.pdf`) }
    catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div><h1 style={{ margin: 0 }}>Envíos</h1>
          <p className="muted sm" style={{ margin: 0 }}>Etiquetas de Flex y Centro de envío (colecta) de todas tus cuentas, en una sola descarga.</p></div>
        <div className="row" style={{ gap: 8 }}>
          <input className="input" type="date" value={dia} onChange={(e) => setDia(e.target.value)} title="Filtra por día (vacío = últimos 15 días)" />
          {dia && <button className="btn btn-sm" onClick={() => setDia('')}>Ver recientes</button>}
          <button className="btn btn-accent" disabled={busy || !rows?.length} onClick={descargarDia}>
            {busy ? 'Preparando…' : '⬇ Descargar etiquetas'}
          </button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      <p className="muted sm" style={{ marginTop: -4 }}>{dia ? `Mostrando el ${dia}.` : 'Mostrando los últimos 15 días.'} Las ventas FULL no aparecen (las despacha Mercado Libre).</p>

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!rows ? <div className="empty">Cargando…</div> : rows.length === 0 ? (
          <div className="empty">No hay ventas con envío imprimible.<br /><span className="sm">Si acabas de vender, ve a <b>Ventas → Sincronizar ventas</b> para traerlas.</span></div>
        ) : (
          <table style={{ minWidth: 760 }}>
            <thead><tr><th>Hora</th><th>Cuenta</th><th>Comprador</th><th>Productos</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {rows.map((o) => (
                <tr key={o.id}>
                  <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>{new Date(o.fecha + 'Z').toLocaleTimeString('es-CL', { hour: '2-digit', minute: '2-digit' })}</td>
                  <td><b>{o.cuenta}</b></td>
                  <td className="muted sm">{o.comprador || '—'}</td>
                  <td className="sm" style={{ maxWidth: 320 }}>{o.productos}</td>
                  <td><span className="badge">{o.estado}</span></td>
                  <td style={{ textAlign: 'right' }}><button className="btn btn-sm" onClick={() => descargarUna(o)}>Etiqueta</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
