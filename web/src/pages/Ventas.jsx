import { useEffect, useState } from 'react'
import { get, money, post, descargarCsv } from '../api'

const PERIODOS = [[7, '7 días'], [30, '30 días'], [90, '90 días']]

function Kpi({ label, value, sub, tone }) {
  return (
    <div className="card" style={{ flex: '1 1 150px' }}>
      <div className="muted sm">{label}</div>
      <div style={{ fontSize: 22, fontWeight: 700, color: tone ? `var(--${tone})` : undefined }}>{value}</div>
      {sub && <div className="muted sm">{sub}</div>}
    </div>
  )
}

const fechaDia = (iso) => { const d = new Date(iso); return isNaN(d) ? iso : d.toLocaleDateString('es-CL', { weekday: 'short', day: '2-digit', month: 'short' }) }

// Gráfico de barras interactivo: cada columna es una zona de hover (fácil de apuntar aunque la
// barra sea fina) → muestra fecha, venta, ganancia y órdenes.
function BarChart({ serie }) {
  const [hi, setHi] = useState(null)
  const maxV = Math.max(1, ...serie.map((d) => d.venta))
  const d = hi != null ? serie[hi] : null
  return (
    <div style={{ position: 'relative' }}>
      <div style={{ display: 'flex', alignItems: 'flex-end', gap: 3, height: 130 }}>
        {serie.map((s, i) => (
          <div key={s.dia} onMouseEnter={() => setHi(i)} onMouseLeave={() => setHi(null)}
            style={{ flex: 1, minWidth: 4, alignSelf: 'stretch', display: 'flex', alignItems: 'flex-end', cursor: 'pointer' }}>
            <div style={{ width: '100%', height: `${Math.max(2, Math.round((s.venta / maxV) * 100))}%`, borderRadius: '3px 3px 0 0', transition: 'background .1s', background: hi === i ? 'var(--accent)' : 'color-mix(in srgb, var(--accent) 72%, transparent)' }} />
          </div>
        ))}
      </div>
      {d && (
        <div style={{ position: 'absolute', top: -6, left: `clamp(0px, calc(${((hi + 0.5) / serie.length) * 100}% - 75px), calc(100% - 150px))`, background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 8, padding: '7px 11px', pointerEvents: 'none', boxShadow: '0 8px 20px rgba(0,0,0,.35)', minWidth: 140, zIndex: 5 }}>
          <div className="xs muted" style={{ textTransform: 'capitalize' }}>{fechaDia(d.dia)}</div>
          <div className="sm" style={{ fontWeight: 700 }}>{money(d.venta)}</div>
          <div className="xs" style={{ color: d.ganancia >= 0 ? 'var(--ok)' : 'var(--bad, #d93025)' }}>ganancia {money(d.ganancia)}</div>
          <div className="xs muted">{d.ordenes} orden(es)</div>
        </div>
      )}
    </div>
  )
}

