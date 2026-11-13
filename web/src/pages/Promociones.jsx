import { useEffect, useMemo, useState } from 'react'
import { get, post, money } from '../api'

const fecha = (iso) => { if (!iso) return ''; const d = new Date(iso); return isNaN(d) ? '' : d.toLocaleDateString('es-CL', { day: '2-digit', month: 'short', year: '2-digit' }) }
const tipoTxt = (t) => ({
  DEAL: 'Oferta', MARKETPLACE_CAMPAIGN: 'Campaña ML', PRICE_DISCOUNT: 'Descuento', SELLER_CAMPAIGN: 'Campaña propia',
  DOD: 'Oferta del día', LIGHTNING: 'Oferta relámpago', PRE_NEGOTIATED: 'Pre-negociada', SMART: 'Descuento inteligente',
}[t] || t || 'Promoción')
const estadoInfo = (s) => {
  const x = String(s || '').toLowerCase()
  if (['started', 'active'].includes(x)) return ['Activa', 'var(--ok)']
  if (['pending', 'candidate'].includes(x)) return ['Disponible', 'var(--warn)']
  if (['finished', 'closed'].includes(x)) return ['Finalizada', 'var(--muted)']
  return [s || '—', 'var(--muted)']
}
const itemsDe = (d) => d?.results || d?.data || (Array.isArray(d) ? d : [])
const participando = (st) => ['started', 'active', 'approved', 'applied'].includes(String(st || '').toLowerCase())

export default function Promociones() {
  const [cuenta, setCuenta] = useState('')
  const [cuentas, setCuentas] = useState([])
  const [rows, setRows] = useState(null)
  const [titulos, setTitulos] = useState({})
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [cargando, setCargando] = useState(false)
  const [abierto, setAbierto] = useState(null)
  const [items, setItems] = useState({}) // promoId -> items[]
  const [precios, setPrecios] = useState({}) // itemId -> deal price editado
  const [busy, setBusy] = useState('')

  useEffect(() => {
    get('/ml/cuentas').then(setCuentas).catch(() => {})
    get('/productos').then((d) => { const m = {}; (Array.isArray(d) ? d : (d?.data || [])).forEach((p) => { m[p.ml_item_id] = p.titulo }); setTitulos(m) }).catch(() => {})
  }, [])

  const cargar = () => {
    setCargando(true); setError('')
    const qs = cuenta ? `?cuenta=${cuenta}` : ''
    get(`/ml/promociones${qs}`).then(setRows).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const abrir = async (p) => {
    const k = `${p.cuenta_id}-${p.id}`
    if (abierto === k) { setAbierto(null); return }
    setAbierto(k)
    if (!items[k]) {
      try { const d = await get(`/ml/promociones/items?cuenta=${p.cuenta_id}&promotion_id=${encodeURIComponent(p.id)}&tipo=${encodeURIComponent(p.tipo)}`); setItems((m) => ({ ...m, [k]: itemsDe(d) })) }
      catch (e) { setError(e.message) }
    }
  }

  const accion = async (p, it, quitar) => {
    const k = `${p.cuenta_id}-${p.id}`
    const itemId = it.id || it.item_id
    setBusy(itemId); setError(''); setMsg('')
    try {
      if (quitar) {
        await post('/ml/promociones/quitar', { cuenta_id: p.cuenta_id, item_id: itemId, promotion_id: p.id, tipo: p.tipo })
        setMsg('Producto quitado de la promoción.')
      } else {
        const dp = Number(precios[itemId]) || undefined
        await post('/ml/promociones/agregar', { cuenta_id: p.cuenta_id, item_id: itemId, promotion_id: p.id, tipo: p.tipo, deal_price: dp })
        setMsg('Producto agregado a la promoción.')
      }
      const d = await get(`/ml/promociones/items?cuenta=${p.cuenta_id}&promotion_id=${encodeURIComponent(p.id)}&tipo=${encodeURIComponent(p.tipo)}`)
      setItems((m) => ({ ...m, [k]: itemsDe(d) }))
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const lista = rows || []

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Promociones y campañas</h1>
          <p className="muted sm" style={{ margin: 0 }}>Ofertas y campañas de Mercado Libre. Suma tus productos con su precio promocional.</p>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)}>
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : lista.length === 0 ? (
        <div className="empty">No hay promociones disponibles por ahora.</div>
      ) : (
        <div className="stack" style={{ gap: 10 }}>
          {lista.map((p) => {
            const k = `${p.cuenta_id}-${p.id}`
            const [et, color] = estadoInfo(p.estado)
            const its = items[k]
            return (
              <div key={k} className="card stack" style={{ gap: 8 }}>
                <div className="row between" style={{ flexWrap: 'wrap', gap: 6, alignItems: 'center' }}>
                  <div className="row" style={{ gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
                    <b>{p.nombre || tipoTxt(p.tipo)}</b>
                    <span className="badge" style={{ color }}>{et}</span>
                    <span className="tag">{tipoTxt(p.tipo)}</span>
                  </div>
                  <div className="row" style={{ gap: 8, alignItems: 'center' }}>
                    <span className="muted xs">{p.inicio ? `${fecha(p.inicio)} – ${fecha(p.fin)}` : ''} · {p.cuenta}</span>
                    <button className="btn btn-sm" onClick={() => abrir(p)}>{abierto === k ? 'Ocultar' : 'Ver productos'}</button>
                  </div>
                </div>

                {abierto === k && (
                  <div className="table-wrap" style={{ overflowX: 'auto', borderTop: '1px solid var(--line)', paddingTop: 8 }}>
                    {!its ? <div className="muted sm">Cargando productos…</div> : its.length === 0 ? <div className="muted sm">Sin productos elegibles para esta promoción.</div> : (
                      <table style={{ minWidth: 560 }}>
                        <thead><tr><th>Producto</th><th>Precio</th><th>Precio promo</th><th></th></tr></thead>
                        <tbody>
                          {its.map((it) => {
                            const itemId = it.id || it.item_id
                            const enPromo = participando(it.status)
                            const precio = it.price ?? it.original_price
                            return (
                              <tr key={itemId}>
                                <td className="sm"><b>{titulos[itemId] || itemId}</b><div className="muted xs">{itemId}</div></td>
                                <td className="sm">{precio != null ? money(precio) : '—'}</td>
                                <td>
                                  {enPromo ? <span className="sm">{it.deal_price != null ? money(it.deal_price) : '—'}</span> : (
                                    <input className="input" style={{ width: 110 }} type="number" min="0" placeholder={it.suggested_discount != null ? '' : 'precio'}
                                      value={precios[itemId] ?? (it.deal_price ?? '')} onChange={(e) => setPrecios((m) => ({ ...m, [itemId]: e.target.value }))} />
                                  )}
                                </td>
                                <td className="r" style={{ whiteSpace: 'nowrap' }}>
                                  {enPromo
                                    ? <button className="btn btn-sm" disabled={busy === itemId} onClick={() => accion(p, it, true)}>Quitar</button>
                                    : <button className="btn btn-sm btn-accent" disabled={busy === itemId} onClick={() => accion(p, it, false)}>{busy === itemId ? '…' : 'Agregar'}</button>}
                                </td>
                              </tr>
                            )
                          })}
                        </tbody>
                      </table>
                    )}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
