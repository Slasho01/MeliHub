import { useEffect, useState } from 'react'
import { get, post, del } from '../api'
import FalabellaPublicar from './FalabellaPublicar'
import FalabellaOrdenes from './FalabellaOrdenes'
import FalabellaEstado from './FalabellaEstado'

export default function Falabella({ me }) {
  const owner = me?.es_owner
  const [cuentas, setCuentas] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)
  const [form, setForm] = useState({ user_id: '', api_key: '', nickname: '' })

  const cargar = () => get('/falabella/cuentas').then(setCuentas).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const conectar = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null); setMsg(null)
    try {
      await post('/falabella/cuentas', { user_id: form.user_id, api_key: form.api_key, nickname: form.nickname })
      setMsg('Cuenta de Falabella conectada correctamente.')
      setForm({ user_id: '', api_key: '', nickname: '' })
      cargar()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  const desconectar = async (c) => {
    if (!window.confirm(`¿Desconectar ${c.nickname || c.user_id}?`)) return
    try { await del(`/falabella/cuentas/${c.id}`); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div>
        <h1 style={{ margin: 0 }}>Falabella</h1>
        <p className="muted sm" style={{ margin: 0 }}>Conecta tu cuenta de Falabella Seller Center para distribuir tu catálogo del hub.</p>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {cuentas == null ? (
        <div className="muted">Cargando…</div>
      ) : (
        <div className="card stack" style={{ gap: 10 }}>
          <div className="strong">Cuentas conectadas</div>
          {cuentas.length === 0 ? (
            <div className="muted sm">Aún no has conectado ninguna cuenta.</div>
          ) : (
            <div className="stack" style={{ gap: 8 }}>
              {cuentas.map((c) => (
                <div key={c.id} className="row between" style={{ gap: 8, flexWrap: 'wrap' }}>
                  <div><span className="strong">{c.nickname || c.user_id}</span> <span className="muted sm">· {c.user_id}</span></div>
                  {owner && <button className="btn btn-sm btn-danger" onClick={() => desconectar(c)}>Desconectar</button>}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {cuentas && cuentas.length > 0 && <FalabellaEstado cuenta={cuentas[0]} />}

      {cuentas && cuentas.length > 0 && <FalabellaOrdenes cuenta={cuentas[0]} />}

      {owner && cuentas && cuentas.length > 0 && <FalabellaPublicar cuentas={cuentas} />}

      {owner && (
        <form className="card stack" onSubmit={conectar} style={{ gap: 10 }}>
          <div className="strong">Conectar cuenta</div>
          <p className="muted sm" style={{ margin: 0 }}>
            Credenciales del Seller Center API (se guardan <b>cifradas</b>). El <b>UserID</b> es tu email de acceso; la <b>API key</b> la generas en Seller Center → Integraciones.
          </p>
          <label className="stack" style={{ gap: 4 }}><span className="sm">UserID (email)</span>
            <input className="input" value={form.user_id} onChange={(e) => setForm({ ...form, user_id: e.target.value })} placeholder="correo@tutienda.cl" autoComplete="off" required /></label>
          <label className="stack" style={{ gap: 4 }}><span className="sm">API key</span>
            <input className="input" type="password" value={form.api_key} onChange={(e) => setForm({ ...form, api_key: e.target.value })} autoComplete="off" required /></label>
          <label className="stack" style={{ gap: 4 }}><span className="sm">Nombre (opcional)</span>
            <input className="input" value={form.nickname} onChange={(e) => setForm({ ...form, nickname: e.target.value })} placeholder="Mi tienda Falabella" /></label>
          <div className="row" style={{ justifyContent: 'flex-end' }}>
            <button className="btn btn-accent" disabled={busy || !form.user_id || !form.api_key}>{busy ? 'Conectando…' : 'Conectar'}</button>
          </div>
        </form>
      )}
    </div>
  )
}