export default function Ventas() {
  const [dias, setDias] = useState(30)
  const [dash, setDash] = useState(null)
  const [ordenes, setOrdenes] = useState([])
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [sync, setSync] = useState(false)

  const cargar = () => {
    get(`/ventas/dashboard?dias=${dias}`).then(setDash).catch((e) => setError(e.message))
    get('/ventas?limit=20').then(setOrdenes).catch(() => {})
  }
  useEffect(cargar, [dias]) // eslint-disable-line react-hooks/exhaustive-deps

  const sincronizar = async () => {
    setSync(true); setError(null); setMsg(null)
    try {
      const r = await post('/ventas/sincronizar')
      setMsg(`${r.sincronizadas} orden(es) sincronizadas${r.errores ? ` · ${r.errores} cuenta(s) con error (¿activaste el permiso de ventas en Mercado Libre y reconectaste?)` : ''}`)
      cargar()
    } catch (e) { setError(e.message) } finally { setSync(false) }
  }

  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between', alignItems: 'flex-end', flexWrap: 'wrap', gap: 10 }}>
        <div><h1 style={{ margin: 0 }}>Ventas</h1><p className="muted sm" style={{ margin: 0 }}>Órdenes de Mercado Libre con ganancia real (precio − comisión ML − costo).</p></div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={dias} onChange={(e) => setDias(Number(e.target.value))}>
            {PERIODOS.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
          </select>
          <button className="btn btn-sm" onClick={() => descargarCsv(`/ventas/export?dias=${dias}`, `ventas-${dias}d.csv`)}>Exportar CSV</button>
          <button className="btn btn-accent" disabled={sync} onClick={sincronizar}>{sync ? 'Sincronizando…' : 'Sincronizar ventas'}</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {dash && (
        <>
          <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
            <Kpi label="Ventas" value={money(dash.venta_total)} sub={`${dash.ordenes} orden(es)`} />
            <Kpi label="Ganancia" value={money(dash.ganancia_total)} sub={`margen ${dash.margen_pct.toFixed(1)}%`} tone={dash.ganancia_total >= 0 ? 'ok' : 'bad'} />
            <Kpi label="Comisión ML" value={money(dash.comision_total)} />
            <Kpi label="Costo" value={money(dash.costo_total)} />
            <Kpi label="Ticket promedio" value={money(dash.ticket_promedio)} />
            <Kpi label="Sin costo cargado" value={dash.sin_costo} sub="margen no confiable" tone={dash.sin_costo > 0 ? 'warn' : undefined} />
          </div>

          <div className="card">
            <div className="muted sm" style={{ marginBottom: 10 }}>Ventas por día</div>
            {dash.serie.length === 0 ? (
              <div className="empty">Sin ventas en el período.</div>
            ) : (
              <BarChart serie={dash.serie} />
            )}
          </div>

          <div className="card" style={{ padding: 0 }}>
            <div className="muted sm" style={{ padding: '12px 14px 4px' }}>Top productos</div>
            {dash.top_productos.length === 0 ? <div className="empty">—</div> : (
              <table>
                <thead><tr><th>Producto</th><th>Uds</th><th>Venta</th></tr></thead>
                <tbody>{dash.top_productos.map((p) => (
                  <tr key={p.ml_item_id}><td>{p.titulo || p.ml_item_id}</td><td>{p.cantidad}</td><td>{money(p.venta)}</td></tr>
                ))}</tbody>
              </table>
            )}
          </div>

          <div className="card" style={{ padding: 0 }}>
            <div className="muted sm" style={{ padding: '12px 14px 4px' }}>Sin rotación en el período ({dash.sin_rotacion_total}) · productos activos con stock que no vendieron</div>
            {dash.sin_rotacion.length === 0 ? <div className="empty">Todo rotó 🎉</div> : (
              <table>
                <thead><tr><th>Producto</th><th>Stock</th></tr></thead>
                <tbody>{dash.sin_rotacion.map((p) => (
                  <tr key={p.ml_item_id}><td>{p.titulo || p.ml_item_id}</td><td>{p.stock}</td></tr>
                ))}</tbody>
              </table>
            )}
          </div>
        </>
      )}

      <div className="card" style={{ padding: 0 }}>
        <div className="muted sm" style={{ padding: '12px 14px 4px' }}>Órdenes recientes</div>
        {ordenes.length === 0 ? (
          <div className="empty">Aún no hay órdenes. Pulsa “Sincronizar ventas”.</div>
        ) : (
          <table>
            <thead><tr><th>Fecha</th><th>Estado</th><th>Comprador</th><th>Venta</th><th>Ganancia</th></tr></thead>
            <tbody>{ordenes.map((o) => (
              <tr key={o.id}>
                <td className="muted sm">{new Date(o.fecha + 'Z').toLocaleDateString('es-CL')}</td>
                <td><span className="badge">{o.estado}</span></td>
                <td className="muted sm">{o.comprador || '—'}</td>
                <td>{money(o.venta_total)}</td>
                <td style={{ color: `var(--${o.ganancia >= 0 ? 'ok' : 'bad'})` }}>{money(o.ganancia)}</td>
              </tr>
            ))}</tbody>
          </table>
        )}
      </div>
    </div>
  )
}
