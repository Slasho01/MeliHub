import { useEffect, useState } from 'react'
import { del, get, post } from '../api'

export default function ApiKeys() {
  const [keys, setKeys] = useState(null)
  const [cuentas, setCuentas] = useState([])
  const [error, setError] = useState(null)
  const [nueva, setNueva] = useState({ nombre: '', canal: 'jarstore', ml_cuenta_id: '' })
  const [creada, setCreada] = useState(null) // la clave en claro, se muestra una vez

  const cargar = () => get('/api-keys').then(setKeys).catch((e) => setError(e.message))
  useEffect(() => { cargar(); get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])

  const crear = async (e) => {
    e.preventDefault(); setError(null)
    try {
      const body = { nombre: nueva.nombre, canal: nueva.canal }
      if (nueva.ml_cuenta_id) body.ml_cuenta_id = nueva.ml_cuenta_id
      const r = await post('/api-keys', body)
      setCreada(r.api_key)
      setNueva({ nombre: '', canal: 'jarstore', ml_cuenta_id: '' })
      cargar()
    } catch (err) { setError(err.message) }
  }

  const revocar = async (k) => {
    if (!window.confirm(`¿Revocar la API key “${k.nombre}”? La plataforma que la use dejará de funcionar.`)) return
    try { await del(`/api-keys/${k.id}`); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div><h1>API keys</h1><p className="muted sm" style={{ margin: 0 }}>Para que JarStore u otra plataforma (ej. WordPress) consuma tus productos y reporte ventas.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {creada && (
        <div className="card stack" style={{ borderColor: 'var(--accent)' }}>
          <b>¡Copia tu API key ahora! No se volverá a mostrar.</b>
          <code style={{ background: 'var(--bg)', padding: '10px 12px', borderRadius: 8, wordBreak: 'break-all', fontSize: 13 }}>{creada}</code>
          <div className="row">
            <button className="btn btn-sm" onClick={() => navigator.clipboard?.writeText(creada)}>Copiar</button>
            <button className="btn btn-sm" onClick={() => setCreada(null)}>Listo</button>
          </div>
        </div>
      )}
      <div className="card">
        <form className="row" style={{ gap: 10, flexWrap: 'wrap', alignItems: 'flex-end' }} onSubmit={crear}>
          <div style={{ flex: '2 1 180px' }}><label>Nombre</label><input className="input" placeholder="Mi tienda JarStore" value={nueva.nombre} onChange={(e) => setNueva({ ...nueva, nombre: e.target.value })} required /></div>
          <div style={{ flex: '1 1 150px' }}><label>Canal (precio que sirve)</label>
            <select className="input" value={nueva.canal} onChange={(e) => setNueva({ ...nueva, canal: e.target.value })}>
              <option value="jarstore">JarStore</option>
              <option value="mercadolibre">Mercado Libre</option>
            </select>
          </div>
          <div style={{ flex: '1 1 170px' }}><label>Cuenta (opcional)</label>
            <select className="input" value={nueva.ml_cuenta_id} onChange={(e) => setNueva({ ...nueva, ml_cuenta_id: e.target.value })}>
              <option value="">Todas mis cuentas</option>
              {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
            </select>
          </div>
          <button className="btn btn-accent">Crear API key</button>
        </form>
      </div>
      <div className="card" style={{ padding: 0 }}>
        {!keys ? <div className="empty">Cargando…</div> : keys.length === 0 ? (
          <div className="empty">Aún no has creado API keys.</div>
        ) : (
          <table>
            <thead><tr><th>Nombre</th><th>Canal</th><th>Cuenta</th><th>Último uso</th><th></th></tr></thead>
            <tbody>
              {keys.map((k) => (
                <tr key={k.id}>
                  <td><b>{k.nombre}</b></td>
                  <td><span className="badge">{k.canal}</span></td>
                  <td className="muted sm">{k.ml_cuenta_id ? (cuentas.find((c) => c.id === k.ml_cuenta_id)?.nickname || 'cuenta') : 'Todas'}</td>
                  <td className="muted sm">{k.ultima_uso_at ? new Date(k.ultima_uso_at + 'Z').toLocaleString('es-CL') : 'Nunca'}</td>
                  <td style={{ textAlign: 'right' }}><button className="btn btn-sm btn-danger" onClick={() => revocar(k)}>Revocar</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
