import { useEffect, useMemo, useState } from 'react'
import { get, post } from '../api'

// Atributos que el backend ya arma solo (no se piden en el formulario).
const AUTO = new Set(['Name', 'Description', 'PrimaryCategory', 'SellerSku', 'Price', 'Quantity', 'Variation', 'Brand', 'ConditionType'])

const hijos = (node) => {
  const c = node?.Children
  if (c && typeof c === 'object') {
    const x = c.Category
    return Array.isArray(x) ? x : x ? [x] : []
  }
  return []
}
const opciones = (attr) => {
  const o = attr?.Options?.Option
  const arr = Array.isArray(o) ? o : o ? [o] : []
  return arr.map((x) => x?.Name).filter(Boolean)
}
const esObligatorio = (a) => String(a.isMandatory ?? a.IsMandatory ?? '0') === '1'

export default function FalabellaPublicar({ cuentas }) {
  const cuenta = cuentas?.[0] // de momento, la primera cuenta conectada
  const [productos, setProductos] = useState([])
  const [tree, setTree] = useState(null)
  const [ruta, setRuta] = useState([]) // categorías elegidas nivel a nivel
  const [productoId, setProductoId] = useState('')
  const [marca, setMarca] = useState('')
  const [brands, setBrands] = useState(null) // nombres de marcas válidas de Falabella
  const [attrs, setAttrs] = useState(null) // atributos de la categoría hoja
  const [valores, setValores] = useState({})
  const [busy, setBusy] = useState('')
  const [error, setError] = useState(null)
  const [resultado, setResultado] = useState(null)
  const [feedErrores, setFeedErrores] = useState(null) // errores reales del feed tras publicar

  // Solo productos CON stock en Mercado Libre (no tiene sentido publicar sin stock).
  useEffect(() => { get('/productos').then((d) => setProductos((d || []).filter((p) => (p.stock || 0) > 0))).catch(() => {}) }, [])
  useEffect(() => {
    if (!cuenta) return
    get(`/falabella/categorias?cuenta=${cuenta.id}`)
      .then((d) => setTree(d?.SuccessResponse?.Body?.Categories?.Category || []))
      .catch((e) => setError(e.message))
  }, [cuenta])

  // Niveles del selector en cascada: cada nivel son los hijos del elegido anterior.
  const niveles = useMemo(() => {
    if (!tree) return []
    const out = [tree]
    for (const nodo of ruta) {
      const h = hijos(nodo)
      if (h.length) out.push(h)
      else break
    }
    return out
  }, [tree, ruta])

  const hoja = ruta.length > 0 && hijos(ruta[ruta.length - 1]).length === 0 ? ruta[ruta.length - 1] : null

  const elegirNivel = (i, categoryId) => {
    const nodo = niveles[i].find((n) => n.CategoryId === categoryId)
    const nueva = ruta.slice(0, i)
    if (nodo) nueva.push(nodo)
    setRuta(nueva)
    setAttrs(null); setValores({}); setResultado(null)
  }

  const cargarAtributos = async () => {
    if (!hoja) return
    setBusy('attrs'); setError(null)
    try {
      const d = await get(`/falabella/categorias/atributos?cuenta=${cuenta.id}&categoria=${hoja.CategoryId}`)
      let a = d?.SuccessResponse?.Body?.Attribute || []
      if (!Array.isArray(a)) a = [a]
      setAttrs(a.filter((x) => esObligatorio(x) && !AUTO.has(x.FeedName)))
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const cargarMarcas = () => {
    if (brands !== null) return
    setBrands([]) // evita recargas
    get(`/falabella/marcas?cuenta=${cuenta.id}`)
      .then((d) => {
        let b = d?.SuccessResponse?.Body?.Brands?.Brand || []
        if (!Array.isArray(b)) b = [b]
        setBrands(b.map((x) => x?.Name).filter(Boolean))
      })
      .catch((e) => setError(e.message))
  }
  const marcasSugeridas = useMemo(() => {
    if (!brands || !marca.trim()) return []
    const q = marca.trim().toLowerCase()
    return brands.filter((n) => n.toLowerCase().includes(q)).slice(0, 50)
  }, [brands, marca])

  const publicar = async () => {
    if (!productoId || !hoja) return
    // Valida que todos los atributos obligatorios del formulario estén llenos.
    const faltan = (attrs || []).filter((a) => !(valores[a.FeedName] || '').trim()).map((a) => a.Label || a.FeedName)
    if (faltan.length) { setError('Completa los atributos obligatorios: ' + faltan.join(', ')); return }
    setBusy('pub'); setError(null); setResultado(null); setFeedErrores(null)
    try {
      const atributos = (attrs || [])
        .map((a) => ({ nombre: a.FeedName, valor: (valores[a.FeedName] || '').trim() }))
        .filter((x) => x.valor)
      const r = await post('/falabella/publicar', { producto_id: productoId, cuenta_id: cuenta.id, categoria: hoja.CategoryId, marca, atributos })
      setResultado(r)
      // Falabella procesa el feed asíncrono: consultamos su estado real a los ~3s.
      setTimeout(verEstadoUltimoFeed, 3000)
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const verEstadoUltimoFeed = async () => {
    try {
      const fl = await get(`/falabella/feeds?cuenta=${cuenta.id}`)
      let feeds = fl?.SuccessResponse?.Body?.Feed || []
      if (!Array.isArray(feeds)) feeds = [feeds]
      if (!feeds.length) return
      const fid = feeds[0].Feed
      const fs = await get(`/falabella/feed?cuenta=${cuenta.id}&id=${fid}`)
      const fd = fs?.SuccessResponse?.Body?.FeedDetail || fs?.SuccessResponse?.Body || {}
      let errs = fd?.FeedErrors?.Error || []
      if (!Array.isArray(errs)) errs = errs ? [errs] : []
      setFeedErrores(errs.map((e) => (e.Message || String(e)).replace(/&#039;/g, "'")))
    } catch { /* silencioso */ }
  }

  if (!cuenta) return null

  return (
    <div className="card stack" style={{ gap: 12 }}>
      <div><div className="strong">Publicar un producto en Falabella</div>
        <p className="muted sm" style={{ margin: 0 }}>Elige el producto, su categoría en Falabella y completa los atributos obligatorios.</p></div>

      {error && <div className="alert alert-bad">{error}</div>}

      <label className="stack" style={{ gap: 4 }}><span className="sm">Producto del catálogo</span>
        <select className="input" value={productoId} onChange={(e) => setProductoId(e.target.value)}>
          <option value="">— Elige un producto —</option>
          {productos.map((p) => <option key={p.id} value={p.id}>{p.titulo}</option>)}
        </select>
      </label>

      <div className="stack" style={{ gap: 4 }}>
        <span className="sm">Categoría en Falabella</span>
        {!tree ? <span className="muted sm">Cargando categorías…</span> : niveles.map((nivel, i) => (
          <select key={i} className="input" value={ruta[i]?.CategoryId || ''} onChange={(e) => elegirNivel(i, e.target.value)}>
            <option value="">{i === 0 ? '— Categoría —' : '— Subcategoría —'}</option>
            {nivel.map((n) => <option key={n.CategoryId} value={n.CategoryId}>{n.Name}</option>)}
          </select>
        ))}
        {hoja && <span className="muted xs">Categoría elegida: [{hoja.CategoryId}] {hoja.Name}</span>}
      </div>

      <label className="stack" style={{ gap: 4 }}><span className="sm">Marca (debe existir en Falabella — elige de la lista)</span>
        <input className="input" list="fala-brands" value={marca} onFocus={cargarMarcas}
          onChange={(e) => setMarca(e.target.value)} placeholder={brands === null ? 'Haz clic para cargar marcas…' : 'Escribe para buscar (ej: Bosch)'} />
        <datalist id="fala-brands">{marcasSugeridas.map((n) => <option key={n} value={n} />)}</datalist>
        {brands && brands.length > 0 && <span className="muted xs">{brands.length.toLocaleString('es-CL')} marcas disponibles · escribe para filtrar</span>}
      </label>

      {hoja && !attrs && (
        <div className="row"><button className="btn btn-sm" onClick={cargarAtributos} disabled={busy === 'attrs'}>{busy === 'attrs' ? 'Cargando…' : 'Cargar atributos obligatorios'}</button></div>
      )}

      {attrs && attrs.length > 0 && (
        <div className="stack" style={{ gap: 8 }}>
          <div className="strong sm">Atributos obligatorios</div>
          {attrs.map((a) => {
            const ops = opciones(a)
            return (
              <label key={a.FeedName} className="stack" style={{ gap: 4 }}>
                <span className="sm">{a.Label || a.Name} <span className="muted xs">({a.FeedName})</span></span>
                {ops.length > 0 ? (
                  <select className="input" value={valores[a.FeedName] || ''} onChange={(e) => setValores({ ...valores, [a.FeedName]: e.target.value })}>
                    <option value="">— Elige —</option>
                    {ops.map((o) => <option key={o} value={o}>{o}</option>)}
                  </select>
                ) : (
                  <input className="input" type={a.InputType === 'numberfield' ? 'number' : 'text'} value={valores[a.FeedName] || ''}
                    onChange={(e) => setValores({ ...valores, [a.FeedName]: e.target.value })} placeholder={a.ExampleValue || ''} />
                )}
              </label>
            )
          })}
        </div>
      )}
      {attrs && attrs.length === 0 && <div className="muted sm">Esta categoría no pide atributos extra. Puedes publicar.</div>}

      {resultado && <div className="alert alert-ok">Producto enviado a Falabella. Las <b>imágenes se suben en segundo plano</b> (1-2 min, cuando el producto queda registrado). Revisa el estado en tu Seller Center; recuerda elegir la <b>categoría correcta</b> o Falabella lo rechaza en control de calidad.</div>}
      {feedErrores !== null && (feedErrores.length === 0
        ? <div className="alert alert-ok">✓ Feed procesado sin errores. El producto puede tardar en aparecer o requerir aprobación de Falabella.</div>
        : <div className="alert alert-bad"><b>Falabella rechazó el producto:</b><ul style={{ margin: '6px 0 0', paddingLeft: 18 }}>{feedErrores.map((e, i) => <li key={i}>{e}</li>)}</ul></div>)}

      <div className="row" style={{ justifyContent: 'flex-end' }}>
        <button className="btn btn-accent" onClick={publicar} disabled={busy === 'pub' || !productoId || !hoja || !marca}>
          {busy === 'pub' ? 'Publicando…' : 'Publicar en Falabella'}
        </button>
      </div>
      <p className="muted xs" style={{ margin: 0 }}>Nota: Falabella es producción (sin ambiente de pruebas). Prueba primero con un producto de bajo valor.</p>
    </div>
  )
}
