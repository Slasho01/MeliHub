import { useEffect, useState } from 'react'
import { get, post } from '../api'

const asList = (v) => v?.data || v?.results || (Array.isArray(v) ? v : [])
const LIMIT = 20

export default function ParisOrdenes({ cuenta }) {
  const [subs, setSubs] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState('')
  const [off, setOff] = useState(0)
  const [more, setMore] = useState(false)
  const [razones, setRazones] = useState([])
  const [motivo, setMotivo] = useState({}) // { [subOrderNumber]: reasonId }

  const cargar = (reset = true) => {
    const start = reset ? 0 : off
    if (reset) { setSubs(null); setError(null) }
    get(`/paris/cuentas/${cuenta.id}/sub-ordenes?query=${encodeURIComponent(`limit=${LIMIT}&offset=${start}`)}`)
      .then((d) => { const l = asList(d); setSubs((p) => (reset ? l : [...(p || []), ...l])); setOff(start + l.length); setMore(l.length === LIMIT) })
      .catch((e) => setError(e.message))
  }
  useEffect(() => { cargar(true); get(`/paris/cuentas/${cuenta.id}/razones-cancelacion`).then((d) => setRazones(asList(d))).catch(() => {}) }, [cuenta.id]) // eslint-disable-line react-hooks/exhaustive-deps

  const crearEnvio = async (s) => {
    if (!window.confirm(`¿Crear el envío de la sub-orden ${s.subOrderNumber}?`)) return
    setBusy(s.subOrderNumber); setError(null); setMsg(null)
    try {
      const items = (s.items || []).map((it) => ({ sku: it.sku || it.sellerSku, quantity: it.quantity || 1, description: it.name || '' }))
      await post(`/paris/cuentas/${cuenta.id}/sub-ordenes/${s.subOrderNumber}/envio`, { shipments: [{ packages: 1, items }] })
      setMsg(`Envío creado para la sub-orden ${s.subOrderNumber}.`)
      cargar(true)
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const verEtiqueta = async (s) => {
    setError(null)
    try {
      const r = await get(`/paris/cuentas/${cuenta.id}/etiqueta/${s.labelId}`)
      const url = r?.url || r?.label || r?.data?.url || (typeof r === 'string' ? r : null)
      if (url) window.open(url, '_blank')
      else setMsg('Etiqueta solicitada — si no abre, descárgala desde el Seller Center.')
    } catch (e) { setError(e.message) }
  }

  const cancelar = async (s) => {
    const reasonId = Number(motivo[s.subOrderNumber])
    if (!reasonId) { setError('Elige un motivo de cancelación.'); return }
    if (!window.confirm(`¿Cancelar la sub-orden ${s.subOrderNumber}? Esta acción no se puede deshacer.`)) return
    setBusy(s.subOrderNumber); setError(null); setMsg(null)
    try {
      const skus = (s.items || []).map((it) => ({ sku: it.sku || it.sellerSku, quantity: it.quantity || 1 }))
      const shipmentId = s.shipmentId || s.shipments?.[0]?.shipmentId
      const shipments = [shipmentId ? { shipmentId, skus } : { skus }]
      await post(`/paris/cuentas/${cuenta.id}/sub-ordenes/${s.subOrderNumber}/cancelar`, { status: 'CANCELED', cancellationReasonId: reasonId, shipments })
      setMsg(`Sub-orden ${s.subOrderNumber} cancelada.`)
      cargar(true)
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (error && !subs) return <div className="alert alert-bad">{error}</div>
  if (subs == null) return <div className="muted">Cargando órdenes…</div>

  return (
    <div className="stack" style={{ gap: 10 }}>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      <div className="row between">
        <span className="sm muted">{subs.length} sub-orden(es)</span>
        <button className="btn btn-sm" onClick={() => cargar(true)}>Actualizar</button>
      </div>
      {subs.length === 0 ? <div className="muted sm">Sin órdenes por ahora.</div> : subs.map((s, i) => (
        <div key={s.subOrderNumber || i} className="card stack" style={{ gap: 6 }}>
          <div className="row between" style={{ flexWrap: 'wrap', gap: 6 }}>
            <div><span className="strong">#{s.subOrderNumber}</span> <span className="muted sm">· {s.order?.customer?.name || '—'}</span></div>
            <span className="badge">{s.status?.name || s.status || '—'}</span>
          </div>
          <div className="sm muted">{(s.order?.originOrderDate || s.createdAt || '').slice(0, 10)} · {s.carrier || 'sin courier'}{s.labelId ? ` · etiqueta ${String(s.labelId).slice(0, 8)}…` : ''}</div>
          <div className="stack" style={{ gap: 2 }}>
            {(s.items || []).map((it, j) => (
              <div key={j} className="sm">• {it.name || it.sku} <span className="muted">({it.sellerSku || it.sku})</span></div>
            ))}
          </div>
          <div className="row" style={{ gap: 6, flexWrap: 'wrap', alignItems: 'center' }}>
            <button className="btn btn-sm btn-dark" disabled={busy === s.subOrderNumber} onClick={() => crearEnvio(s)}>
              {busy === s.subOrderNumber ? 'Creando…' : 'Crear envío'}
            </button>
            {s.labelId && <button className="btn btn-sm" onClick={() => verEtiqueta(s)}>Etiqueta</button>}
            <span style={{ flex: 1 }} />
            <select className="input" style={{ width: 'auto', maxWidth: 200 }} value={motivo[s.subOrderNumber] || ''} onChange={(e) => setMotivo((m) => ({ ...m, [s.subOrderNumber]: e.target.value }))}>
              <option value="">Motivo…</option>
              {razones.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
            </select>
            <button className="btn btn-sm btn-danger" disabled={busy === s.subOrderNumber || !motivo[s.subOrderNumber]} onClick={() => cancelar(s)}>Cancelar</button>
          </div>
        </div>
      ))}
      {more && <button className="btn btn-sm" style={{ alignSelf: 'center' }} onClick={() => cargar(false)}>Cargar más</button>}
    </div>
  )
}
