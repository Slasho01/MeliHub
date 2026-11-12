import { useEffect, useMemo, useState } from 'react'
import { get, post, del } from '../api'

export default function Shopify() {
  const [cuentas, setCuentas] = useState(null)
  const [form, setForm] = useState({ shop_domain: '', access_token: '', nombre: '' })
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)

  // Publicar
  const [productos, setProductos] = useState([])
  const [buscar, setBuscar] = useState('')
  const [sel, setSel] = useState(null)
  const [cuentaPub, setCuentaPub] = useState('')
  const [pub, setPub] = useState(false)

  const cargar = () => { setCuentas(null); get('/shopify/cuentas').then(setCuentas).catch((e) => setError(e.message)) }
  useEffect(() => { cargar(); get('/productos').then((d) => setProductos(Array.isArray(d) ? d : (d?.data || []))).catch(() => {}) }, [])

  const conectar = async (e) => {
    e.preventDefault()
    if (!form.shop_domain.trim() || !form.access_token.trim()) { setError('Ingresa el dominio y el Admin API access token.'); return }
    setBusy(true); setError(''); setMsg('')
    try { await post('/shopify/cuentas', form); setMsg('Tienda conectada.'); setForm({ shop_domain: '', access_token: '', nombre: '' }); cargar() }
    catch (err) { setError(err.message) } finally { setBusy(false) }
  }
  const desconectar = async (c) => {
    if (!window.confirm(`¿Desconectar ${c.nombre || c.shop_domain}?`)) return
    try { await del(`/shopify/cuentas/${c.id}`); cargar() } catch (e) { setError(e.message) }
  }

  const resultados = useMemo(() => {
    const q = buscar.trim().toLowerCase()
    if (!q) return []
    return productos.filter((p) => (p.titulo || '').toLowerCase().includes(q) || (p.ml_item_id || '').toLowerCase().includes(q)).slice(0, 8)
  }, [buscar, productos])

  const publicar = async () => {
    if (!sel || !cuentaPub) { setError('Elige un producto y una tienda.'); return }
    setPub(true); setError(''); setMsg('')
    try {
      const r = await post('/shopify/publicar', { cuenta_id: cuentaPub, producto_id: sel.id })
      setMsg(`"${sel.titulo}" publicado en Shopify${r.stock_ok ? ' (con stock)' : ''}.`)
      setSel(null); setBuscar('')
    } catch (e) { setError(e.message) } finally { setPub(false) }
  }

  const list = cuentas || []

  return (
    <div className="stack">
      <div><h1 style={{ margin: 0 }}>Shopify</h1><p className="muted sm" style={{ margin: 0 }}>Conecta tu tienda Shopify y publica productos de tu inventario.</p></div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="row" style={{ gap: 12, alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <form className="card stack" style={{ gap: 10, flex: '1 1 340px' }} onSubmit={conectar}>
          <b className="sm">Conectar tienda</b>
          <label>Dominio de la tienda
            <input className="input" placeholder="mitienda.myshopify.com" value={form.shop_domain} onChange={(e) => setForm({ ...form, shop_domain: e.target.value })} />
          </label>
          <label>Admin API access token
            <input className="input" type="password" autoComplete="off" placeholder="shpat_..." value={form.access_token} onChange={(e) => setForm({ ...form, access_token: e.target.value })} />
          </label>
          <label>Nombre (opcional)
            <input className="input" placeholder="Mi tienda" value={form.nombre} onChange={(e) => setForm({ ...form, nombre: e.target.value })} />
          </label>
          <button className="btn btn-dark" disabled={busy}>{busy ? 'Conectando…' : 'Conectar'}</button>
          <p className="muted xs" style={{ margin: 0 }}>En Shopify: Configuración → Apps → <b>Desarrollar apps</b> → crea una app, dale permisos de productos/inventario/pedidos y copia el <b>Admin API access token</b>.</p>
        </form>

        <div className="card stack" style={{ gap: 8, flex: '1 1 280px' }}>
          <b className="sm">Tiendas conectadas</b>
          {cuentas == null ? <div className="muted sm">Cargando…</div> : list.length === 0 ? (
            <div className="muted sm">Aún no conectas ninguna tienda.</div>
          ) : list.map((c) => (
            <div key={c.id} className="row between" style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
              <div><b>{c.nombre || c.shop_domain}</b><div className="muted xs">{c.shop_domain}</div></div>
              <button className="btn btn-sm" onClick={() => desconectar(c)}>Desconectar</button>
            </div>
          ))}
        </div>
      </div>

      {list.length > 0 && (
        <div className="card stack" style={{ gap: 10 }}>
          <b className="sm">Publicar un producto</b>
          <div style={{ position: 'relative' }}>
            <input className="input" placeholder="Buscar un producto del inventario…" value={sel ? sel.titulo : buscar} onChange={(e) => { setSel(null); setBuscar(e.target.value) }} />
            {!sel && resultados.length > 0 && (
              <div style={{ position: 'absolute', zIndex: 20, left: 0, right: 0, top: '100%', marginTop: 2, maxHeight: 260, overflowY: 'auto', background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 8, boxShadow: '0 10px 24px rgba(0,0,0,.45)' }}>
                {resultados.map((p) => (
                  <div key={p.id} onMouseDown={(e) => { e.preventDefault(); setSel(p); setBuscar('') }} style={{ padding: '8px 10px', cursor: 'pointer', display: 'flex', gap: 8, alignItems: 'center', borderBottom: '1px solid var(--line)' }}>
                    {p.thumbnail && <img src={p.thumbnail} alt="" width="30" height="30" style={{ borderRadius: 5, objectFit: 'cover' }} />}
                    <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 13 }}>{p.titulo}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
          <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
            <select className="input" style={{ flex: 1, minWidth: 180 }} value={cuentaPub} onChange={(e) => setCuentaPub(e.target.value)}>
              <option value="">Elige la tienda…</option>
              {list.map((c) => <option key={c.id} value={c.id}>{c.nombre || c.shop_domain}</option>)}
            </select>
            <button className="btn btn-accent" disabled={pub || !sel || !cuentaPub} onClick={publicar}>{pub ? 'Publicando…' : 'Publicar en Shopify'}</button>
          </div>
          <span className="muted xs">Crea el producto en tu tienda con título, descripción, fotos, precio (tu precio o el de ML) y stock.</span>
        </div>
      )}
    </div>
  )
}
