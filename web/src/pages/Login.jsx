import { useState } from 'react'
import { post, setToken } from '../api'

export default function Login({ onAuth }) {
  const [modo, setModo] = useState('login') // login | registro
  const [form, setForm] = useState({ nombre: '', email: '', password: '' })
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState(false)
  const set = (k) => (e) => setForm({ ...form, [k]: e.target.value })

  const submit = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null)
    try {
      const path = modo === 'login' ? '/auth/login' : '/auth/registro'
      const body = modo === 'login'
        ? { email: form.email, password: form.password }
        : { nombre: form.nombre, email: form.email, password: form.password }
      const r = await post(path, body)
      setToken(r.token)
      onAuth()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

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
          <div><label>Contraseña</label><input className="input" type="password" value={form.password} onChange={set('password')} required minLength={8} /></div>
          {error && <div className="alert alert-bad">{error}</div>}
          <button className="btn btn-accent" disabled={busy}>{busy ? '…' : modo === 'login' ? 'Entrar' : 'Crear cuenta'}</button>
        </form>
        <div className="center sm muted">
          {modo === 'login'
            ? <>¿No tienes cuenta? <button className="link-btn" onClick={() => { setModo('registro'); setError(null) }}>Regístrate</button></>
            : <>¿Ya tienes cuenta? <button className="link-btn" onClick={() => { setModo('login'); setError(null) }}>Inicia sesión</button></>}
        </div>
      </div>
    </div>
  )
}
