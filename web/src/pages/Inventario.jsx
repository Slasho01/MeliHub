import { useEffect, useState } from 'react'
import { get, money, put } from '../api'

function GanCell({ g }) {
  if (!g) return <span className="muted">—</span>
  const tone = g.ganancia > 0 ? 'ok' : 'bad'
  return (
    <div>
      <b style={{ color: `var(--${tone})` }}>{money(g.ganancia)}</b>
      <div className="muted sm">{g.margen}% · comis. {money(g.comision)}</div>
    </div>
  )
}

export default function Inventario() {
  const [items, setItems] = useState(null)
  const [cuentas, setCuentas] = useState([])
  const [error, setError] = useState(null)
  const [q, setQ] = useState('')
  const [cuenta, setCuenta] = useState('')
  const [edit, setEdit] = useState({}) // { [id]: {costo, precio_jarstore} }

  const cargar = () => {
    const params = new URLSearchParams()
    if (q.trim()) params.set('q', q.trim())
    if (cuenta) params.set('cuenta_id', cuenta)
    const s = params.toString()
    get(`/productos${s ? `?${s}` : ''}`).then((d) => { setItems(d); setEdit({}) }).catch((e) => setError(e.message))
  }
  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])
  useEffect(() => { cargar() }, [q, cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const val = (p, campo) => edit[p.id]?.[campo] ?? (p[campo] ?? '')
  const onChange = (p, campo, v) => setEdit({ ...edit, [p.id]: { ...edit[p.id], [campo]: v } })

  const guardar = async (p) => {
    const e = edit[p.id]
    if (!e) return
    const body = {}
    if ('costo' in e) body.costo = e.costo === '' ? null : Number(e.costo)
    if ('precio_jarstore' in e) body.precio_jarstore = e.precio_jarstore === '' ? null : Number(e.precio_jarstore)
    if (!Object.keys(body).length) return
    try { await put(`/productos/${p.id}/inventario`, body); cargar() } catch (err) { setError(err.message) }
  }

  return (
    <div className="stack">
      <div className="row between">
        <div><h1>Inventario</h1><p className="muted sm" style={{ margin: 0 }}>Edita costo y precio JarStore; la ganancia por canal se calcula sola.</p></div>
        <div className="row">
          <input className="input" style={{ width: 200 }} placeholder="Buscar…" value={q} onChange={(e) => setQ(e.target.value)} />
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)}>
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
        </div>
      </div>
      {error && <div className="alert alert-bad">{error}</div>}
      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!items ? <div className="empty">Cargando…</div> : items.length === 0 ? (
          <div className="empty">Sin publicaciones. Conecta una cuenta y sincroniza.</div>
        ) : (
          <table style={{ minWidth: 900 }}>
            <thead><tr>
              <th></th><th>Publicación</th><th>Precio ML</th><th>Costo</th><th>Precio JarStore</th>
              <th>Ganancia ML</th><th>Ganancia JarStore</th><th>Stock</th>
            </tr></thead>
            <tbody>
              {items.map((p) => (
                <tr key={p.id}>
                  <td>{p.thumbnail ? <img src={p.thumbnail} alt="" /> : <div style={{ width: 40, height: 40 }} />}</td>
                  <td style={{ maxWidth: 220 }}><b>{p.titulo}</b><div className="muted sm">{p.ml_item_id}</div></td>
                  <td>{money(p.precio)}</td>
                  <td><input className="input" style={{ width: 92 }} type="number" min="0" placeholder="—"
                    value={val(p, 'costo')} onChange={(e) => onChange(p, 'costo', e.target.value)} onBlur={() => guardar(p)} /></td>
                  <td><input className="input" style={{ width: 100 }} type="number" min="0" placeholder="—"
                    value={val(p, 'precio_jarstore')} onChange={(e) => onChange(p, 'precio_jarstore', e.target.value)} onBlur={() => guardar(p)} /></td>
                  <td><GanCell g={p.ml} /></td>
                  <td><GanCell g={p.jarstore} /></td>
                  <td style={{ color: p.stock <= 0 ? 'var(--bad)' : undefined }}>{p.stock}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
