import { useEffect, useState } from 'react'
import { get } from '../api'

const fecha = (iso) => { if (!iso) return ''; const d = new Date(iso); return isNaN(d) ? '' : d.toLocaleDateString('es-CL', { day: '2-digit', month: 'short', year: '2-digit' }) }

const tipoTxt = (t) => ({
  DEAL: 'Oferta', MARKETPLACE_CAMPAIGN: 'Campaña ML', PRICE_DISCOUNT: 'Descuento', SELLER_CAMPAIGN: 'Campaña propia',
  DOD: 'Oferta del día', LIGHTNING: 'Oferta relámpago', PRE_NEGOTIATED: 'Pre-negociada', SMART: 'Descuento inteligente',
}[t] || t || 'Promoción')

const estadoInfo = (s) => {
  const x = String(s || '').toLowerCase()
  if (['started', 'active'].includes(x)) return ['Activa', 'var(--ok)']
  if (['pending', 'candidate'].includes(x)) return ['Disponible', 'var(--warn)']
  if (['finished', 'closed'].includes(x)) return ['Finalizada', 'var(--muted)']
  return [s || '—', 'var(--muted)']
}

export default function Promociones() {
  const [cuenta, setCuenta] = useState('')
  const [cuentas, setCuentas] = useState([])
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])
  const cargar = () => {
    setCargando(true); setError('')
    const qs = cuenta ? `?cuenta=${cuenta}` : ''
    get(`/ml/promociones${qs}`).then(setRows).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Promociones y campañas</h1>
          <p className="muted sm" style={{ margin: 0 }}>Ofertas, campañas y descuentos de Mercado Libre en los que puedes participar.</p>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)}>
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : rows.length === 0 ? (
        <div className="empty">No hay promociones disponibles por ahora.</div>
      ) : (
        <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
          <table style={{ minWidth: 760 }}>
            <thead><tr><th>Promoción</th><th>Tipo</th><th>Estado</th><th>Vigencia</th><th>Cuenta</th></tr></thead>
            <tbody>
              {rows.map((p, i) => {
                const [et, color] = estadoInfo(p.estado)
                return (
                  <tr key={`${p.cuenta_id}-${p.id || i}`}>
                    <td><b>{p.nombre || tipoTxt(p.tipo)}</b><div className="muted xs">{p.id}</div></td>
                    <td className="sm">{tipoTxt(p.tipo)}</td>
                    <td><span className="badge" style={{ color }}>{et}</span></td>
                    <td className="sm muted">{p.inicio ? `${fecha(p.inicio)} – ${fecha(p.fin)}` : '—'}</td>
                    <td className="sm muted">{p.cuenta || '—'}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
          <p className="xs muted" style={{ padding: '10px 14px', margin: 0 }}>Para <b>sumar productos</b> a una promoción, entra a la sección de Promociones de Mercado Libre (gestión avanzada por ahora se hace allí).</p>
        </div>
      )}
    </div>
  )
}
