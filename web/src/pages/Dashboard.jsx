import { useEffect, useState } from 'react'
import { del, get, post } from '../api'

export default function Dashboard() {
  const [cuentas, setCuentas] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState('')

  const cargar = () => get('/ml/cuentas').then(setCuentas).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const conectar = async () => {
    setError(null); setMsg(null)
    try {
      const { authorize_url } = await post('/ml/conectar')
      window.open(authorize_url, '_blank', 'noopener')
      setMsg('Autoriza tu cuenta en la ventana de Mercado Libre y luego pulsa “Actualizar”.')
    } catch (e) { setError(e.message) }
  }

  const sincronizar = async (c) => {
    setBusy(c.id); setError(null); setMsg(null)
    try {
      const r = await post(`/ml/cuentas/${c.id}/sincronizar`)
      setMsg(`${c.nickname || 'Cuenta'}: ${r.sincronizados} publicaciones sincronizadas.`)
      cargar()
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const desconectar = async (c) => {
    if (!window.confirm(`¿Desconectar ${c.nickname || 'esta cuenta'}? Se quitarán sus productos.`)) return
    try { await del(`/ml/cuentas/${c.id}`); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div className="row between">
        <div><h1>Cuentas de Mercado Libre</h1><p className="muted sm" style={{ margin: 0 }}>Conecta una o varias cuentas para centralizar sus publicaciones.</p></div>
        <div className="row">
          <button className="btn btn-sm" onClick={cargar}>Actualizar</button>
          <button className="btn btn-accent btn-sm" onClick={conectar}>+ Conectar cuenta</button>
        </div>
      </div>
      {msg && <div className="alert alert-ok">{msg}</div>}
      {error && <div className="alert alert-bad">{error}</div>}
      <div className="card" style={{ padding: 0 }}>
        {!cuentas ? <div className="empty">Cargando…</div> : cuentas.length === 0 ? (
          <div className="empty">Aún no hay cuentas conectadas.<br />Pulsa <b>“Conectar cuenta”</b> para empezar.</div>
        ) : (
          <table>
            <thead><tr><th>Cuenta</th><th>Sitio</th><th>Última sincronización</th><th></th></tr></thead>
            <tbody>
              {cuentas.map((c) => (
                <tr key={c.id}>
                  <td><b>{c.nickname || `Usuario ${c.ml_user_id}`}</b><div className="muted sm">ID {c.ml_user_id}</div></td>
                  <td><span className="badge">{c.sitio}</span></td>
                  <td className="muted sm">{c.ultima_sync_at ? new Date(c.ultima_sync_at + 'Z').toLocaleString('es-CL') : 'Nunca'}</td>
                  <td style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>
                    <button className="btn btn-sm" disabled={busy === c.id} onClick={() => sincronizar(c)}>{busy === c.id ? 'Sincronizando…' : 'Sincronizar'}</button>{' '}
                    <button className="btn btn-sm btn-danger" onClick={() => desconectar(c)}>Desconectar</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
