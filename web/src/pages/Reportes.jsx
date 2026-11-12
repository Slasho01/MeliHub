import { useEffect, useState } from 'react'
import { get, money } from '../api'

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]))
const hoy = () => new Date().toLocaleDateString('es-CL', { day: '2-digit', month: '2-digit', year: 'numeric' })

export default function Reportes() {
  const [dias, setDias] = useState(30)
  const [dash, setDash] = useState(null)
  const [empresa, setEmpresa] = useState({ razon_social: '', rut: '' })
  const [logo, setLogo] = useState('')
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)

  const cargar = () => {
    setCargando(true); setError('')
    get(`/ventas/dashboard?dias=${dias}`).then(setDash).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [dias]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    get('/cotizador/logo').then((r) => setLogo(r?.logo || '')).catch(() => {})
    get('/facturacion').then((f) => { if (f) setEmpresa({ razon_social: f.razon_social || '', rut: f.rut || '' }) }).catch(() => {})
  }, [])

  const generarPDF = () => {
    if (!dash) return
    const kpi = (l, v) => `<div class="kpi"><div class="l">${l}</div><div class="v">${v}</div></div>`
    const kpis = [
      kpi('Ventas', money(dash.venta_total)),
      kpi('Ganancia', money(dash.ganancia_total)),
      kpi('Margen', `${dash.margen_pct.toFixed(1)}%`),
      kpi('Órdenes', dash.ordenes),
      kpi('Ticket prom.', money(dash.ticket_promedio)),
    ].join('')
    const filasCuenta = (dash.por_cuenta || []).map((c) => `<tr><td>${esc(c.cuenta)}</td><td class="c">${c.ordenes}</td><td class="r">${money(c.ventas)}</td><td class="r">${money(c.ganancia)}</td></tr>`).join('') || '<tr><td colspan="4" class="mut">Sin ventas en el período.</td></tr>'
    const filasTop = (dash.top_productos || []).map((p, i) => `<tr><td class="c">${i + 1}</td><td>${esc(p.titulo || p.ml_item_id)}</td><td class="c">${p.cantidad}</td><td class="r">${money(p.venta)}</td></tr>`).join('') || '<tr><td colspan="4" class="mut">—</td></tr>'
    const html = `<!doctype html><html lang="es"><head><meta charset="utf-8"><title>Reporte de ventas · ${esc(empresa.razon_social)}</title>
<style>
  :root{--navy:#13233b;--accent:#2a6f97;--soft:#f3f6fa;--line:#dfe4ea;--ink:#1f2733;--mut:#6b7280;}
  *{box-sizing:border-box;} body{font-family:-apple-system,'Segoe UI',Roboto,Arial,sans-serif;color:var(--ink);font-size:13px;margin:0;}
  .doc{max-width:820px;margin:0 auto;padding:40px 44px;}
  .top{display:flex;justify-content:space-between;align-items:flex-start;gap:24px;}
  .brand{display:flex;gap:14px;align-items:center;} .brand img{max-height:56px;max-width:180px;object-fit:contain;}
  .brand .name{font-size:19px;font-weight:800;color:var(--navy);} .brand .det{color:var(--mut);font-size:11.5px;}
  .meta{text-align:right;} .meta .t{font-size:18px;font-weight:800;letter-spacing:.04em;color:var(--navy);} .meta div{color:var(--mut);font-size:12px;}
  .rule{height:3px;background:var(--accent);border-radius:3px;margin:16px 0 18px;}
  .kpis{display:flex;gap:10px;flex-wrap:wrap;margin-bottom:18px;}
  .kpi{flex:1 1 120px;background:var(--soft);border:1px solid var(--line);border-radius:8px;padding:10px 12px;}
  .kpi .l{font-size:10.5px;text-transform:uppercase;letter-spacing:.06em;color:var(--mut);font-weight:700;} .kpi .v{font-size:17px;font-weight:800;color:var(--navy);margin-top:2px;}
  h2{font-size:13px;text-transform:uppercase;letter-spacing:.05em;color:var(--navy);margin:18px 0 6px;}
  table{width:100%;border-collapse:collapse;} th{background:var(--navy);color:#fff;text-align:left;font-size:11px;text-transform:uppercase;padding:8px 10px;}
  th.c,td.c{text-align:center;} th.r,td.r{text-align:right;white-space:nowrap;}
  td{padding:8px 10px;border-bottom:1px solid var(--line);} td.mut{color:var(--mut);}
  .foot{margin-top:24px;padding-top:12px;border-top:1px solid var(--line);color:var(--mut);font-size:11px;text-align:center;}
  @media print{body{-webkit-print-color-adjust:exact;print-color-adjust:exact;} .doc{padding:10px 8px;}}
</style></head><body><div class="doc">
  <div class="top">
    <div class="brand">${logo ? `<img src="${logo}" alt="">` : ''}<div><div class="name">${esc(empresa.razon_social) || 'Mi Empresa'}</div><div class="det">${empresa.rut ? 'RUT ' + esc(empresa.rut) : ''}</div></div></div>
    <div class="meta"><div class="t">REPORTE DE VENTAS</div><div>Últimos ${dash.dias} días</div><div>Generado: ${hoy()}</div></div>
  </div>
  <div class="rule"></div>
  <div class="kpis">${kpis}</div>
  <h2>Ventas por cuenta</h2>
  <table><thead><tr><th>Cuenta</th><th class="c">Órdenes</th><th class="r">Ventas</th><th class="r">Ganancia</th></tr></thead><tbody>${filasCuenta}</tbody></table>
  <h2>Productos más vendidos</h2>
  <table><thead><tr><th class="c">#</th><th>Producto</th><th class="c">Unid.</th><th class="r">Ventas</th></tr></thead><tbody>${filasTop}</tbody></table>
  <div class="foot">${esc(empresa.razon_social)} · Reporte generado el ${hoy()} · Ganancia = venta − comisión ML − costo · Montos en CLP</div>
</div></body></html>`
    const win = window.open('', '_blank')
    if (!win) { setError('Permite pop-ups para descargar el PDF.'); return }
    win.document.write(html); win.document.close(); win.focus(); setTimeout(() => win.print(), 350)
  }

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Reportes</h1>
          <p className="muted sm" style={{ margin: 0 }}>Resumen de ventas y rentabilidad por cuenta, exportable a PDF con tu marca.</p>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={dias} onChange={(e) => setDias(Number(e.target.value))}>
            <option value={7}>Últimos 7 días</option>
            <option value={30}>Últimos 30 días</option>
            <option value={90}>Últimos 90 días</option>
            <option value={365}>Último año</option>
          </select>
          <button className="btn btn-dark" disabled={!dash || cargando} onClick={generarPDF}>📄 Reporte PDF</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {!dash ? <div className="muted">Cargando…</div> : (
        <>
          <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
            {[['Ventas', money(dash.venta_total)], ['Ganancia', money(dash.ganancia_total)], ['Margen', `${dash.margen_pct.toFixed(1)}%`], ['Órdenes', dash.ordenes], ['Ticket prom.', money(dash.ticket_promedio)]].map(([l, v]) => (
              <div key={l} className="card" style={{ flex: '1 1 130px' }}><div className="muted sm">{l}</div><div style={{ fontSize: 22, fontWeight: 800 }}>{v}</div></div>
            ))}
          </div>

          <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
            <div className="row between" style={{ padding: '12px 14px 4px' }}><b className="sm">Ventas por cuenta</b></div>
            {(dash.por_cuenta || []).length === 0 ? <div className="empty">Sin ventas en el período.</div> : (
              <table>
                <thead><tr><th>Cuenta</th><th>Órdenes</th><th>Ventas</th><th>Ganancia</th></tr></thead>
                <tbody>{dash.por_cuenta.map((c, i) => (
                  <tr key={i}><td><b>{c.cuenta}</b></td><td>{c.ordenes}</td><td>{money(c.ventas)}</td><td style={{ color: `var(--${c.ganancia >= 0 ? 'ok' : 'bad'})` }}>{money(c.ganancia)}</td></tr>
                ))}</tbody>
              </table>
            )}
          </div>

          <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
            <div className="row between" style={{ padding: '12px 14px 4px' }}><b className="sm">Productos más vendidos</b></div>
            {(dash.top_productos || []).length === 0 ? <div className="empty">—</div> : (
              <table>
                <thead><tr><th>#</th><th>Producto</th><th>Unid.</th><th>Ventas</th></tr></thead>
                <tbody>{dash.top_productos.map((p, i) => (
                  <tr key={p.ml_item_id}><td>{i + 1}</td><td><b>{p.titulo || p.ml_item_id}</b></td><td>{p.cantidad}</td><td>{money(p.venta)}</td></tr>
                ))}</tbody>
              </table>
            )}
          </div>
        </>
      )}
    </div>
  )
}
