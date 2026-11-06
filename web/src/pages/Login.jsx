import { useState } from 'react'
import { post, setToken } from '../api'
import PasswordMeter from '../components/PasswordMeter'

export default function Login({ onAuth }) {
  const [modo, setModo] = useState('login') // login | registro | forgot
  const [form, setForm] = useState({ nombre: '', email: '', password: '' })
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)
  const [need2fa, setNeed2fa] = useState(false)
  const [codigo, setCodigo] = useState('')
  const set = (k) => (e) => setForm({ ...form, [k]: e.target.value })
  const ir = (m) => { setModo(m); setError(null); setMsg(null); setNeed2fa(false); setCodigo('') }

  const submit = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null); setMsg(null)
    try {
      if (modo === 'forgot') {
        const r = await post('/auth/forgot', { email: form.email })
        setMsg(r.message || 'Si el correo está registrado, te enviamos un enlace.')
        setBusy(false)
        return
      }
      const path = modo === 'login' ? '/auth/login' : '/auth/registro'
      const body = modo === 'login'
        ? { email: form.email, password: form.password, ...(need2fa ? { codigo } : {}) }
        : { nombre: form.nombre, email: form.email, password: form.password }
      const r = await post(path, body)
      if (modo === 'login' && r.twofa_required) { setNeed2fa(true); setBusy(false); return }
      setToken(r.token)
      onAuth()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  const titulo = { login: 'Entrar', registro: 'Crear cuenta', forgot: 'Enviar enlace' }[modo]

  return (
    <div className="auth-wrap">
      <div className="card auth-card stack">
        <div className="center">
          <div className="brand" style={{ fontSize: 24 }}>Meli<b>Hub</b></div>
          <p className="muted sm" style={{ margin: '4px 0 0' }}>Centraliza tus publicaciones de Mercado Libre</p>
        </div>
        <form className="stack" onSubmit={submit}>
          {modo === 'registro' && (
            <div><label>Nombre</label><input className="input" value={form.nombre} onChange={set('nombre')} required /></div>
          )}
          <div><label>Email</label><input className="input" type="email" value={form.email} onChange={set('email')} required /></div>
          {modo !== 'forgot' && (
            <div><label>Contraseña</label><input className="input" type="password" value={form.password} onChange={set('password')} required minLength={8} />
              {modo === 'registro' && <PasswordMeter value={form.password} />}</div>
          )}
          {need2fa && modo === 'login' && (
            <div><label>Código de verificación (2FA)</label>
              <input className="input" inputMode="numeric" autoComplete="one-time-code" placeholder="6 dígitos" value={codigo} onChange={(e) => setCodigo(e.target.value)} autoFocus /></div>
          )}
          {error && <div className="alert alert-bad">{error}</div>}
          {msg && <div className="alert alert-ok">{msg}</div>}
          <button className="btn btn-accent" disabled={busy}>{busy ? '…' : titulo}</button>
        </form>
        <div className="center sm muted stack" style={{ '--gap': '6px' }}>
          {modo === 'login' && (
            <>
              <div>¿No tienes cuenta? <button className="link-btn" onClick={() => ir('registro')}>Regístrate</button></div>
              <div><button className="link-btn" onClick={() => ir('forgot')}>¿Olvidaste tu contraseña?</button></div>
            </>
          )}
          {modo === 'registro' && <div>¿Ya tienes cuenta? <button className="link-btn" onClick={() => ir('login')}>Inicia sesión</button></div>}
          {modo === 'forgot' && <div><button className="link-btn" onClick={() => ir('login')}>← Volver</button></div>}
        </div>
        <div className="center sm muted"><a className="link-btn" href="/legal">Términos y Privacidad</a></div>
      </div>
    </div>
  )
}
