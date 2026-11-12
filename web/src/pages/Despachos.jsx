import { useEffect, useState } from 'react'
import { NavLink } from 'react-router-dom'
import { get, descargarArchivo } from '../api'

const asArray = (x) => (Array.isArray(x) ? x : x ? [x] : [])
const fechaCorta = (iso) => { if (!iso) return ''; const d = new Date(iso); return isNaN(d) ? '' : d.toLocaleString('es-CL', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }) }

const CANAL = { ML: ['Mercado Libre', '#ffe600'], Falabella: ['Falabella', '#78be20'], Paris: ['Paris', '#2a6f97'] }

// ML: pendiente de despacho (hay etiqueta por imprimir / preparando).
const IMPRIMIBLE = ['ready_to_print', 'printed', 'ready_to_ship', 'invoice_pending']
const pendienteML = (estado, sub) => {
  if (estado === 'pending' || estado === 'handling') return true
  if (estado === 'ready_to_ship') return sub ? IMPRIMIBLE.includes(sub) : true
  return false
}
const estadoMl = (e) => ({ shipped: 'Enviado', delivered: 'Entregado', not_delivered: 'No entregado', cancelled: 'Cancelado', pending: 'Preparando', handling: 'Preparando', ready_to_ship: 'Por despachar' }[e] || e || '—')

