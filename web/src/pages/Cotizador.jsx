import { useEffect, useMemo, useState } from 'react'
import { get, put, post, money } from '../api'

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]))
const hoy = () => new Date().toLocaleDateString('es-CL', { day: '2-digit', month: '2-digit', year: 'numeric' })
const nroDefault = () => { const d = new Date(); const p = (n) => String(n).padStart(2, '0'); return `COT-${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}` }
const LS = 'melihub_cotizador_empresa'

export default function Cotizador() {
  const [productos, setProductos] = useState([])
  const [empresa, setEmpresa] = useState({ razon_social: '', rut: '', direccion: '', telefono: '', email: '' })
  const [logo, setLogo] = useState('')
  const [subiendo, setSubiendo] = useState(false)
  const [cliente, setCliente] = useState({ nombre: '', rut: '', email: '' })
  const [numero, setNumero] = useState(nroDefault())
  const [lineas, setLineas] = useState([])
  const [descuentoPct, setDescuentoPct] = useState('')
  const [validez, setValidez] = useState('15')
  const [iva, setIva] = useState(false)
  const [notas, setNotas] = useState('')
  const [buscar, setBuscar] = useState('')
  const [dteCuentas, setDteCuentas] = useState([])
  const [tipoDte, setTipoDte] = useState(39)
  const [emitiendo, setEmitiendo] = useState(false)
  const [emitRes, setEmitRes] = useState(null)
  const [error, setError] = useState('')

  useEffect(() => {
    get('/productos').then((d) => setProductos(Array.isArray(d) ? d : (d?.data || []))).catch(() => {})
    get('/cotizador/logo').then((r) => setLogo(r?.logo || '')).catch(() => {})
    get('/dte/cuentas').then((d) => setDteCuentas(Array.isArray(d) ? d : [])).catch(() => {})
    let guardado = null
    try { guardado = JSON.parse(localStorage.getItem(LS) || 'null') } catch { /* ignore */ }
    if (guardado) setEmpresa((e) => ({ ...e, ...guardado }))
    else get('/facturacion').then((f) => { if (f) setEmpresa((e) => ({ ...e, razon_social: f.razon_social || '', rut: f.rut || '', direccion: f.direccion || '', telefono: f.telefono || '' })) }).catch(() => {})
  }, [])

  const setEmp = (k, v) => setEmpresa((e) => { const n = { ...e, [k]: v }; try { localStorage.setItem(LS, JSON.stringify(n)) } catch { /* ignore */ } return n })

  const onLogo = (file) => {
    if (!file) return
    if (!file.type.startsWith('image/')) { setError('El logo debe ser una imagen (PNG/JPG).'); return }
    setError(''); setSubiendo(true)
    const reader = new FileReader()
    reader.onload = () => {
      const img = new Image()
      img.onload = async () => {
        // Reescala a máx 360px de ancho para mantener el archivo liviano.
        const maxW = 360, scale = Math.min(1, maxW / img.width)
        const cv = document.createElement('canvas')
        cv.width = Math.round(img.width * scale); cv.height = Math.round(img.height * scale)
        cv.getContext('2d').drawImage(img, 0, 0, cv.width, cv.height)
        const data = cv.toDataURL('image/png')
        setLogo(data)
        try { await put('/cotizador/logo', { logo: data }) } catch (e) { setError(e.message) } finally { setSubiendo(false) }
      }
      img.onerror = () => { setError('No se pudo leer la imagen.'); setSubiendo(false) }
      img.src = reader.result
    }
    reader.onerror = () => { setError('No se pudo leer el archivo.'); setSubiendo(false) }
    reader.readAsDataURL(file)
  }
  const quitarLogo = async () => { setLogo(''); try { await put('/cotizador/logo', { logo: null }) } catch (e) { setError(e.message) } }

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

  const emitir = async () => {
    if (!lineas.length) { setError('Agrega productos primero.'); return }
    if (!dteCuentas.length) return
    setEmitiendo(true); setError(''); setEmitRes(null)
    try {
      const r = await post('/dte/emitir', {
        cuenta_id: dteCuentas[0].id,
        tipo: Number(tipoDte),
        receptor: { rut: cliente.rut, razon_social: cliente.nombre },
        detalle: lineas.map((x) => ({ nombre: x.nombre || 'Ítem', cantidad: Number(x.cantidad) || 0, precio: Number(x.precio) || 0 })),
        descuento_pct: Number(descuentoPct) || 0,
      })
      setEmitRes(r)
    } catch (e) { setError(e.message) } finally { setEmitiendo(false) }
  }

  const generarPDF = () => {
    if (!lineas.length) { setError('Agrega al menos un producto a la cotización.'); return }
    setError('')
    const filas = lineas.map((x, i) => {
      const tot = (Number(x.cantidad) || 0) * (Number(x.precio) || 0)
      return `<tr class="${i % 2 ? 'alt' : ''}"><td>${esc(x.nombre)}</td><td class="c">${Number(x.cantidad) || 0}</td><td class="r">${money(x.precio)}</td><td class="r b">${money(tot)}</td></tr>`
    }).join('')
    const totLine = (lbl, val, cls = '') => `<tr class="${cls}"><td class="r k">${lbl}</td><td class="r v">${val}</td></tr>`
    const totales = [
      totLine('Subtotal', money(subtotal)),
      descuento > 0 ? totLine(`Descuento (${Number(descuentoPct)}%)`, '- ' + money(descuento)) : '',
      iva ? totLine('Neto', money(neto)) + totLine('IVA (19%)', money(ivaMonto)) : '',
      totLine('TOTAL', money(total), 'grand'),
    ].join('')
    const html = `<!doctype html><html lang="es"><head><meta charset="utf-8"><title>Cotización ${esc(numero)} · ${esc(empresa.razon_social)}</title>
<style>
  :root { --navy:#13233b; --accent:#2a6f97; --soft:#f3f6fa; --line:#dfe4ea; --ink:#1f2733; --mut:#6b7280; }
  * { box-sizing: border-box; } html,body { margin:0; }
  body { font-family: -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; color: var(--ink); font-size: 13px; line-height: 1.45; }
  .doc { max-width: 820px; margin: 0 auto; padding: 40px 44px; }
  .top { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; }
  .brand { display: flex; gap: 16px; align-items: center; }
  .brand img { max-height: 64px; max-width: 200px; object-fit: contain; }
  .brand .name { font-size: 20px; font-weight: 800; color: var(--navy); letter-spacing: -.01em; }
  .brand .det { color: var(--mut); font-size: 11.5px; line-height: 1.5; margin-top: 3px; }
  .qbox { text-align: right; min-width: 190px; }
  .qbox .tag { display:inline-block; background: var(--navy); color:#fff; font-weight:700; letter-spacing:.12em; font-size:12px; padding:7px 16px; border-radius:6px; }
  .qbox .meta { margin-top: 8px; color: var(--mut); font-size: 12px; line-height: 1.6; }
  .qbox .meta b { color: var(--ink); }
  .rule { height: 3px; background: var(--accent); border-radius: 3px; margin: 18px 0 20px; }
  .para { background: var(--soft); border: 1px solid var(--line); border-radius: 8px; padding: 12px 16px; }
  .para .lbl { font-size: 10.5px; text-transform: uppercase; letter-spacing: .08em; color: var(--mut); font-weight: 700; margin-bottom: 4px; }
  .para .who { font-size: 14px; font-weight: 700; }
  .para .sub { color: var(--mut); font-size: 12px; }
  table.items { width: 100%; border-collapse: collapse; margin-top: 20px; }
  table.items th { background: var(--navy); color: #fff; text-align: left; font-size: 11px; text-transform: uppercase; letter-spacing: .05em; padding: 10px 12px; }
  table.items th.c { text-align:center; } table.items th.r { text-align:right; }
  table.items td { padding: 10px 12px; border-bottom: 1px solid var(--line); vertical-align: top; }
  table.items tr.alt td { background: #fafbfc; }
  td.c { text-align:center; } td.r { text-align:right; white-space:nowrap; } td.b { font-weight:700; }
  .totwrap { display:flex; justify-content:flex-end; margin-top: 8px; }
  table.tot { border-collapse: collapse; min-width: 280px; }
  table.tot td { padding: 7px 12px; } table.tot td.k { color: var(--mut); } table.tot td.v { text-align:right; font-weight:600; white-space:nowrap; }
  table.tot tr.grand td { background: var(--navy); color:#fff; font-size: 16px; font-weight: 800; border-radius: 4px; }
  table.tot tr.grand td.k { color:#fff; }
  .notas { margin-top: 24px; background: var(--soft); border-left: 3px solid var(--accent); border-radius: 0 6px 6px 0; padding: 10px 14px; color: var(--ink); font-size: 12px; white-space: pre-wrap; }
  .foot { margin-top: 28px; padding-top: 12px; border-top: 1px solid var(--line); color: var(--mut); font-size: 11px; display:flex; justify-content:space-between; gap:12px; }
  @media print { .doc { padding: 10px 8px; } body { -webkit-print-color-adjust: exact; print-color-adjust: exact; } }
</style></head><body><div class="doc">
  <div class="top">
    <div class="brand">
      ${logo ? `<img src="${logo}" alt="logo">` : ''}
      <div><div class="name">${esc(empresa.razon_social) || 'Mi Empresa'}</div>
        <div class="det">${empresa.rut ? 'RUT ' + esc(empresa.rut) + '<br>' : ''}${esc(empresa.direccion)}${empresa.direccion ? '<br>' : ''}${[empresa.telefono, empresa.email].filter(Boolean).map(esc).join(' · ')}</div>
      </div>
    </div>
    <div class="qbox"><span class="tag">COTIZACIÓN</span>
      <div class="meta">N° <b>${esc(numero)}</b><br>Fecha: <b>${hoy()}</b></div>
    </div>
  </div>
  <div class="rule"></div>
  <div class="para"><div class="lbl">Preparado para</div>
    <div class="who">${esc(cliente.nombre) || '—'}</div>
    <div class="sub">${[cliente.rut ? 'RUT ' + esc(cliente.rut) : '', esc(cliente.email)].filter(Boolean).join(' · ')}</div>
  </div>
  <table class="items">
    <thead><tr><th>Descripción</th><th class="c">Cant.</th><th class="r">P. unitario</th><th class="r">Total</th></tr></thead>
    <tbody>${filas}</tbody>
  </table>
  <div class="totwrap"><table class="tot">${totales}</table></div>
  ${notas.trim() ? `<div class="notas">${esc(notas)}</div>` : ''}
  <div class="foot"><span>${esc(empresa.razon_social)}</span><span>Precios en CLP${String(validez).trim() ? ' · Validez ' + esc(validez) + ' días' : ''} salvo indicación contraria.</span></div>
</div></body></html>`
    const win = window.open('', '_blank')
    if (!win) { setError('El navegador bloqueó la ventana. Permite pop-ups para descargar el PDF.'); return }
    win.document.write(html)
    win.document.close()
    win.focus()
    setTimeout(() => win.print(), 350)
  }

  return (
    <div className="stack">
      <div>
        <h1 style={{ margin: 0 }}>Cotizador</h1>
        <p className="muted sm" style={{ margin: 0 }}>Arma una cotización con productos de tu inventario, aplica descuento y descárgala en PDF con el logo de tu empresa.</p>
      </div>
      {error && <div className="alert alert-bad">{error}</div>}

      <div className="row" style={{ gap: 12, alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <div className="card stack" style={{ gap: 8, flex: '1 1 320px' }}>
          <b className="sm">Tu empresa</b>
          <div className="row" style={{ gap: 10, alignItems: 'center' }}>
            <div style={{ width: 66, height: 66, borderRadius: 10, border: '1px dashed var(--line)', background: 'var(--bg)', display: 'grid', placeItems: 'center', overflow: 'hidden', flexShrink: 0 }}>
              {logo ? <img src={logo} alt="logo" style={{ maxWidth: '100%', maxHeight: '100%', objectFit: 'contain' }} /> : <span className="muted xs">Logo</span>}
            </div>
            <div className="stack" style={{ gap: 4 }}>
              <label className="btn btn-sm" style={{ cursor: 'pointer', margin: 0 }}>
                {subiendo ? 'Subiendo…' : logo ? 'Cambiar logo' : 'Subir logo'}
                <input type="file" accept="image/*" style={{ display: 'none' }} onChange={(e) => onLogo(e.target.files?.[0])} />
              </label>
              {logo && <button className="link-btn" onClick={quitarLogo}>Quitar</button>}
              <span className="muted xs">PNG/JPG · se guarda en tu cuenta</span>
            </div>
          </div>
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
          <label className="row" style={{ gap: 8, alignItems: 'center' }}>
            <span className="sm muted" style={{ width: 72 }}>N° cotización</span>
            <input className="input" value={numero} onChange={(e) => setNumero(e.target.value)} />
          </label>
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
            <span className="sm muted" style={{ width: 90 }}>Validez (días)</span>
            <input className="input" style={{ width: 90 }} type="number" min="0" value={validez} onChange={(e) => setValidez(e.target.value)} placeholder="15" />
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

      {dteCuentas.length > 0 && (
        <div className="card stack" style={{ gap: 8 }}>
          <b className="sm">Facturar en el SII</b>
          <div className="row" style={{ gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
            <select className="input" style={{ width: 180 }} value={tipoDte} onChange={(e) => setTipoDte(e.target.value)}>
              <option value={39}>Boleta electrónica</option>
              <option value={33}>Factura electrónica</option>
            </select>
            <button className="btn btn-accent" disabled={emitiendo || !lineas.length} onClick={emitir}>
              {emitiendo ? 'Emitiendo…' : `Emitir ${Number(tipoDte) === 33 ? 'factura' : 'boleta'}`}
            </button>
            <span className="muted xs">Emite con {dteCuentas[0].razon_social || dteCuentas[0].rut_emisor}{dteCuentas[0].certificacion ? ' · certificación' : ''}</span>
          </div>
          {emitRes && (
            <div className="alert alert-ok">
              Documento emitido{emitRes.folio ? ` (folio ${emitRes.folio})` : ''}.{' '}
              {emitRes.pdf && <a href={emitRes.pdf} target="_blank" rel="noreferrer">Ver PDF</a>}
            </div>
          )}
          <span className="muted xs">La <b>factura</b> puede requerir giro, dirección y comuna del cliente (se agregan cuando validemos la emisión).</span>
        </div>
      )}
    </div>
  )
}
