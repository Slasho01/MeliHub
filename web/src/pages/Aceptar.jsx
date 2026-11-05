import { useState } from 'react'
import { post } from '../api'

export default function Aceptar() {
  const token = new URLSearchParams(window.location.search).get('token') || ''
  const [pass, setPass] = useState('')
  const [error, setError] = useState(null)
  const [ok, setOk] = useState(false)
  const [busy, setBusy] = useState(false)

  const submit = async (e) => {
    e.preventDefault()
    if (pass.length < 8) { setError('La contraseña debe tener al menos 8 caracteres.'); return }
    setBusy(true); setError(null)
    try { await post('/equipo/aceptar', { token, password: pass }); setOk(true) }
    catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  return (
    <div className="auth-wrap">
      <div className="card auth-card stack">
        <div className="center"><div className="brand" style={{ fontSize: 24 }}>Meli<b>Hub</b></div></div>
        {!token ? (
          <div className="alert alert-bad">Enlace de invitación inválido o incompleto.</div>
        ) : ok ? (
          <>
            <div className="alert alert-ok">¡Listo! Tu cuenta quedó activa. Ya puedes iniciar sesión.</div>
            <a className="btn btn-accent" href="/">Ir a iniciar sesión</a>
          </>
        ) : (
          <form className="stack" onSubmit={submit}>
            <p className="muted sm" style={{ margin: 0 }}>Te invitaron a MeliHub. Crea tu contraseña para entrar.</p>
            <div><label>Contraseña (mín. 8)</label>
              <input className="input" type="password" autoComplete="new-password" value={pass} onChange={(e) => setPass(e.target.value)} required minLength={8} /></div>
            {error && <div className="alert alert-bad">{error}</div>}
            <button className="btn btn-accent" disabled={busy}>{busy ? '…' : 'Aceptar invitación'}</button>
          </form>
        )}
      </div>
    </div>
  )
}
