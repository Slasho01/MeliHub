import { useEffect, useState } from 'react'
import { get, post } from '../api'

// Las respuestas de Paris vienen como { results: [...] } (a veces { data: [...] }).
const asList = (v) => v?.results || v?.data || (Array.isArray(v) ? v : [])

// Combobox con búsqueda: escribes y filtra las opciones. options: [{value,label}].
function Combo({ options, value, onChange, placeholder }) {
  const [q, setQ] = useState('')
  const [open, setOpen] = useState(false)
  const sel = options.find((o) => o.value === value)
  const text = open ? q : (sel ? sel.label : '')
  const filtered = options.filter((o) => (o.label || '').toLowerCase().includes(q.toLowerCase())).slice(0, 80)
  return (
    <div style={{ position: 'relative' }}>
      <input
        className="input"
        placeholder={placeholder}
        value={text}
        onChange={(e) => { setQ(e.target.value); setOpen(true) }}
        onFocus={() => { setQ(''); setOpen(true) }}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && filtered.length > 0 && (
        <div style={{ position: 'absolute', zIndex: 20, left: 0, right: 0, top: '100%', marginTop: 2, maxHeight: 240, overflowY: 'auto', background: 'var(--surface)', color: 'var(--ink)', border: '1px solid var(--line)', borderRadius: 8, boxShadow: '0 10px 24px rgba(0,0,0,.45)' }}>
          {filtered.map((o) => (
            <div
              key={o.value}
              style={{ padding: '8px 10px', cursor: 'pointer', fontSize: 14, color: 'var(--ink)', borderBottom: '1px solid var(--line)', background: o.value === value ? 'var(--line)' : 'transparent' }}
              onMouseDown={(e) => { e.preventDefault(); onChange(o.value); setOpen(false); setQ('') }}
            >
              {o.label}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

const norm = (s) => (s || '').toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/\([^)]*\)/g, '').trim()
const numOf = (v) => { const m = String(v || '').match(/[\d.,]+/); return m ? m[0].replace(',', '.') : '' }

// ¿atributo obligatorio? Paris mete "OBLIGATORIO" en el nombre (o trae un flag).
const esReq = (a) => /obligatorio/i.test(a.name || '') || a.isRequired || a.required || a.mandatory || a.isMandatory
const tieneOpciones = (a) => (a.attributeOptions || a.options || []).length > 0

// Pre-rellena los atributos de Paris con datos de ML del producto (heurística por nombre/id).
function autoFill(product, parisAttrs) {
  const ml = (product && product.datos && product.datos.attributes) || []
  const byId = (id) => { const x = ml.find((a) => a.id === id); return x ? x.value_name : '' }
  const byName = (pn) => { const x = ml.find((a) => norm(a.name) === pn); return x ? x.value_name : '' }
  const out = {}
  for (const pa of parisAttrs) {
    const pn = norm(pa.name)
    let v = ''
    if (pn.includes('peso') && pn.includes('paquete')) v = numOf(byId('PACKAGE_WEIGHT'))
    else if (pn.includes('ancho') && pn.includes('paquete')) v = numOf(byId('PACKAGE_WIDTH'))
    else if (pn.includes('alto') && pn.includes('paquete')) v = numOf(byId('PACKAGE_HEIGHT'))
    else if (pn.includes('largo') && pn.includes('paquete')) v = numOf(byId('PACKAGE_LENGTH'))
    else if (pn.includes('marca')) v = byId('BRAND') || byName('marca')
    else if (pn.includes('modelo')) v = byId('MODEL') || byName('modelo')
    else if (pn.includes('color')) v = byId('COLOR') || byName('color')
    else if (pn.includes('talla') || pn.includes('talle') || pn.includes('tamano')) v = byId('SIZE') || byName('talla') || byName('talle')
    else v = byName(pn)
    if (!v) continue
    const opts = pa.attributeOptions || pa.options || []
    if (opts.length > 0) {
      const match = opts.find((o) => norm(o.name) === norm(v))
      if (match) out[pa.id] = match.id // en enum, Paris quiere el ID de la opción (no el nombre)
    } else {
      out[pa.id] = String(v)
    }
  }
  return out
}

export default function ParisPublicar({ cuenta }) {
  const [familias, setFamilias] = useState(null)
  const [categorias, setCategorias] = useState([])
  const [productos, setProductos] = useState([])
  const [attrDefs, setAttrDefs] = useState([]) // atributos de PRODUCTO
  const [attrVals, setAttrVals] = useState({}) // { [attrId]: valor }
  const [varAttrDefs, setVarAttrDefs] = useState([]) // atributos de VARIANTE (color/talla)
  const [varAttrVals, setVarAttrVals] = useState({})
  const [imagenes, setImagenes] = useState([''])
  const [loadingFam, setLoadingFam] = useState(false)
  const [showOpc, setShowOpc] = useState(false)
  const [error, setError] = useState(null)
  const [result, setResult] = useState(null)
  const [busy, setBusy] = useState(false)
  const [form, setForm] = useState({ name: '', ref_product: '', family_code: '', categoria_id: '', precio: '', oferta: '', stock: '', producto_id: '' })

  useEffect(() => {
    setFamilias(null); setError(null); setAttrDefs([]); setAttrVals({}); setCategorias([])
    setForm((f) => ({ ...f, family_code: '', categoria_id: '' }))
    get(`/paris/cuentas/${cuenta.id}/familias`).then(setFamilias).catch((e) => setError(e.message))
  }, [cuenta.id])

  useEffect(() => {
    get('/productos').then((d) => setProductos(Array.isArray(d) ? d : (d?.data || d?.productos || []))).catch(() => {})
  }, [])

  const famList = asList(familias)
  const catSelect = categorias.filter((c) => c.isSelectable !== false)

  const elegirProducto = (id) => {
    const p = productos.find((x) => x.id === id)
    if (!p) { setForm((f) => ({ ...f, producto_id: '' })); return }
    setForm((f) => ({ ...f, producto_id: p.id, name: p.titulo || f.name, ref_product: p.ml_item_id || p.sku || f.ref_product, precio: p.precio != null ? String(p.precio) : f.precio, stock: p.stock != null ? String(p.stock) : f.stock }))
    // Trae TODAS las imágenes de ML (galería), no solo el thumbnail.
    const pics = (p.datos && p.datos.pictures) || []
    const imgs = pics.map((x) => x.secure_url || x.url).filter(Boolean)
    if (!imgs.length && p.thumbnail) imgs.push(p.thumbnail)
    setImagenes(imgs.length ? imgs : [''])
    // Si ya hay una familia con atributos cargados, autorrellena desde ML.
    if (attrDefs.length) setAttrVals(autoFill(p, attrDefs))
    if (varAttrDefs.length) setVarAttrVals(autoFill(p, varAttrDefs))
  }

  const elegirFamilia = async (famId) => {
    setForm((f) => ({ ...f, family_code: famId, categoria_id: '' }))
    setAttrDefs([]); setAttrVals({}); setVarAttrDefs([]); setVarAttrVals({}); setCategorias([])
    if (!famId) return
    setLoadingFam(true)
    try {
      const [cats, attrs, vattrs] = await Promise.all([
        get(`/paris/cuentas/${cuenta.id}/categorias/${famId}`).catch(() => null),
        get(`/paris/cuentas/${cuenta.id}/atributos/${famId}`).catch(() => null),
        get(`/paris/cuentas/${cuenta.id}/atributos-variante/${famId}`).catch(() => null),
      ])
      setCategorias(asList(cats))
      const attrsList = asList(attrs)
      const varList = asList(vattrs)
      setAttrDefs(attrsList)
      setVarAttrDefs(varList)
      // Autocompleta desde el producto de ML elegido (si hay).
      const prod = productos.find((x) => x.id === form.producto_id)
      setAttrVals(prod ? autoFill(prod, attrsList) : {})
      setVarAttrVals(prod ? autoFill(prod, varList) : {})
    } catch (e) { setError(e.message) } finally { setLoadingFam(false) }
  }

  const setAttr = (id, v) => setAttrVals((m) => ({ ...m, [id]: v }))
  const setImg = (i, v) => setImagenes((a) => a.map((x, j) => (j === i ? v : x)))
  const addImg = () => setImagenes((a) => [...a, ''])
  const rmImg = (i) => setImagenes((a) => a.filter((_, j) => j !== i))

  const setVarAttr = (id, v) => setVarAttrVals((m) => ({ ...m, [id]: v }))
  const renderAttr = (a, vals, setter, forceReq) => {
    const opts = a.attributeOptions || a.options || []
    const req = forceReq || esReq(a)
    return (
      <div key={a.id} className="row" style={{ gap: 6, alignItems: 'center' }}>
        <label className="sm" style={{ minWidth: 180 }}>{a.name || a.id}{req && <span style={{ color: 'var(--accent)' }}> *</span>}</label>
        {opts.length > 0 ? (
          <select className="input" value={vals[a.id] || ''} onChange={(e) => setter(a.id, e.target.value)}>
            <option value="">—</option>
            {opts.map((o) => <option key={o.id || o.name} value={o.id || o.name}>{o.name || o.id}</option>)}
          </select>
        ) : (
          <input className="input" value={vals[a.id] || ''} onChange={(e) => setter(a.id, e.target.value)} placeholder="valor" />
        )}
      </div>
    )
  }
  // Siempre visibles: obligatorios + los de lista (desplegables, suelen ser requeridos).
  const importantes = attrDefs.filter((a) => esReq(a) || tieneOpciones(a))
  const opcionales = attrDefs.filter((a) => !(esReq(a) || tieneOpciones(a)))

  const publicar = async (e) => {
    e.preventDefault()
    if (!form.family_code) { setError('Elige una familia'); return }
    if (!form.categoria_id) { setError('Elige una categoría'); return }
    setBusy(true); setError(null); setResult(null)
    try {
      const toPairs = (obj) => Object.entries(obj).filter(([, v]) => v !== '' && v != null).map(([code, value]) => ({ code, value: String(value) }))
      const atributos = toPairs(attrVals)
      const variante_atributos = toPairs(varAttrVals)
      const body = {
        name: form.name,
        ref_product: form.ref_product,
        family_code: form.family_code,
        categoria_id: form.categoria_id,
        precio: Number(form.precio) || 0,
        stock: Number(form.stock) || 0,
        atributos,
        variante_atributos,
        imagenes: imagenes.filter((u) => u && u.trim()),
        producto_id: form.producto_id || null,
      }
      const r = await post(`/paris/cuentas/${cuenta.id}/producto`, body)
      setResult(r)
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  return (
    <form className="stack" style={{ gap: 10 }} onSubmit={publicar}>
      {error && <div className="alert alert-bad">{error}</div>}
      {result && (
        <div className="alert alert-ok">
          {result.ya_existia ? '✅ El producto ya está publicado en Paris.' : '✅ Producto creado en Paris.'}
          {result.precio_ok ? ' Precio aplicado.' : ' El precio quedó pendiente — ajústalo en el Seller Center.'}
          {' '}<span className="muted sm">Revisa su aprobación en el Seller Center.</span>
        </div>
      )}

      <label className="sm muted">Producto del hub (opcional — pre-rellena y habilita el sync de stock)</label>
      <select className="input" value={form.producto_id} onChange={(e) => elegirProducto(e.target.value)}>
        <option value="">— Manual (sin vincular) —</option>
        {productos.map((p) => <option key={p.id} value={p.id}>{p.titulo} · {p.ml_item_id || p.sku}</option>)}
      </select>

      <input className="input" placeholder="Nombre del producto" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} required />
      <input className="input" placeholder="SKU / referencia (ej. el id de Mercado Libre)" value={form.ref_product} onChange={(e) => setForm({ ...form, ref_product: e.target.value })} required />

      <label className="sm muted">Familia {familias == null && <span>(cargando…)</span>}</label>
      <Combo
        placeholder={famList.length ? 'Escribe para buscar familia…' : 'Sin familias disponibles en tu cuenta'}
        options={famList.map((f) => ({ value: f.id || f.code, label: f.name || f.nombre }))}
        value={form.family_code}
        onChange={elegirFamilia}
      />

      {form.family_code && (
        <>
          <label className="sm muted">Categoría (subcategoría) {loadingFam && <span>(cargando…)</span>}</label>
          <Combo
            placeholder={catSelect.length ? 'Escribe para buscar categoría…' : 'Sin categorías disponibles'}
            options={catSelect.map((c) => ({ value: c.id, label: c.path || c.name }))}
            value={form.categoria_id}
            onChange={(v) => setForm((f) => ({ ...f, categoria_id: v }))}
          />
        </>
      )}

      <div className="row" style={{ gap: 8 }}>
        <input className="input" type="number" placeholder="Precio (CLP)" value={form.precio} onChange={(e) => setForm({ ...form, precio: e.target.value })} required />
        <input className="input" type="number" placeholder="Stock" value={form.stock} onChange={(e) => setForm({ ...form, stock: e.target.value })} />
      </div>

      <div className="stack" style={{ gap: 6 }}>
        <div className="row between"><span className="sm muted">Imágenes (URL) — Paris exige al menos una</span><button type="button" className="btn btn-sm" onClick={addImg}>+ Imagen</button></div>
        {imagenes.map((u, i) => (
          <div key={i} className="row" style={{ gap: 6 }}>
            <input className="input" placeholder="https://…/imagen.jpg" value={u} onChange={(e) => setImg(i, e.target.value)} />
            {imagenes.length > 1 && <button type="button" className="btn btn-sm" onClick={() => rmImg(i)}>✕</button>}
          </div>
        ))}
      </div>

      {form.family_code && (
        <div className="stack" style={{ gap: 6 }}>
          <span className="sm muted">Atributos de la familia{form.producto_id ? ' · autocompletados desde Mercado Libre donde se pudo' : ''}</span>
          {loadingFam ? (
            <div className="muted sm">Cargando atributos…</div>
          ) : attrDefs.length === 0 ? (
            <div className="muted sm">Esta familia no devolvió atributos.</div>
          ) : (
            <>
              {importantes.map((a) => renderAttr(a, attrVals, setAttr))}
              {opcionales.length > 0 && (
                <>
                  <button type="button" className="btn btn-sm" style={{ alignSelf: 'flex-start' }} onClick={() => setShowOpc((v) => !v)}>
                    {showOpc ? 'Ocultar' : 'Mostrar'} más atributos opcionales ({opcionales.length})
                  </button>
                  {showOpc && opcionales.map((a) => renderAttr(a, attrVals, setAttr))}
                </>
              )}
            </>
          )}
        </div>
      )}

      {form.family_code && varAttrDefs.length > 0 && (
        <div className="stack" style={{ gap: 6 }}>
          <span className="sm muted">Atributos de la variante (requeridos){form.producto_id ? ' · autocompletados desde ML donde se pudo' : ''}</span>
          {varAttrDefs.map((a) => renderAttr(a, varAttrVals, setVarAttr, true))}
        </div>
      )}

      <button className="btn btn-dark" disabled={busy}>{busy ? 'Publicando…' : 'Publicar en Paris'}</button>
    </form>
  )
}
