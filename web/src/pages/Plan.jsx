import { useEffect, useState } from 'react'
import { get, money, post } from '../api'

const lim = (v) => (v < 0 ? '∞' : v)

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
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState('')
  const [ciclo, setCiclo] = useState('mensual')
  const [cupInput, setCupInput] = useState('')
  const [cupon, setCupon] = useState(null) // { codigo, tipo, valor, descripcion }
  const [cupError, setCupError] = useState(null)
  const [factFalta, setFactFalta] = useState(false)

  const cargar = () => {
    get('/mi-plan').then(setData).catch((e) => setError(e.message))
    get('/facturacion').then((r) => {
      const f = r.facturacion
      const ok = f && f.rut && f.razon_social && f.direccion && f.comuna && (f.tipo_documento !== 'factura' || f.giro)
      setFactFalta(!ok)
    }).catch(() => {})
  }
  useEffect(() => { cargar() }, [])

  const base = (p) => (ciclo === 'anual' ? p.precio_anual : p.precio_mensual)
  const conDescuento = (precio) => {
    if (!cupon || precio <= 0) return precio
    const d = cupon.tipo === 'porcentaje' ? Math.round((precio * Math.min(100, cupon.valor)) / 100) : cupon.valor
    return Math.max(0, precio - d)
  }
  const ahorroAnual = (p) => {
    if (p.precio_mensual <= 0 || p.precio_anual <= 0) return 0
    return Math.round((1 - p.precio_anual / (p.precio_mensual * 12)) * 100)
  }

  const aplicarCupon = async () => {
    setCupError(null)
    const codigo = cupInput.trim()
    if (!codigo) { setCupon(null); return }
    try { const d = await post('/cupon/validar', { codigo }); setCupon(d) }
    catch (e) { setCupon(null); setCupError(e.message) }
  }

  const suscribir = async (planId) => {
    setBusy(planId); setError(null)
    try {
      const r = await post('/suscripcion/suscribir', { plan: planId, ciclo, cupon: cupon?.codigo })
      if (r.init_point) window.location.href = r.init_point
      else setError('No se pudo iniciar el pago.')
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (error && !data) return <div className="alert alert-bad">{error}</div>
  if (!data) return <div className="empty">Cargando…</div>
  const { plan, uso, planes } = data
  const sufijo = ciclo === 'anual' ? '/año' : '/mes'

  return (
    <div className="stack">
      <div><h1>Plan y suscripción</h1><p className="muted sm" style={{ margin: 0 }}>Tu plan actual, cuánto usas y cómo mejorarlo.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {factFalta && (
        <div className="alert">📄 Para suscribirte a un plan de pago, primero completa tus <a href="/facturacion">datos de facturación</a>.</div>
      )}

      <div className="card stack">
        <div className="row" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <b style={{ fontSize: 18 }}>Plan {plan.nombre}</b>
          <span className="badge">{plan.precio_mensual === 0 ? 'Gratis' : `${money(plan.precio_mensual)}/mes`}</span>
        </div>
        <Barra label="Cuentas de Mercado Libre" used={uso.cuentas} max={plan.max_cuentas} />
        <Barra label="Productos" used={uso.productos} max={plan.max_productos} />
        <Barra label="Plataformas (API keys)" used={uso.plataformas} max={plan.max_plataformas} />
      </div>

      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'center' }}>
        <h2 style={{ fontSize: 16, margin: 0 }}>Planes disponibles</h2>
        <div className="seg">
          <button className={ciclo === 'mensual' ? 'on' : ''} onClick={() => setCiclo('mensual')}>Mensual</button>
          <button className={ciclo === 'anual' ? 'on' : ''} onClick={() => setCiclo('anual')}>Anual <span className="sm">· 2 meses gratis</span></button>
        </div>
      </div>

      <div className="plan-grid">
        {planes.map((p) => {
          const b = base(p)
          const final = conDescuento(b)
          const esActual = p.id === plan.id
          const ah = ciclo === 'anual' ? ahorroAnual(p) : 0
          return (
            <div key={p.id} className="card plan-card" style={{ borderColor: esActual ? 'var(--accent)' : undefined }}>
              <b style={{ fontSize: 15 }}>{p.nombre}{esActual && ' ✓'}</b>
              <div style={{ margin: '6px 0' }}>
                {b === 0 ? (
                  <span style={{ fontSize: 22, fontWeight: 700 }}>Gratis</span>
                ) : (
                  <>
                    {cupon && final !== b && <span className="muted" style={{ textDecoration: 'line-through', marginRight: 6 }}>{money(b)}</span>}
                    <span style={{ fontSize: 22, fontWeight: 700 }}>{money(final)}</span>
                    <span className="muted sm">{sufijo}</span>
                  </>
                )}
                {ah > 0 && <div className="sm" style={{ color: 'var(--ok)' }}>Ahorras {ah}% vs mensual</div>}
                {cupon && final !== b && b > 0 && <div className="sm" style={{ color: 'var(--ok)' }}>Cupón {cupon.codigo}: {cupon.descripcion}</div>}
              </div>
              <ul className="muted sm" style={{ margin: 0, paddingLeft: 16, lineHeight: 1.8 }}>
                <li>{lim(p.max_cuentas)} cuenta(s) ML</li>
                <li>{lim(p.max_productos)} productos</li>
                <li>{lim(p.max_plataformas)} plataforma(s)</li>
              </ul>
              <div style={{ marginTop: 'auto', paddingTop: 10 }}>
                {esActual ? (
                  <span className="badge" style={{ color: 'var(--ok)' }}>Plan actual</span>
                ) : b > 0 ? (
                  <button className="btn btn-sm btn-accent" style={{ width: '100%' }} disabled={busy === p.id} onClick={() => suscribir(p.id)}>
                    {busy === p.id ? 'Redirigiendo…' : 'Suscribirme'}
                  </button>
                ) : (
                  <span className="muted sm">Plan de entrada</span>
                )}
              </div>
            </div>
          )
        })}
      </div>

      <div className="card stack" style={{ maxWidth: 420 }}>
        <b className="sm">¿Tienes un código de descuento?</b>
        <div className="row" style={{ gap: 8 }}>
          <input className="input" placeholder="CÓDIGO" value={cupInput}
            onChange={(e) => setCupInput(e.target.value.toUpperCase())} style={{ textTransform: 'uppercase' }} />
          <button className="btn btn-sm" onClick={aplicarCupon}>Aplicar</button>
          {cupon && <button className="btn btn-sm" onClick={() => { setCupon(null); setCupInput(''); setCupError(null) }}>Quitar</button>}
        </div>
        {cupon && <div className="sm" style={{ color: 'var(--ok)' }}>✓ {cupon.codigo}: {cupon.descripcion}</div>}
        {cupError && <div className="sm" style={{ color: 'var(--bad)' }}>{cupError}</div>}
      </div>

      <p className="muted sm">Los pagos se procesan con Mercado Pago. Puedes cancelar cuando quieras.</p>
    </div>
  )
}
