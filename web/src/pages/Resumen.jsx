import { useEffect, useRef, useState } from 'react'
import { NavLink } from 'react-router-dom'
import { get, money } from '../api'
import Onboarding from './Onboarding'

const horasDesde = (iso) => { const d = new Date(iso); return isNaN(d) ? null : Math.floor((Date.now() - d.getTime()) / 3.6e6) }
const edad = (h) => (h == null ? '' : h < 1 ? '<1 h' : h < 24 ? `${h} h` : `${Math.floor(h / 24)} d`)
const fechaDia = (iso) => { const d = new Date(iso); return isNaN(d) ? iso : d.toLocaleDateString('es-CL', { day: '2-digit', month: 'short' }) }

// Tendencia vs período anterior → chip con flecha + color de estado (no color solo).
function Trend({ cur, prev }) {
  if (prev == null || prev === 0) return <span className="muted xs">sin comparación</span>
  const pct = ((cur - prev) / Math.abs(prev)) * 100
  const up = pct >= 0
  const col = up ? 'var(--ok)' : 'var(--bad, #d93025)'
  return <span className="xs" style={{ color: col, fontWeight: 700 }}>{up ? '▲' : '▼'} {Math.abs(pct).toFixed(0)}% <span className="muted" style={{ fontWeight: 400 }}>vs 30d previos</span></span>
}

function StatTile({ label, value, children }) {
  return (
    <div className="card" style={{ flex: '1 1 160px', display: 'flex', flexDirection: 'column', gap: 2 }}>
      <div className="muted sm" style={{ textTransform: 'uppercase', letterSpacing: '.04em', fontSize: 11, fontWeight: 700 }}>{label}</div>
      <div style={{ fontSize: 26, fontWeight: 800, letterSpacing: '-.01em', lineHeight: 1.1 }}>{value}</div>
      {children}
    </div>
  )
}

// Gráfico de área responsive con línea 2px, degradado y hover (una sola serie → sin leyenda).
function SalesArea({ serie }) {
  const ref = useRef(null)
  const [w, setW] = useState(640)
  const [hi, setHi] = useState(null)
  useEffect(() => {
    const el = ref.current; if (!el) return
    const ro = new ResizeObserver(() => setW(Math.max(200, el.clientWidth)))
    ro.observe(el); setW(Math.max(200, el.clientWidth))
    return () => ro.disconnect()
  }, [])
  const H = 150, padT = 12, padB = 26
  const n = serie.length
  const maxV = Math.max(1, ...serie.map((d) => d.venta))
  const x = (i) => (n <= 1 ? w / 2 : (i / (n - 1)) * (w - 4) + 2)
  const y = (v) => padT + (1 - v / maxV) * (H - padT - padB)
  const line = serie.map((d, i) => `${i === 0 ? 'M' : 'L'}${x(i).toFixed(1)},${y(d.venta).toFixed(1)}`).join(' ')
  const area = n ? `${line} L${x(n - 1).toFixed(1)},${H - padB} L${x(0).toFixed(1)},${H - padB} Z` : ''
  const onMove = (e) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const px = e.clientX - rect.left
    const i = Math.max(0, Math.min(n - 1, Math.round(((px - 2) / (w - 4)) * (n - 1))))
    setHi(i)
  }
  const d = hi != null ? serie[hi] : null
  return (
    <div ref={ref} style={{ position: 'relative', width: '100%' }}>
      <svg width="100%" height={H} viewBox={`0 0 ${w} ${H}`} preserveAspectRatio="none" onMouseMove={onMove} onMouseLeave={() => setHi(null)} style={{ display: 'block', overflow: 'visible' }}>
        <defs>
          <linearGradient id="gv" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.28" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.02" />
          </linearGradient>
        </defs>
        {/* grilla recesiva: línea base */}
        <line x1="0" y1={H - padB} x2={w} y2={H - padB} stroke="var(--line)" strokeWidth="1" />
        {area && <path d={area} fill="url(#gv)" />}
        {line && <path d={line} fill="none" stroke="var(--accent)" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />}
        {d && (
          <>
            <line x1={x(hi)} y1={padT} x2={x(hi)} y2={H - padB} stroke="var(--accent)" strokeWidth="1" strokeDasharray="3 3" opacity="0.6" />
            <circle cx={x(hi)} cy={y(d.venta)} r="4" fill="var(--accent)" stroke="var(--surface)" strokeWidth="2" />
          </>
        )}
      </svg>
      {d && (
        <div style={{ position: 'absolute', top: 0, left: Math.min(Math.max(x(hi) - 70, 0), Math.max(0, w - 140)), background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 8, padding: '6px 10px', pointerEvents: 'none', boxShadow: '0 6px 18px rgba(0,0,0,.3)', minWidth: 120 }}>
          <div className="xs muted">{fechaDia(d.dia)}</div>
          <div className="sm" style={{ fontWeight: 700 }}>{money(d.venta)}</div>
          <div className="xs" style={{ color: d.ganancia >= 0 ? 'var(--ok)' : 'var(--bad, #d93025)' }}>ganancia {money(d.ganancia)}</div>
          <div className="xs muted">{d.ordenes} orden(es)</div>
        </div>
      )}
    </div>
  )
}

