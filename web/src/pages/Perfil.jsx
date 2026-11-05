import { useEffect, useState } from 'react'
import { get, put } from '../api'

export default function Perfil() {
  const [v, setV] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [pwd, setPwd] = useState({ actual: '', nueva: '' })

  useEffect(() => { get('/auth/me').then(setV).catch((e) => setError(e.message)) }, [])

  const guardarPerfil = async () => {
    setError(null); setMsg(null)
    try { const u = await put('/auth/perfil', { nombre: v.nombre, email: v.email }); setV(u); setMsg('Perfil actualizado.') }
    catch (e) { setError(e.message) }
  }

  const cambiarPwd = async () => {
    setError(null); setMsg(null)
    if (pwd.nueva.length < 8) { setError('La nueva contraseña debe tener al menos 8 caracteres.'); return }
    try { await put('/auth/password', pwd); setMsg('Contraseña cambiada.'); setPwd({ actual: '', nueva: '' }) }
    catch (e) { setError(e.message) }
  }

  if (!v) return <div className="empty">Cargando…</div>
  return (
    <div className="stack" style={{ maxWidth: 480 }}>
      <div><h1>Mi cuenta</h1><p className="muted sm" style={{ margin: 0 }}>Edita tu perfil y tu contraseña.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="card stack">
        <b>Perfil</b>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Nombre</span>
          <input className="input" value={v.nombre} onChange={(e) => setV({ ...v, nombre: e.target.value })} /></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Email</span>
          <input className="input" type="email" value={v.email} onChange={(e) => setV({ ...v, email: e.target.value })} /></label>
        <div className="sm muted">Rol: <b>{v.rol}</b> · Plan: <b>{v.plan}</b></div>
        <div><button className="btn btn-accent" onClick={guardarPerfil}>Guardar perfil</button></div>
      </div>

      <div className="card stack">
        <b>Cambiar contraseña</b>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Contraseña actual</span>
          <input className="input" type="password" autoComplete="current-password" value={pwd.actual} onChange={(e) => setPwd({ ...pwd, actual: e.target.value })} /></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Nueva contraseña (mín. 8)</span>
          <input className="input" type="password" autoComplete="new-password" value={pwd.nueva} onChange={(e) => setPwd({ ...pwd, nueva: e.target.value })} /></label>
        <div><button className="btn btn-accent" onClick={cambiarPwd}>Cambiar contraseña</button></div>
      </div>
    </div>
  )
}
