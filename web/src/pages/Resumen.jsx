import { useEffect, useState } from 'react'
import { NavLink } from 'react-router-dom'
import { get, money } from '../api'
import Onboarding from './Onboarding'

const horasDesde = (iso) => { const d = new Date(iso); return isNaN(d) ? null : Math.floor((Date.now() - d.getTime()) / 3.6e6) }
const edad = (h) => (h == null ? '' : h < 1 ? '<1 h' : h < 24 ? `${h} h` : `${Math.floor(h / 24)} d`)

function Kpi({ label, value, sub, tone }) {
  return (
    <div className="card" style={{ flex: '1 1 150px' }}>
      <div className="muted sm">{label}</div>
      <div style={{ fontSize: 24, fontWeight: 800, color: tone ? `var(--${tone})` : undefined }}>{value}</div>
      {sub && <div className="muted xs">{sub}</div>}
    </div>
  )
}

export default function Resumen() {
  const [dash, setDash] = useState(null)
  const [alertas, setAlertas] = useState([])
  const [conex, setConex] = useState([])
  const [preguntas, setPreguntas] = useState([])

  useEffect(() => {
    get('/ventas/dashboard?dias=30').then(setDash).catch(() => {})
    get('/productos/alertas?umbral=5').then((d) => setAlertas(Array.isArray(d) ? d : [])).catch(() => {})
    get('/salud/conexiones').then((d) => setConex(Array.isArray(d) ? d : [])).catch(() => {})
    get('/ml/preguntas').then((d) => setPreguntas(Array.isArray(d) ? d : [])).catch(() => {})
  }, [])

  const conexProblema = conex.filter((c) => c.estado !== 'ok').length
  const pregMax = preguntas.length ? Math.max(...preguntas.map((q) => horasDesde(q.fecha) || 0)) : 0
  const maxV = Math.max(1, ...(dash?.serie || []).map((d) => d.venta))

  return (
    <div className="stack">
      <div><h1 style={{ margin: 0 }}>Resumen</h1><p className="muted sm" style={{ margin: 0 }}>Tu negocio de un vistazo · últimos 30 días.</p></div>

      <Onboarding />

      {/* KPIs */}
      <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
        <Kpi label="Ventas" value={dash ? money(dash.venta_total) : '—'} sub={dash ? `${dash.ordenes} órdenes` : ''} />
        <Kpi label="Ganancia" value={dash ? money(dash.ganancia_total) : '—'} sub={dash ? `margen ${dash.margen_pct.toFixed(1)}%` : ''} tone={dash && dash.ganancia_total >= 0 ? 'ok' : 'bad'} />
        <Kpi label="Ticket promedio" value={dash ? money(dash.ticket_promedio) : '—'} />
        <Kpi label="Sin costo cargado" value={dash ? dash.sin_costo : '—'} sub="ganancia no confiable" tone={dash && dash.sin_costo > 0 ? 'warn' : undefined} />
      </div>

      {/* Acciones pendientes */}
      <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
        <NavLink to="/preguntas" className="card" style={{ flex: '1 1 200px', textDecoration: 'none', color: 'inherit', borderLeft: `4px solid ${pregMax >= 24 ? 'var(--bad, #d93025)' : preguntas.length ? 'var(--warn)' : 'var(--ok)'}` }}>
          <div className="muted sm">Preguntas sin responder</div>
          <div style={{ fontSize: 22, fontWeight: 800 }}>{preguntas.length}</div>
          <div className="muted xs">{preguntas.length ? `la más antigua hace ${edad(pregMax)}` : 'todo respondido 🎉'}</div>
        </NavLink>
        <NavLink to="/inventario" className="card" style={{ flex: '1 1 200px', textDecoration: 'none', color: 'inherit', borderLeft: `4px solid ${alertas.length ? 'var(--warn)' : 'var(--ok)'}` }}>
          <div className="muted sm">Stock bajo (≤ 5)</div>
          <div style={{ fontSize: 22, fontWeight: 800 }}>{alertas.length}</div>
          <div className="muted xs">{alertas.length ? 'revisa tu inventario' : 'stock sano 🎉'}</div>
        </NavLink>
        <NavLink to="/conexiones" className="card" style={{ flex: '1 1 200px', textDecoration: 'none', color: 'inherit', borderLeft: `4px solid ${conexProblema ? 'var(--bad, #d93025)' : 'var(--ok)'}` }}>
          <div className="muted sm">Conexiones con atención</div>
          <div style={{ fontSize: 22, fontWeight: 800 }}>{conexProblema}</div>
          <div className="muted xs">{conexProblema ? 'revisa tus integraciones' : `${conex.length} operativas`}</div>
        </NavLink>
        <NavLink to="/reportes" className="card" style={{ flex: '1 1 200px', textDecoration: 'none', color: 'inherit' }}>
          <div className="muted sm">Reportes</div>
          <div style={{ fontSize: 15, fontWeight: 700, marginTop: 6 }}>📄 Ver reporte PDF →</div>
          <div className="muted xs">ventas y rentabilidad por cuenta</div>
        </NavLink>
      </div>

      {/* Mini gráfico de ventas */}
      <div className="card">
        <b className="sm">Ventas diarias (30 días)</b>
        {!dash || dash.serie.length === 0 ? (
          <div className="empty">Sin ventas en el período.</div>
        ) : (
          <div style={{ display: 'flex', alignItems: 'flex-end', gap: 3, height: 90, marginTop: 10 }}>
            {dash.serie.map((d) => (
              <div key={d.dia} title={`${d.dia}: ${money(d.venta)} · ganancia ${money(d.ganancia)} · ${d.ordenes} orden(es)`}
                style={{ flex: 1, minWidth: 3, height: `${Math.max(3, (d.venta / maxV) * 100)}%`, background: 'var(--accent)', borderRadius: '3px 3px 0 0' }} />
            ))}
          </div>
        )}
      </div>

      {/* Top productos */}
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
