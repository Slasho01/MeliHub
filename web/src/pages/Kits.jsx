import { useEffect, useState } from 'react'
import { get, post, del } from '../api'

// Buscador de productos del inventario (type-ahead sobre /productos?q=).
function BuscadorProducto({ placeholder, excluir = [], onPick }) {
  const [q, setQ] = useState('')
  const [res, setRes] = useState([])
  const [abierto, setAbierto] = useState(false)
  useEffect(() => {
    if (q.trim().length < 2) { setRes([]); return }
    const t = setTimeout(() => {
      get('/productos?q=' + encodeURIComponent(q.trim()))
        .then((d) => setRes((Array.isArray(d) ? d : d?.data || []).filter((p) => !excluir.includes(p.id)).slice(0, 8)))
        .catch(() => setRes([]))
    }, 250)
    return () => clearTimeout(t)
  }, [q, excluir])
  return (
    <div style={{ position: 'relative' }}>
      <input className="input" value={q} placeholder={placeholder} style={{ width: '100%' }}
        onChange={(e) => { setQ(e.target.value); setAbierto(true) }}
        onFocus={() => setAbierto(true)}
        onBlur={() => setTimeout(() => setAbierto(false), 150)} />
      {abierto && res.length > 0 && (
        <div style={{ position: 'absolute', zIndex: 20, top: '100%', left: 0, right: 0, background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 8, marginTop: 4, maxHeight: 280, overflowY: 'auto', boxShadow: '0 8px 24px rgba(0,0,0,.18)' }}>
          {res.map((p) => (
            <button key={p.id} type="button"
              onMouseDown={(e) => { e.preventDefault(); onPick(p); setQ(''); setRes([]); setAbierto(false) }}
              style={{ display: 'flex', alignItems: 'center', gap: 8, width: '100%', padding: '8px 10px', background: 'none', border: 'none', borderBottom: '1px solid var(--line)', cursor: 'pointer', textAlign: 'left' }}>
              {p.thumbnail ? <img src={p.thumbnail} alt="" style={{ width: 32, height: 32, objectFit: 'cover', borderRadius: 4, flexShrink: 0 }} /> : <span style={{ width: 32 }} />}
              <span style={{ flex: 1, fontSize: 13, lineHeight: 1.2 }}>{p.titulo}</span>
              <i className="muted sm" style={{ whiteSpace: 'nowrap' }}>stock {p.stock}</i>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

export default function Kits() {
  const [kits, setKits] = useState([])
  const [cargando, setCargando] = useState(true)
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  // Formulario de nuevo kit.
  const [kitProd, setKitProd] = useState(null)
  const [comps, setComps] = useState([]) // [{ producto, cantidad }]
  const [guardando, setGuardando] = useState(false)

  const cargar = () => {
    setCargando(true)
    get('/kits').then((d) => setKits(d?.kits || [])).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(cargar, [])

  const addComp = (p) => {
    setError('')
    if (kitProd && p.id === kitProd.id) { setError('Un componente no puede ser el mismo kit.'); return }
    if (comps.some((c) => c.producto.id === p.id)) return
    setComps((cs) => [...cs, { producto: p, cantidad: 1 }])
  }
  const setCant = (id, n) => setComps((cs) => cs.map((c) => (c.producto.id === id ? { ...c, cantidad: Math.max(1, Math.floor(n) || 1) } : c)))
  const rmComp = (id) => setComps((cs) => cs.filter((c) => c.producto.id !== id))

  const guardar = async () => {
    setError(''); setMsg('')
    if (!kitProd) { setError('Elige el producto que será el kit.'); return }
    if (comps.length === 0) { setError('Agrega al menos un componente.'); return }
    setGuardando(true)
    try {
      await post('/kits', { kit_producto_id: kitProd.id, componentes: comps.map((c) => ({ producto_id: c.producto.id, cantidad: c.cantidad })) })
      setMsg('Kit guardado. El stock del kit ya refleja sus componentes ✓')
      setKitProd(null); setComps([]); cargar()
    } catch (e) { setError(e.message) } finally { setGuardando(false) }
  }

  const eliminar = async (id) => {
    if (!confirm('¿Deshacer este kit? La publicación queda como producto normal (sin componentes). No borra la publicación.')) return
    setError('')
    try { await del('/kits/' + id); cargar() } catch (e) { setError(e.message) }
  }

  const excluir = [kitProd?.id, ...comps.map((c) => c.producto.id)].filter(Boolean)
  const dispNuevo = comps.length ? Math.min(...comps.map((c) => Math.floor((c.producto.stock || 0) / c.cantidad))) : 0

  return (
    <div>
      <h1 style={{ margin: 0 }}>Kits / combos</h1>
      <p className="muted">Arma packs con varios productos. Al vender un kit se descuenta el stock de cada componente, y su disponibilidad se calcula sola: <b>mínimo</b> de (stock de cada componente ÷ cantidad).</p>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {/* Nuevo kit */}
      <div className="card" style={{ marginBottom: 18 }}>
        <h3 style={{ marginTop: 0 }}>Nuevo kit</h3>
        <label className="muted sm">1 · Producto que será el kit (la publicación que vendes como pack)</label>
        {kitProd ? (
          <div className="row" style={{ alignItems: 'center', gap: 8, margin: '6px 0 14px' }}>
            {kitProd.thumbnail && <img src={kitProd.thumbnail} alt="" style={{ width: 36, height: 36, objectFit: 'cover', borderRadius: 4 }} />}
            <b style={{ flex: 1 }}>{kitProd.titulo}</b>
            <button className="btn btn-sm" onClick={() => setKitProd(null)}>Cambiar</button>
          </div>
        ) : (
          <div style={{ margin: '6px 0 14px' }}>
            <BuscadorProducto placeholder="Busca el producto del kit…" excluir={comps.map((c) => c.producto.id)} onPick={setKitProd} />
          </div>
        )}

        <label className="muted sm">2 · Componentes (qué productos y cuántas unidades consume cada kit)</label>
        <div style={{ margin: '6px 0 10px' }}>
          <BuscadorProducto placeholder="Busca un componente para agregar…" excluir={excluir} onPick={addComp} />
        </div>

        {comps.length > 0 && (
          <table className="table" style={{ marginBottom: 12 }}>
            <thead><tr><th>Componente</th><th style={{ width: 90 }}>Cant/kit</th><th style={{ width: 80 }}>Stock</th><th style={{ width: 70 }}>Aporta</th><th style={{ width: 40 }} /></tr></thead>
            <tbody>
              {comps.map((c) => {
                const aporta = Math.floor((c.producto.stock || 0) / c.cantidad)
                return (
                  <tr key={c.producto.id}>
                    <td>{c.producto.titulo}</td>
                    <td><input className="input" type="number" min="1" value={c.cantidad} style={{ width: 70 }} onChange={(e) => setCant(c.producto.id, +e.target.value)} /></td>
                    <td>{c.producto.stock ?? 0}</td>
                    <td><span className={aporta === dispNuevo ? 'tag' : 'muted'}>{aporta}</span></td>
                    <td><button className="btn btn-sm" title="Quitar" onClick={() => rmComp(c.producto.id)}>✕</button></td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}

        <div className="row" style={{ alignItems: 'center', gap: 12 }}>
          <button className="btn btn-accent" onClick={guardar} disabled={guardando}>{guardando ? 'Guardando…' : 'Guardar kit'}</button>
          {comps.length > 0 && <span className="muted sm">Disponibilidad resultante del kit: <b>{dispNuevo}</b></span>}
        </div>
      </div>

      {/* Kits existentes */}
      <h3>Tus kits</h3>
      {cargando ? <p className="muted">Cargando…</p> : kits.length === 0 ? (
        <p className="muted">Aún no tienes kits. Crea uno arriba: elige el producto del pack y sus componentes.</p>
      ) : (
        <div className="lp-grid" style={{ gap: 14 }}>
          {kits.map((k) => (
            <div key={k.kit_producto_id} className="card">
              <div className="row" style={{ alignItems: 'center', gap: 10 }}>
                {k.thumbnail && <img src={k.thumbnail} alt="" style={{ width: 44, height: 44, objectFit: 'cover', borderRadius: 6 }} />}
                <div style={{ flex: 1 }}>
                  <b>{k.titulo}</b>
                  <div className="muted sm">
                    Disponible: <b>{k.disponible}</b>
                    {k.stock_actual !== k.disponible && <span className="tag" style={{ marginLeft: 6 }}>stock pub.: {k.stock_actual} (se sincroniza)</span>}
                  </div>
                </div>
                <button className="btn btn-sm" onClick={() => eliminar(k.kit_producto_id)}>Deshacer</button>
              </div>
              <table className="table" style={{ marginTop: 10, marginBottom: 0 }}>
                <tbody>
                  {k.componentes.map((c) => (
                    <tr key={c.producto_id}>
                      <td style={{ fontSize: 13 }}>{c.titulo}</td>
                      <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>×{c.cantidad}</td>
                      <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>stock {c.stock}</td>
                      <td style={{ whiteSpace: 'nowrap' }}><span className={c.aporta === k.disponible ? 'tag' : 'muted sm'}>aporta {c.aporta}</span></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
