import { useEffect, useState } from 'react'
import { get, post } from '../api'

// Las respuestas de Paris vienen como { results: [...] } (a veces { data: [...] }).
const asList = (v) => v?.results || v?.data || (Array.isArray(v) ? v : [])

export default function ParisPublicar({ cuenta }) {
  const [familias, setFamilias] = useState(null)
  const [categorias, setCategorias] = useState([])
  const [productos, setProductos] = useState([])
  const [attrDefs, setAttrDefs] = useState([]) // atributos de la familia elegida
  const [attrVals, setAttrVals] = useState({}) // { [attrId]: valor }
  const [imagenes, setImagenes] = useState([''])
  const [loadingFam, setLoadingFam] = useState(false)
  const [error, setError] = useState(null)
  const [result, setResult] = useState(null)
  const [busy, setBusy] = useState(false)
  const [form, setForm] = useState({ name: '', ref_product: '', family_code: '', categoria_id: '', precio: '', oferta: '', producto_id: '' })

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
    setForm((f) => ({ ...f, producto_id: p.id, name: p.titulo || f.name, ref_product: p.ml_item_id || p.sku || f.ref_product, precio: p.precio != null ? String(p.precio) : f.precio }))
    if (p.thumbnail) setImagenes((imgs) => (imgs.some((u) => u) ? imgs : [p.thumbnail]))
  }

  const elegirFamilia = async (famId) => {
    setForm((f) => ({ ...f, family_code: famId, categoria_id: '' }))
    setAttrDefs([]); setAttrVals({}); setCategorias([])
    if (!famId) return
    setLoadingFam(true)
    try {
      const [cats, attrs] = await Promise.all([
        get(`/paris/cuentas/${cuenta.id}/categorias/${famId}`).catch(() => null),
        get(`/paris/cuentas/${cuenta.id}/atributos/${famId}`).catch(() => null),
      ])
      setCategorias(asList(cats))
      setAttrDefs(asList(attrs))
    } catch (e) { setError(e.message) } finally { setLoadingFam(false) }
  }

  const setAttr = (id, v) => setAttrVals((m) => ({ ...m, [id]: v }))
  const setImg = (i, v) => setImagenes((a) => a.map((x, j) => (j === i ? v : x)))
  const addImg = () => setImagenes((a) => [...a, ''])
  const rmImg = (i) => setImagenes((a) => a.filter((_, j) => j !== i))

  const publicar = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null); setResult(null)
    try {
      const atributos = Object.entries(attrVals).filter(([, v]) => v !== '' && v != null).map(([code, value]) => ({ code, value: String(value) }))
      const body = {
        name: form.name,
        ref_product: form.ref_product,
        family_code: form.family_code,
        categoria_id: form.categoria_id,
        precio: Number(form.precio) || 0,
        oferta: form.oferta ? Number(form.oferta) : null,
        atributos,
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
      {result && <div className="alert alert-ok">Producto enviado a Paris. <span className="muted sm">Revisa su aprobación en tu Seller Center.</span></div>}

      <label className="sm muted">Producto del hub (opcional — pre-rellena y habilita el sync de stock)</label>
      <select className="input" value={form.producto_id} onChange={(e) => elegirProducto(e.target.value)}>
        <option value="">— Manual (sin vincular) —</option>
        {productos.map((p) => <option key={p.id} value={p.id}>{p.titulo} · {p.ml_item_id || p.sku}</option>)}
      </select>

      <input className="input" placeholder="Nombre del producto" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} required />
      <input className="input" placeholder="SKU / referencia (ej. el id de Mercado Libre)" value={form.ref_product} onChange={(e) => setForm({ ...form, ref_product: e.target.value })} required />

      <label className="sm muted">Familia</label>
      <select className="input" value={form.family_code} onChange={(e) => elegirFamilia(e.target.value)} required>
        <option value="">{familias == null ? 'Cargando familias…' : (famList.length ? '— Elige una familia —' : 'Sin familias disponibles en tu cuenta')}</option>
        {famList.map((f) => <option key={f.id || f.code} value={f.id || f.code}>{f.name || f.nombre}</option>)}
      </select>

      {form.family_code && (
        <>
          <label className="sm muted">Categoría (subcategoría de la familia)</label>
          <select className="input" value={form.categoria_id} onChange={(e) => setForm({ ...form, categoria_id: e.target.value })} required>
            <option value="">{loadingFam ? 'Cargando categorías…' : (catSelect.length ? '— Elige una categoría —' : 'Sin categorías disponibles')}</option>
            {catSelect.map((c) => <option key={c.id} value={c.id}>{c.path || c.name}</option>)}
          </select>
        </>
      )}

      <div className="row" style={{ gap: 8 }}>
        <input className="input" type="number" placeholder="Precio (CLP)" value={form.precio} onChange={(e) => setForm({ ...form, precio: e.target.value })} required />
        <input className="input" type="number" placeholder="Precio oferta (opcional)" value={form.oferta} onChange={(e) => setForm({ ...form, oferta: e.target.value })} />
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
          <span className="sm muted">Atributos de la familia</span>
          {loadingFam ? (
            <div className="muted sm">Cargando atributos…</div>
          ) : attrDefs.length === 0 ? (
            <div className="muted sm">Esta familia no devolvió atributos (o la API los rechazó). Revisa con el curl de diagnóstico.</div>
          ) : (
            attrDefs.map((a) => {
            const opts = a.attributeOptions || a.options || []
            return (
              <div key={a.id} className="row" style={{ gap: 6, alignItems: 'center' }}>
                <label className="sm" style={{ minWidth: 150 }}>{a.name || a.id}</label>
                {opts.length > 0 ? (
                  <select className="input" value={attrVals[a.id] || ''} onChange={(e) => setAttr(a.id, e.target.value)}>
                    <option value="">—</option>
                    {opts.map((o) => <option key={o.id || o.name} value={o.name || o.id}>{o.name || o.id}</option>)}
                  </select>
                ) : (
                  <input className="input" value={attrVals[a.id] || ''} onChange={(e) => setAttr(a.id, e.target.value)} placeholder="valor" />
                )}
              </div>
            )
          })
          )}
        </div>
      )}

      <button className="btn btn-dark" disabled={busy}>{busy ? 'Publicando…' : 'Publicar en Paris'}</button>
    </form>
  )
}
