import { useEffect, useState } from 'react'
import { get, post } from '../api'

const asList = (v) => v?.data || v?.results || (Array.isArray(v) ? v : [])

export default function ParisOrdenes({ cuenta }) {
  const [subs, setSubs] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState('')

  const cargar = () => {
    setSubs(null); setError(null)
    get(`/paris/cuentas/${cuenta.id}/sub-ordenes`).then((d) => setSubs(asList(d))).catch((e) => setError(e.message))
  }
  useEffect(cargar, [cuenta.id])

  const crearEnvio = async (s) => {
    if (!window.confirm(`¿Crear el envío de la sub-orden ${s.subOrderNumber}?`)) return
    setBusy(s.subOrderNumber); setError(null); setMsg(null)
    try {
      const items = (s.items || []).map((it) => ({ sku: it.sku || it.sellerSku, quantity: 1, description: it.name || '' }))
      await post(`/paris/cuentas/${cuenta.id}/sub-ordenes/${s.subOrderNumber}/envio`, { shipments: [{ packages: 1, items }] })
      setMsg(`Envío creado para la sub-orden ${s.subOrderNumber}.`)
      cargar()
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

  if (error && !subs) return <div className="alert alert-bad">{error}</div>
  if (subs == null) return <div className="muted">Cargando órdenes…</div>

  return (
    <div className="stack" style={{ gap: 10 }}>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      <div className="row between">
        <span className="sm muted">{subs.length} sub-orden(es)</span>
        <button className="btn btn-sm" onClick={cargar}>Actualizar</button>
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
          <div className="row" style={{ gap: 6 }}>
            <button className="btn btn-sm btn-dark" disabled={busy === s.subOrderNumber} onClick={() => crearEnvio(s)}>
              {busy === s.subOrderNumber ? 'Creando…' : 'Crear envío'}
            </button>
            {s.labelId && <button className="btn btn-sm" onClick={() => verEtiqueta(s)}>Etiqueta</button>}
          </div>
        </div>
      ))}
    </div>
  )
}
