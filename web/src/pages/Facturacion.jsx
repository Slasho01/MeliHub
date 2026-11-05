import { useEffect, useState } from 'react'
import { get, put } from '../api'

const INI = { tipo_documento: 'boleta', rut: '', razon_social: '', giro: '', direccion: '', comuna: '', ciudad: '', telefono: '', pais: 'Chile' }

export default function Facturacion() {
  const [f, setF] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)

  useEffect(() => {
    get('/facturacion').then((r) => setF({ ...INI, ...(r.facturacion || {}) })).catch((e) => setError(e.message))
  }, [])

  const set = (k) => (e) => setF({ ...f, [k]: e.target.value })
  const guardar = async () => {
    setError(null); setMsg(null)
    try { const g = await put('/facturacion', f); setF({ ...INI, ...g }); setMsg('Datos de facturación guardados.') }
    catch (e) { setError(e.message) }
  }

  if (!f) return <div className="empty">Cargando…</div>
  return (
    <div className="stack" style={{ maxWidth: 520 }}>
      <div><h1>Facturación</h1><p className="muted sm" style={{ margin: 0 }}>Datos para emitir tu boleta o factura. Se usan al suscribirte a un plan de pago.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="card stack">
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Tipo de documento</span>
          <select className="input" value={f.tipo_documento} onChange={set('tipo_documento')}>
            <option value="boleta">Boleta</option>
            <option value="factura">Factura</option>
          </select></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">RUT</span>
          <input className="input" placeholder="12.345.678-9" value={f.rut} onChange={set('rut')} /></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">{f.tipo_documento === 'factura' ? 'Razón social' : 'Nombre'}</span>
          <input className="input" value={f.razon_social} onChange={set('razon_social')} /></label>
        {f.tipo_documento === 'factura' && (
          <label className="stack" style={{ gap: 2 }}><span className="sm muted">Giro</span>
            <input className="input" value={f.giro || ''} onChange={set('giro')} /></label>
        )}
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Dirección</span>
          <input className="input" value={f.direccion} onChange={set('direccion')} /></label>
        <div className="row" style={{ gap: 8 }}>
          <label className="stack" style={{ gap: 2, flex: 1 }}><span className="sm muted">Comuna</span>
            <input className="input" value={f.comuna} onChange={set('comuna')} /></label>
          <label className="stack" style={{ gap: 2, flex: 1 }}><span className="sm muted">Ciudad</span>
            <input className="input" value={f.ciudad || ''} onChange={set('ciudad')} /></label>
        </div>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Teléfono (opcional)</span>
          <input className="input" value={f.telefono || ''} onChange={set('telefono')} /></label>
        <div><button className="btn btn-accent" onClick={guardar}>Guardar</button></div>
      </div>
    </div>
  )
}
