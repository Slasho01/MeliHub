import { useState } from 'react'
import { post } from '../api'

export default function Reset() {
  const token = new URLSearchParams(window.location.search).get('token') || ''
  const [nueva, setNueva] = useState('')
  const [error, setError] = useState(null)
  const [ok, setOk] = useState(false)
  const [busy, setBusy] = useState(false)

  const submit = async (e) => {
    e.preventDefault()
    if (nueva.length < 8) { setError('La contraseña debe tener al menos 8 caracteres.'); return }
    setBusy(true); setError(null)
    try { await post('/auth/reset', { token, nueva }); setOk(true) }
    catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  return (
    <div className="auth-wrap">
      <div className="card auth-card stack">
        <div className="center"><div className="brand" style={{ fontSize: 24 }}>Meli<b>Hub</b></div></div>
        {!token ? (
          <div className="alert alert-bad">Enlace inválido o incompleto.</div>
        ) : ok ? (
          <>
            <div className="alert alert-ok">¡Contraseña actualizada! Ya puedes iniciar sesión.</div>
            <a className="btn btn-accent" href="/">Ir a iniciar sesión</a>
          </>
        ) : (
          <form className="stack" onSubmit={submit}>
            <p className="muted sm" style={{ margin: 0 }}>Elige tu nueva contraseña.</p>
            <div><label>Nueva contraseña (mín. 8)</label>
              <input className="input" type="password" autoComplete="new-password" value={nueva} onChange={(e) => setNueva(e.target.value)} required minLength={8} /></div>
            {error && <div className="alert alert-bad">{error}</div>}
            <button className="btn btn-accent" disabled={busy}>{busy ? '…' : 'Cambiar contraseña'}</button>
          </form>
        )}
      </div>
    </div>
  )
}
