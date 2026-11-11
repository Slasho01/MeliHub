import { useEffect, useState } from 'react'
import { get, post, del } from '../api'
import ParisPublicar from './ParisPublicar'
import ParisOrdenes from './ParisOrdenes'

export default function Paris({ me }) {
  const owner = me?.es_owner
  const [cuentas, setCuentas] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)
  const [form, setForm] = useState({ nombre: '', api_key: '', sandbox: true })
  const [sel, setSel] = useState(null)
  const [tab, setTab] = useState('publicar')

  const cargar = () => get('/paris/cuentas').then(setCuentas).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const conectar = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null); setMsg(null)
    try {
      await post('/paris/cuentas', { nombre: form.nombre || null, api_key: form.api_key, sandbox: form.sandbox })
      setMsg('Cuenta de Paris conectada.')
      setForm({ nombre: '', api_key: '', sandbox: true })
      cargar()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  const desconectar = async (c) => {
    if (!window.confirm(`¿Desconectar ${c.nombre || 'esta cuenta de Paris'}?`)) return
    try { await del(`/paris/cuentas/${c.id}`); if (sel?.id === c.id) setSel(null); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div>
        <h1 style={{ margin: 0 }}>Paris (Cencosud)</h1>
        <p className="muted sm" style={{ margin: 0 }}>Conecta tu cuenta de Marketplace Cencosud (Paris) y distribuye tu catálogo del hub. Disponible en planes Pro y Business.</p>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {cuentas == null ? <div className="muted">Cargando…</div> : (
        <div className="card stack" style={{ gap: 10 }}>
          <div className="strong">Cuentas conectadas</div>
          {cuentas.length === 0 ? <div className="muted sm">Aún no has conectado ninguna cuenta.</div> : (
            <div className="stack" style={{ gap: 8 }}>
              {cuentas.map((c) => (
                <div key={c.id} className="row between" style={{ gap: 8, flexWrap: 'wrap' }}>
                  <div>
                    <span className="strong">{c.nombre || 'Cuenta Paris'}</span>{' '}
                    <span className={`badge ${c.sandbox ? '' : 'badge-ok'}`}>{c.sandbox ? 'Sandbox' : 'Producción'}</span>
                  </div>
                  <div className="row" style={{ gap: 6 }}>
                    <button className="btn btn-sm" onClick={() => { setSel(c); setTab('publicar') }}>Gestionar</button>
                    {owner && <button className="btn btn-sm btn-danger" onClick={() => desconectar(c)}>Desconectar</button>}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {owner && (
        <form className="card stack" style={{ gap: 10 }} onSubmit={conectar}>
          <div className="strong">Conectar cuenta</div>
          <p className="muted sm" style={{ margin: 0 }}>La API Key se obtiene en tu Seller Center de Paris → Mi cuenta. Para pruebas, crea una cuenta de staging y marca «Sandbox».</p>
          <input className="input" placeholder="Nombre (opcional)" value={form.nombre} onChange={(e) => setForm({ ...form, nombre: e.target.value })} />
          <input className="input" type="password" placeholder="API Key" value={form.api_key} onChange={(e) => setForm({ ...form, api_key: e.target.value })} required />
          <label className="check"><input type="checkbox" checked={form.sandbox} onChange={(e) => setForm({ ...form, sandbox: e.target.checked })} /> Sandbox (entorno de pruebas)</label>
          <button className="btn btn-dark" disabled={busy}>{busy ? 'Conectando…' : 'Conectar'}</button>
        </form>
      )}

      {sel && (
        <div className="card stack" style={{ gap: 10 }}>
          <div className="row between">
            <div className="strong">{sel.nombre || 'Cuenta Paris'} <span className="muted sm">· {sel.sandbox ? 'Sandbox' : 'Producción'}</span></div>
            <button className="btn btn-sm" onClick={() => setSel(null)}>Cerrar</button>
          </div>
          <div className="seg">
            <button className={tab === 'publicar' ? 'on' : ''} onClick={() => setTab('publicar')}>Publicar</button>
            <button className={tab === 'ordenes' ? 'on' : ''} onClick={() => setTab('ordenes')}>Órdenes</button>
          </div>
          {tab === 'publicar' ? <ParisPublicar cuenta={sel} /> : <ParisOrdenes cuenta={sel} />}
        </div>
      )}
    </div>
  )
}
