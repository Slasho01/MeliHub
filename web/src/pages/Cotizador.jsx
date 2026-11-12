import { useEffect, useMemo, useState } from 'react'
import { get, money } from '../api'

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]))
const hoy = () => new Date().toLocaleDateString('es-CL', { day: '2-digit', month: '2-digit', year: 'numeric' })
const LS = 'melihub_cotizador_empresa'

export default function Cotizador() {
  const [productos, setProductos] = useState([])
  const [empresa, setEmpresa] = useState({ razon_social: '', rut: '', direccion: '', telefono: '', email: '' })
  const [cliente, setCliente] = useState({ nombre: '', rut: '', email: '' })
  const [lineas, setLineas] = useState([])
  const [descuentoPct, setDescuentoPct] = useState('')
  const [iva, setIva] = useState(false)
  const [notas, setNotas] = useState('')
  const [buscar, setBuscar] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    get('/productos').then((d) => setProductos(Array.isArray(d) ? d : (d?.data || []))).catch(() => {})
    // Datos de empresa: primero lo recordado localmente, si no, desde facturación.
    let guardado = null
    try { guardado = JSON.parse(localStorage.getItem(LS) || 'null') } catch { /* ignore */ }
    if (guardado) setEmpresa((e) => ({ ...e, ...guardado }))
    else get('/facturacion').then((f) => { if (f) setEmpresa((e) => ({ ...e, razon_social: f.razon_social || '', rut: f.rut || '', direccion: f.direccion || '', telefono: f.telefono || '' })) }).catch(() => {})
  }, [])

  const setEmp = (k, v) => setEmpresa((e) => { const n = { ...e, [k]: v }; try { localStorage.setItem(LS, JSON.stringify(n)) } catch { /* ignore */ } return n })

  const resultados = useMemo(() => {
    const q = buscar.trim().toLowerCase()
    if (!q) return []
    return productos.filter((p) => (p.titulo || '').toLowerCase().includes(q) || (p.ml_item_id || '').toLowerCase().includes(q)).slice(0, 8)
  }, [buscar, productos])

  const agregar = (p) => {
    const precio = p.precio_jarstore || p.precio || 0
    setLineas((l) => [...l, { key: `${p.id}-${Date.now()}`, nombre: p.titulo || p.ml_item_id, cantidad: 1, precio }])
    setBuscar('')
  }
  const agregarManual = () => setLineas((l) => [...l, { key: `m-${Date.now()}`, nombre: '', cantidad: 1, precio: 0 }])
  const setLinea = (key, campo, v) => setLineas((l) => l.map((x) => (x.key === key ? { ...x, [campo]: v } : x)))
  const quitar = (key) => setLineas((l) => l.filter((x) => x.key !== key))

  const subtotal = lineas.reduce((s, x) => s + (Number(x.cantidad) || 0) * (Number(x.precio) || 0), 0)
  const descuento = Math.round(subtotal * (Number(descuentoPct) || 0) / 100)
  const neto = subtotal - descuento
  const ivaMonto = iva ? Math.round(neto * 0.19) : 0
  const total = neto + ivaMonto

  const generarPDF = () => {
    if (!lineas.length) { setError('Agrega al menos un producto a la cotización.'); return }
    setError('')
    const filas = lineas.map((x) => {
      const tot = (Number(x.cantidad) || 0) * (Number(x.precio) || 0)
      return `<tr><td>${esc(x.nombre)}</td><td class="c">${Number(x.cantidad) || 0}</td><td class="r">${money(x.precio)}</td><td class="r">${money(tot)}</td></tr>`
    }).join('')
    const totales = [
      `<tr><td class="r lbl" colspan="3">Subtotal</td><td class="r">${money(subtotal)}</td></tr>`,
      descuento > 0 ? `<tr><td class="r lbl" colspan="3">Descuento (${Number(descuentoPct)}%)</td><td class="r">- ${money(descuento)}</td></tr>` : '',
      iva ? `<tr><td class="r lbl" colspan="3">Neto</td><td class="r">${money(neto)}</td></tr><tr><td class="r lbl" colspan="3">IVA (19%)</td><td class="r">${money(ivaMonto)}</td></tr>` : '',
      `<tr class="tot"><td class="r lbl" colspan="3">TOTAL</td><td class="r">${money(total)}</td></tr>`,
    ].join('')
    const html = `<!doctype html><html lang="es"><head><meta charset="utf-8"><title>Cotización ${esc(empresa.razon_social)}</title>
<style>
  * { box-sizing: border-box; } body { font-family: -apple-system, Segoe UI, Roboto, Arial, sans-serif; color: #1a1a1a; margin: 0; padding: 36px 40px; font-size: 13px; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 3px solid #111; padding-bottom: 14px; }
  .emp h1 { margin: 0 0 4px; font-size: 22px; } .emp div { color: #555; font-size: 12px; line-height: 1.5; }
  .meta { text-align: right; } .meta .t { font-size: 20px; font-weight: 800; letter-spacing: .04em; } .meta div { color: #555; font-size: 12px; }
  .cli { margin: 20px 0 14px; } .cli b { display: block; font-size: 11px; text-transform: uppercase; letter-spacing: .05em; color: #888; margin-bottom: 3px; }
  table { width: 100%; border-collapse: collapse; } th, td { padding: 9px 10px; border-bottom: 1px solid #e3e3e3; text-align: left; }
  th { background: #f4f4f4; font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: #555; }
  td.c { text-align: center; } td.r { text-align: right; white-space: nowrap; } td.lbl { color: #555; font-weight: 600; border: none; }
  tr.tot td { font-size: 16px; font-weight: 800; border-top: 2px solid #111; border-bottom: none; }
  .notas { margin-top: 22px; color: #555; font-size: 12px; white-space: pre-wrap; } .foot { margin-top: 26px; color: #999; font-size: 11px; text-align: center; }
  @media print { body { padding: 0; } }
</style></head><body>
  <div class="head">
    <div class="emp"><h1>${esc(empresa.razon_social) || 'Mi Empresa'}</h1>
      <div>${empresa.rut ? 'RUT ' + esc(empresa.rut) + '<br>' : ''}${esc(empresa.direccion)}${empresa.direccion ? '<br>' : ''}${[empresa.telefono, empresa.email].filter(Boolean).map(esc).join(' · ')}</div>
    </div>
    <div class="meta"><div class="t">COTIZACIÓN</div><div>Fecha: ${hoy()}</div></div>
  </div>
  <div class="cli"><b>Cliente</b>${esc(cliente.nombre) || '—'}${cliente.rut ? '<br>RUT ' + esc(cliente.rut) : ''}${cliente.email ? '<br>' + esc(cliente.email) : ''}</div>
  <table>
    <thead><tr><th>Producto</th><th style="text-align:center">Cant.</th><th style="text-align:right">P. unitario</th><th style="text-align:right">Total</th></tr></thead>
    <tbody>${filas}</tbody>
    <tfoot>${totales}</tfoot>
  </table>
  ${notas.trim() ? `<div class="notas"><b>Notas:</b> ${esc(notas)}</div>` : ''}
  <div class="foot">Cotización generada el ${hoy()} · Precios en CLP · Validez 15 días salvo indicación contraria.</div>
</body></html>`
    const win = window.open('', '_blank')
    if (!win) { setError('El navegador bloqueó la ventana. Permite pop-ups para descargar el PDF.'); return }
    win.document.write(html)
    win.document.close()
    win.focus()
    setTimeout(() => win.print(), 300)
  }

  return (
    <div className="stack">
      <div>
        <h1 style={{ margin: 0 }}>Cotizador</h1>
        <p className="muted sm" style={{ margin: 0 }}>Arma una cotización con productos de tu inventario, aplica descuento y descárgala en PDF.</p>
      </div>
      {error && <div className="alert alert-bad">{error}</div>}

      <div className="row" style={{ gap: 12, alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <div className="card stack" style={{ gap: 8, flex: '1 1 300px' }}>
          <b className="sm">Tu empresa</b>
          <input className="input" placeholder="Razón social / nombre" value={empresa.razon_social} onChange={(e) => setEmp('razon_social', e.target.value)} />
          <div className="row" style={{ gap: 8 }}>
            <input className="input" placeholder="RUT" value={empresa.rut} onChange={(e) => setEmp('rut', e.target.value)} />
            <input className="input" placeholder="Teléfono" value={empresa.telefono} onChange={(e) => setEmp('telefono', e.target.value)} />
          </div>
          <input className="input" placeholder="Dirección" value={empresa.direccion} onChange={(e) => setEmp('direccion', e.target.value)} />
          <input className="input" placeholder="Email (opcional)" value={empresa.email} onChange={(e) => setEmp('email', e.target.value)} />
        </div>
        <div className="card stack" style={{ gap: 8, flex: '1 1 300px' }}>
          <b className="sm">Cliente</b>
          <input className="input" placeholder="Nombre del cliente" value={cliente.nombre} onChange={(e) => setCliente({ ...cliente, nombre: e.target.value })} />
          <div className="row" style={{ gap: 8 }}>
            <input className="input" placeholder="RUT (opcional)" value={cliente.rut} onChange={(e) => setCliente({ ...cliente, rut: e.target.value })} />
            <input className="input" placeholder="Email (opcional)" value={cliente.email} onChange={(e) => setCliente({ ...cliente, email: e.target.value })} />
          </div>
        </div>
      </div>

      <div className="card stack" style={{ gap: 10 }}>
        <b className="sm">Productos</b>
        <div style={{ position: 'relative' }}>
          <input className="input" placeholder="Buscar un producto del inventario…" value={buscar} onChange={(e) => setBuscar(e.target.value)} />
          {resultados.length > 0 && (
            <div style={{ position: 'absolute', zIndex: 20, left: 0, right: 0, top: '100%', marginTop: 2, maxHeight: 260, overflowY: 'auto', background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 8, boxShadow: '0 10px 24px rgba(0,0,0,.45)' }}>
              {resultados.map((p) => (
                <div key={p.id} onMouseDown={(e) => { e.preventDefault(); agregar(p) }} style={{ padding: '8px 10px', cursor: 'pointer', display: 'flex', gap: 8, alignItems: 'center', borderBottom: '1px solid var(--line)' }}>
                  {p.thumbnail && <img src={p.thumbnail} alt="" width="30" height="30" style={{ borderRadius: 5, objectFit: 'cover' }} />}
                  <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 13 }}>{p.titulo}</span>
                  <span className="muted sm">{money(p.precio_jarstore || p.precio)}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        {lineas.length === 0 ? (
          <div className="muted sm">Busca y agrega productos, o <button className="link-btn" onClick={agregarManual}>agrega una línea manual</button>.</div>
        ) : (
          <div className="table-wrap" style={{ overflowX: 'auto' }}>
            <table style={{ minWidth: 560 }}>
              <thead><tr><th>Producto</th><th>Cant.</th><th>P. unitario</th><th>Total</th><th></th></tr></thead>
              <tbody>
                {lineas.map((x) => (
                  <tr key={x.key}>
                    <td style={{ minWidth: 180 }}><input className="input" value={x.nombre} onChange={(e) => setLinea(x.key, 'nombre', e.target.value)} /></td>
                    <td><input className="input" style={{ width: 70 }} type="number" min="0" value={x.cantidad} onChange={(e) => setLinea(x.key, 'cantidad', e.target.value)} /></td>
                    <td><input className="input" style={{ width: 110 }} type="number" min="0" value={x.precio} onChange={(e) => setLinea(x.key, 'precio', e.target.value)} /></td>
                    <td style={{ whiteSpace: 'nowrap', fontWeight: 600 }}>{money((Number(x.cantidad) || 0) * (Number(x.precio) || 0))}</td>
                    <td><button className="btn btn-sm" onClick={() => quitar(x.key)} title="Quitar">✕</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {lineas.length > 0 && <button className="btn btn-sm" style={{ alignSelf: 'flex-start' }} onClick={agregarManual}>+ Línea manual</button>}
      </div>

      <div className="row" style={{ gap: 12, alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <div className="card stack" style={{ gap: 8, flex: '1 1 260px' }}>
          <b className="sm">Ajustes</b>
          <label className="row" style={{ gap: 8, alignItems: 'center' }}>
            <span className="sm muted" style={{ width: 90 }}>Descuento %</span>
            <input className="input" style={{ width: 90 }} type="number" min="0" max="100" value={descuentoPct} onChange={(e) => setDescuentoPct(e.target.value)} placeholder="0" />
          </label>
          <label className="row" style={{ gap: 8, alignItems: 'center' }}>
            <input type="checkbox" checked={iva} onChange={(e) => setIva(e.target.checked)} /> <span className="sm">Agregar IVA (19%)</span>
          </label>
          <textarea className="input" rows={2} placeholder="Notas / condiciones (opcional)" value={notas} onChange={(e) => setNotas(e.target.value)} />
        </div>
        <div className="card stack" style={{ gap: 6, flex: '1 1 260px' }}>
          <b className="sm">Totales</b>
          <div className="row between"><span className="muted">Subtotal</span><b>{money(subtotal)}</b></div>
          {descuento > 0 && <div className="row between"><span className="muted">Descuento ({Number(descuentoPct)}%)</span><b style={{ color: 'var(--bad, #d93025)' }}>- {money(descuento)}</b></div>}
          {iva && <><div className="row between"><span className="muted">Neto</span><b>{money(neto)}</b></div><div className="row between"><span className="muted">IVA (19%)</span><b>{money(ivaMonto)}</b></div></>}
          <div className="row between" style={{ borderTop: '1px solid var(--line)', paddingTop: 6, marginTop: 2 }}><span className="strong">TOTAL</span><b style={{ fontSize: 18 }}>{money(total)}</b></div>
          <button className="btn btn-dark" style={{ marginTop: 6 }} onClick={generarPDF}>📄 Descargar PDF</button>
        </div>
      </div>
    </div>
  )
}