export default function Resumen() {
  const [dash, setDash] = useState(null)
  const [prev, setPrev] = useState(null) // totales de los 30 días previos
  const [alertas, setAlertas] = useState([])
  const [conex, setConex] = useState([])
  const [preguntas, setPreguntas] = useState([])

  useEffect(() => {
    get('/ventas/dashboard?dias=30').then(setDash).catch(() => {})
    Promise.all([get('/ventas/dashboard?dias=60').catch(() => null), get('/ventas/dashboard?dias=30').catch(() => null)])
      .then(([d60, d30]) => { if (d60 && d30) setPrev({ venta_total: d60.venta_total - d30.venta_total, ganancia_total: d60.ganancia_total - d30.ganancia_total, ordenes: d60.ordenes - d30.ordenes }) })
    get('/productos/alertas?umbral=5').then((d) => setAlertas(Array.isArray(d) ? d : [])).catch(() => {})
    get('/salud/conexiones').then((d) => setConex(Array.isArray(d) ? d : [])).catch(() => {})
    get('/ml/preguntas').then((d) => setPreguntas(Array.isArray(d) ? d : [])).catch(() => {})
  }, [])

  const conexProblema = conex.filter((c) => c.estado !== 'ok').length
  const pregMax = preguntas.length ? Math.max(...preguntas.map((q) => horasDesde(q.fecha) || 0)) : 0
  const maxCuenta = Math.max(1, ...((dash?.por_cuenta) || []).map((c) => c.ventas))

  const AccionCard = ({ to, label, value, sub, color }) => (
    <NavLink to={to} className="card" style={{ flex: '1 1 190px', textDecoration: 'none', color: 'inherit', borderLeft: `4px solid ${color}`, display: 'flex', flexDirection: 'column', gap: 2 }}>
      <div className="muted sm">{label}</div>
      <div style={{ fontSize: 22, fontWeight: 800 }}>{value}</div>
      <div className="muted xs">{sub}</div>
    </NavLink>
  )

  return (
    <div className="stack">
      <div><h1 style={{ margin: 0 }}>Resumen</h1><p className="muted sm" style={{ margin: 0 }}>Tu negocio de un vistazo · últimos 30 días.</p></div>

      <Onboarding />

      <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
        <StatTile label="Ventas" value={dash ? money(dash.venta_total) : '—'}>
          {dash && <Trend cur={dash.venta_total} prev={prev?.venta_total} />}
        </StatTile>
        <StatTile label="Ganancia" value={dash ? money(dash.ganancia_total) : '—'}>
          {dash && <span className="xs muted">margen {dash.margen_pct.toFixed(1)}% · <Trend cur={dash.ganancia_total} prev={prev?.ganancia_total} /></span>}
        </StatTile>
        <StatTile label="Órdenes" value={dash ? dash.ordenes : '—'}>
          {dash && <Trend cur={dash.ordenes} prev={prev?.ordenes} />}
        </StatTile>
        <StatTile label="Ticket promedio" value={dash ? money(dash.ticket_promedio) : '—'} />
      </div>

      <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
        <AccionCard to="/preguntas" label="Preguntas sin responder" value={preguntas.length} sub={preguntas.length ? `la más antigua hace ${edad(pregMax)}` : 'todo respondido 🎉'} color={pregMax >= 24 ? 'var(--bad, #d93025)' : preguntas.length ? 'var(--warn)' : 'var(--ok)'} />
        <AccionCard to="/inventario" label="Stock bajo (≤ 5)" value={alertas.length} sub={alertas.length ? 'revisa tu inventario' : 'stock sano 🎉'} color={alertas.length ? 'var(--warn)' : 'var(--ok)'} />
        <AccionCard to="/conexiones" label="Conexiones con atención" value={conexProblema} sub={conexProblema ? 'revisa tus integraciones' : `${conex.length} operativas`} color={conexProblema ? 'var(--bad, #d93025)' : 'var(--ok)'} />
        <AccionCard to="/reportes" label="Reportes" value="📄" sub="ventas y rentabilidad · PDF" color="var(--accent)" />
      </div>

      <div className="row" style={{ gap: 12, alignItems: 'stretch', flexWrap: 'wrap' }}>
        <div className="card" style={{ flex: '2 1 420px', minWidth: 0 }}>
          <div className="row between"><b className="sm">Ventas diarias</b><span className="muted xs">últimos 30 días</span></div>
          {!dash ? <div className="empty">Cargando…</div> : dash.serie.length === 0 ? <div className="empty">Sin ventas en el período.</div> : <SalesArea serie={dash.serie} />}
        </div>
        <div className="card" style={{ flex: '1 1 260px', minWidth: 0 }}>
          <b className="sm">Ventas por cuenta</b>
          {!dash || (dash.por_cuenta || []).length === 0 ? <div className="empty">—</div> : (
            <div className="stack" style={{ gap: 10, marginTop: 10 }}>
              {dash.por_cuenta.slice(0, 6).map((c, i) => (
                <div key={i} className="stack" style={{ gap: 3 }}>
                  <div className="row between" style={{ fontSize: 13 }}><span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.cuenta}</span><b>{money(c.ventas)}</b></div>
                  <div style={{ height: 7, background: 'var(--bg)', borderRadius: 999, overflow: 'hidden' }}><div style={{ width: `${Math.max(3, (c.ventas / maxCuenta) * 100)}%`, height: '100%', background: 'var(--accent)', borderRadius: 999 }} /></div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {dash && dash.top_productos && dash.top_productos.length > 0 && (
        <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
          <div className="row between" style={{ padding: '12px 14px 4px' }}><b className="sm">Más vendidos (30 días)</b><NavLink to="/ventas" className="link-btn">ver ventas</NavLink></div>
          <table>
            <thead><tr><th>Producto</th><th>Unid.</th><th>Ventas</th></tr></thead>
            <tbody>{dash.top_productos.slice(0, 5).map((p) => (
              <tr key={p.ml_item_id}><td><b>{p.titulo || p.ml_item_id}</b></td><td>{p.cantidad}</td><td>{money(p.venta)}</td></tr>
            ))}</tbody>
          </table>
        </div>
      )}
    </div>
  )
}
