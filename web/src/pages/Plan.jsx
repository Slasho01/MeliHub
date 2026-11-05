import { useEffect, useState } from 'react'
import { get, money, post } from '../api'

const lim = (v) => (v < 0 ? '∞' : v)
const precio = (v) => (v === 0 ? 'Gratis' : `${money(v)}/mes`)

function Barra({ label, used, max }) {
  const pct = max > 0 ? Math.min(100, (used / max) * 100) : 0
  const alto = max > 0 && used >= max
  return (
    <div style={{ marginBottom: 10 }}>
      <div className="row" style={{ justifyContent: 'space-between', fontSize: 13 }}>
        <span>{label}</span>
        <span className="muted">{used} / {lim(max)}</span>
      </div>
      {max > 0 && (
        <div style={{ height: 6, background: 'var(--line)', borderRadius: 4, marginTop: 4 }}>
          <div style={{ height: '100%', width: `${pct}%`, background: alto ? 'var(--bad)' : 'var(--accent)', borderRadius: 4 }} />
        </div>
      )}
    </div>
  )
}

export default function Plan() {
  const [data, setData] = useState(null)
  const [sub, setSub] = useState(null)
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState('')

  const cargar = () => {
    get('/mi-plan').then(setData).catch((e) => setError(e.message))
    get('/suscripcion').then((r) => setSub(r.suscripcion)).catch(() => {})
  }
  useEffect(() => { cargar() }, [])

  const suscribir = async (planId) => {
    setBusy(planId); setError(null)
    try {
      const r = await post('/suscripcion/suscribir', { plan: planId })
      if (r.init_point) window.location.href = r.init_point
      else setError('No se pudo iniciar el pago.')
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const cancelar = async () => {
    if (!window.confirm('¿Cancelar tu suscripción? Volverás al plan Free.')) return
    setBusy('cancelar'); setError(null)
    try { await post('/suscripcion/cancelar', {}); cargar() } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (error && !data) return <div className="alert alert-bad">{error}</div>
  if (!data) return <div className="empty">Cargando…</div>
  const { plan, uso, planes } = data
  const activa = sub && sub.estado === 'authorized'

  return (
    <div className="stack">
      <div><h1>Plan y suscripción</h1><p className="muted sm" style={{ margin: 0 }}>Tu plan actual y cuánto usas.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {sub && (
        <div className="muted sm">
          Suscripción: <b>{sub.estado}</b>
          {sub.vigente_hasta ? ` · vigente hasta ${new Date(sub.vigente_hasta + 'Z').toLocaleDateString('es-CL')}` : ''}
          {activa && <> · <button className="link-btn" disabled={busy === 'cancelar'} onClick={cancelar}>cancelar</button></>}
        </div>
      )}

      <div className="card stack">
        <div className="row" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <b style={{ fontSize: 18 }}>Plan {plan.nombre}</b>
          <span className="badge">{precio(plan.precio_mensual)}</span>
        </div>
        <Barra label="Cuentas de Mercado Libre" used={uso.cuentas} max={plan.max_cuentas} />
        <Barra label="Productos" used={uso.productos} max={plan.max_productos} />
        <Barra label="Plataformas (API keys)" used={uso.plataformas} max={plan.max_plataformas} />
      </div>

      <h2 style={{ fontSize: 16, margin: '4px 0 0' }}>Planes disponibles</h2>
      <div className="row" style={{ gap: 12, flexWrap: 'wrap' }}>
        {planes.map((p) => (
          <div key={p.id} className="card stack" style={{ flex: '1 1 190px', borderColor: p.id === plan.id ? 'var(--accent)' : undefined }}>
            <b>{p.nombre}{p.id === plan.id && ' ✓'}</b>
            <div style={{ fontSize: 20, fontWeight: 700 }}>{p.precio_mensual === 0 ? 'Gratis' : money(p.precio_mensual)}{p.precio_mensual !== 0 && <span className="muted sm"> /mes</span>}</div>
            <ul className="muted sm" style={{ margin: 0, paddingLeft: 16, lineHeight: 1.8 }}>
              <li>{lim(p.max_cuentas)} cuenta(s) ML</li>
              <li>{lim(p.max_productos)} productos</li>
              <li>{lim(p.max_plataformas)} plataforma(s)</li>
            </ul>
            {p.id === plan.id ? (
              <span className="badge" style={{ color: 'var(--ok)' }}>Plan actual</span>
            ) : p.precio_mensual > 0 ? (
              <button className="btn btn-sm btn-accent" disabled={busy === p.id} onClick={() => suscribir(p.id)}>
                {busy === p.id ? 'Redirigiendo…' : 'Suscribirme'}
              </button>
            ) : null}
          </div>
        ))}
      </div>
      <p className="muted sm">Los límites se aplican de forma suave. Hoy estás en <b>{plan.nombre}</b>.</p>
    </div>
  )
}
