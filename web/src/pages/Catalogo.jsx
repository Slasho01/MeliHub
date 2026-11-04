import { useEffect, useMemo, useState } from 'react'
import { get } from '../api'

const money = (v, m) => new Intl.NumberFormat('es-CL', { style: 'currency', currency: m || 'CLP', maximumFractionDigits: 0 }).format(v)

const ESTADO = {
  active: ['Activa', 'ok'],
  paused: ['Pausada', 'warn'],
  closed: ['Cerrada', 'bad'],
}

export default function Catalogo() {
  const [productos, setProductos] = useState(null)
  const [cuentas, setCuentas] = useState([])
  const [error, setError] = useState(null)
  const [q, setQ] = useState('')
  const [cuenta, setCuenta] = useState('')

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])
  useEffect(() => {
    const params = new URLSearchParams()
    if (q.trim()) params.set('q', q.trim())
    if (cuenta) params.set('cuenta_id', cuenta)
    const s = params.toString()
    get(`/productos${s ? `?${s}` : ''}`).then(setProductos).catch((e) => setError(e.message))
  }, [q, cuenta])

  const total = useMemo(() => productos?.length ?? 0, [productos])

  return (
    <div className="stack">
      <div className="row between">
        <div><h1>Catálogo unificado</h1><p className="muted sm" style={{ margin: 0 }}>{total} publicación(es) de todas tus cuentas.</p></div>
        <div className="row">
          <input className="input" style={{ width: 220 }} placeholder="Buscar…" value={q} onChange={(e) => setQ(e.target.value)} />
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)}>
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
        </div>
      </div>
      {error && <div className="alert alert-bad">{error}</div>}
      <div className="card" style={{ padding: 0 }}>
        {!productos ? <div className="empty">Cargando…</div> : productos.length === 0 ? (
          <div className="empty">Sin publicaciones. Conecta una cuenta y pulsa “Sincronizar”.</div>
        ) : (
          <table>
            <thead><tr><th></th><th>Publicación</th><th>Precio</th><th>Stock</th><th>Estado</th></tr></thead>
            <tbody>
              {productos.map((p) => {
                const [label, tone] = ESTADO[p.estado] || [p.estado, '']
                return (
                  <tr key={p.id}>
                    <td>{p.thumbnail ? <img src={p.thumbnail} alt="" /> : <div style={{ width: 40, height: 40 }} />}</td>
                    <td>
                      {p.permalink ? <a href={p.permalink} target="_blank" rel="noopener noreferrer"><b>{p.titulo}</b></a> : <b>{p.titulo}</b>}
                      <div className="muted sm">{p.ml_item_id}</div>
                    </td>
                    <td>{money(p.precio, p.moneda)}</td>
                    <td style={{ color: p.stock <= 0 ? 'var(--bad)' : undefined }}>{p.stock}</td>
                    <td><span className={`badge ${tone}`}>{label}</span></td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