export default function Despachos() {
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [cargando, setCargando] = useState(false)
  const [filtro, setFiltro] = useState('')

  const cargar = async () => {
    setCargando(true); setError(''); setMsg('')
    const out = []
    // --- Mercado Libre (agrega todas las cuentas) ---
    try {
      const env = await get('/ventas/envios')
      for (const o of asArray(env)) {
        out.push({
          key: `ml-${o.id}`, canal: 'ML', id: o.id, ref: o.ml_order_id, cuenta: o.cuenta,
          comprador: o.comprador || '—', fecha: o.fecha, estado: estadoMl(o.envio_estado || o.estado),
          tracking: o.tracking ? `${o.tracking_metodo ? o.tracking_metodo + ' · ' : ''}${o.tracking}` : '',
          pendiente: pendienteML(o.envio_estado, o.envio_substatus),
        })
      }
    } catch (e) { setError(e.message) }
    // --- Falabella (por cuenta) ---
    try {
      const cuentas = await get('/falabella/cuentas').catch(() => [])
      for (const c of asArray(cuentas)) {
        const d = await get(`/falabella/ordenes?cuenta=${c.id}&dias=30`).catch(() => null)
        for (const o of asArray(d?.SuccessResponse?.Body?.Orders?.Order)) {
          const st = asArray(o?.Statuses?.Status ?? o?.Status).join(', ')
          out.push({
            key: `fa-${o.OrderId}`, canal: 'Falabella', cuentaId: c.id, orderId: o.OrderId,
            ref: o.OrderNumber || o.OrderId, cuenta: c.nickname || c.user_id,
            comprador: [o.CustomerFirstName, o.CustomerLastName].filter(Boolean).join(' ') || '—',
            fecha: o.CreatedAt, estado: st || '—', tracking: '',
            pendiente: !/ship|deliver|cancel/i.test(st),
          })
        }
      }
    } catch { /* best-effort */ }
    // --- Paris (por cuenta) ---
    try {
      const cuentas = await get('/paris/cuentas').catch(() => [])
      for (const c of asArray(cuentas)) {
        const d = await get(`/paris/cuentas/${c.id}/sub-ordenes`).catch(() => null)
        const subs = d?.data || d?.results || (Array.isArray(d) ? d : [])
        for (const s of asArray(subs)) {
          const est = s.status?.name || s.status || '—'
          out.push({
            key: `pa-${s.subOrderNumber}`, canal: 'Paris', cuentaId: c.id, labelId: s.labelId,
            ref: s.subOrderNumber, cuenta: c.nombre || 'Paris',
            comprador: s.order?.customer?.name || '—', fecha: s.createdAt || s.order?.originOrderDate,
            estado: est, tracking: s.carrier || '', pendiente: !/ship|deliver|cancel/i.test(String(est)),
          })
        }
      }
    } catch { /* best-effort */ }

    out.sort((a, b) => new Date(b.fecha || 0) - new Date(a.fecha || 0))
    setRows(out); setCargando(false)
  }
  useEffect(() => { cargar() }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const etiquetaML = (r) => descargarArchivo(`/ventas/${r.id}/etiqueta`, `etiqueta-${r.ref}.pdf`).catch((e) => setError(e.message))
  const etiquetaFala = (r) => descargarArchivo(`/falabella/ordenes/documento?cuenta=${r.cuentaId}&order_id=${r.orderId}&tipo=shippingLabel`, `etiqueta-${r.ref}.pdf`).catch((e) => setError(e.message))
  const etiquetaParis = async (r) => {
    try { const x = await get(`/paris/cuentas/${r.cuentaId}/etiqueta/${r.labelId}`); const url = x?.url || x?.label || (typeof x === 'string' ? x : null); if (url) window.open(url, '_blank'); else setMsg('Etiqueta solicitada — revísala en el Seller Center.') }
    catch (e) { setError(e.message) }
  }

  const lista = (rows || []).filter((r) => !filtro || r.canal === filtro)
  const pendientes = (c) => (rows || []).filter((r) => r.canal === c && r.pendiente).length

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div><h1 style={{ margin: 0 }}>Despachos</h1><p className="muted sm" style={{ margin: 0 }}>Todos tus despachos pendientes de Mercado Libre, Falabella y Paris en un solo lugar.</p></div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={filtro} onChange={(e) => setFiltro(e.target.value)}>
            <option value="">Todos los canales</option>
            <option value="ML">Mercado Libre</option>
            <option value="Falabella">Falabella</option>
            <option value="Paris">Paris</option>
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="row" style={{ gap: 10, flexWrap: 'wrap', alignItems: 'stretch' }}>
        {['ML', 'Falabella', 'Paris'].map((c) => (
          <div key={c} className="card" style={{ flex: '1 1 160px', borderLeft: `4px solid ${CANAL[c][1]}` }}>
            <div className="muted sm">{CANAL[c][0]}</div>
            <div style={{ fontSize: 24, fontWeight: 800 }}>{pendientes(c)}</div>
            <div className="muted xs">pendientes de despacho</div>
          </div>
        ))}
      </div>

      {rows == null ? (
        <div className="muted">Consultando despachos de todos los canales…</div>
      ) : lista.length === 0 ? (
        <div className="empty">Sin despachos pendientes. 🎉</div>
      ) : (
        <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
          <table style={{ minWidth: 820 }}>
            <thead><tr><th>Canal</th><th>Referencia</th><th>Comprador</th><th>Fecha</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {lista.map((r) => (
                <tr key={r.key} style={r.pendiente ? undefined : { opacity: 0.6 }}>
                  <td><span className="badge" style={{ borderLeft: `3px solid ${CANAL[r.canal][1]}`, paddingLeft: 8, background: 'var(--surface-2)' }}>{CANAL[r.canal][0]}</span></td>
                  <td className="sm"><b>{r.ref}</b><div className="muted xs">{r.cuenta}</div></td>
                  <td className="sm">{r.comprador}</td>
                  <td className="sm muted">{fechaCorta(r.fecha)}</td>
                  <td className="sm">{r.estado}{r.tracking && <div className="muted xs">📦 {r.tracking}</div>}</td>
                  <td className="r" style={{ whiteSpace: 'nowrap' }}>
                    {r.canal === 'ML' && <button className="btn btn-sm" onClick={() => etiquetaML(r)}>⬇ Etiqueta</button>}
                    {r.canal === 'Falabella' && <><button className="btn btn-sm" onClick={() => etiquetaFala(r)}>⬇ Etiqueta</button>{' '}<NavLink to="/falabella" className="btn btn-sm">Gestionar</NavLink></>}
                    {r.canal === 'Paris' && <>{r.labelId && <button className="btn btn-sm" onClick={() => etiquetaParis(r)}>Etiqueta</button>}{' '}<NavLink to="/paris" className="btn btn-sm">Gestionar</NavLink></>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="xs muted" style={{ padding: '10px 14px', margin: 0 }}>Para marcar "listo" (Falabella) o crear el envío (Paris) usa "Gestionar". Las etiquetas se descargan aquí.</p>
        </div>
      )}
    </div>
  )
}
