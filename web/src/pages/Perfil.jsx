import { useEffect, useState } from 'react'
import { get, put, post, setToken } from '../api'
import PasswordMeter, { passwordOk } from '../components/PasswordMeter'

export default function Perfil() {
  const [v, setV] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [pwd, setPwd] = useState({ actual: '', nueva: '' })
  const [tf, setTf] = useState(null) // { secret, url } durante la configuración
  const [tfCode, setTfCode] = useState('')

  const recargarMe = () => get('/auth/me').then(setV).catch((e) => setError(e.message))
  useEffect(() => { recargarMe() }, [])

  const tfSetup = async () => {
    setError(null); setMsg(null)
    try { setTf(await post('/auth/2fa/setup', {})) } catch (e) { setError(e.message) }
  }
  const tfEnable = async () => {
    setError(null); setMsg(null)
    try { await post('/auth/2fa/enable', { codigo: tfCode }); setTf(null); setTfCode(''); setMsg('2FA activado.'); recargarMe() }
    catch (e) { setError(e.message) }
  }
  const tfDisable = async () => {
    setError(null); setMsg(null)
    const codigo = window.prompt('Ingresa un código de tu app para desactivar el 2FA:')
    if (codigo == null) return
    try { await post('/auth/2fa/disable', { codigo }); setMsg('2FA desactivado.'); recargarMe() }
    catch (e) { setError(e.message) }
  }

  const guardarPerfil = async () => {
    setError(null); setMsg(null)
    try { const u = await put('/auth/perfil', { nombre: v.nombre, email: v.email }); setV({ ...v, ...u }); setMsg('Perfil actualizado.') }
    catch (e) { setError(e.message) }
  }

  const cerrarTodo = async () => {
    if (!window.confirm('Esto cerrará tu sesión en TODOS los dispositivos (incluido este). Tendrás que volver a entrar. ¿Continuar?')) return
    try { await post('/auth/logout-all', {}); setToken(null); window.location.href = '/' } catch (e) { setError(e.message) }
  }

  const cambiarPwd = async () => {
    setError(null); setMsg(null)
    if (!passwordOk(pwd.nueva)) { setError('La nueva contraseña debe tener al menos 8 caracteres, con letras y números.'); return }
    try { await put('/auth/password', pwd); setMsg('Contraseña cambiada.'); setPwd({ actual: '', nueva: '' }) }
    catch (e) { setError(e.message) }
  }

  if (!v) return <div className="empty">Cargando…</div>
  return (
    <div className="stack" style={{ maxWidth: 520 }}>
      <div><h1>Mi cuenta</h1><p className="muted sm" style={{ margin: 0 }}>Tu perfil y tu contraseña.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="card stack">
        <b>Perfil</b>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Nombre</span>
          <input className="input" value={v.nombre} onChange={(e) => setV({ ...v, nombre: e.target.value })} /></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Email</span>
          <input className="input" type="email" value={v.email} onChange={(e) => setV({ ...v, email: e.target.value })} /></label>
        <div className="sm muted">Rol: <b>{v.es_miembro ? v.rol_equipo : (v.es_admin ? 'admin' : 'dueño')}</b> · Plan: <b>{v.plan}</b></div>
        <div><button className="btn btn-accent" onClick={guardarPerfil}>Guardar perfil</button></div>
      </div>

      <div className="card stack">
        <b>Cambiar contraseña</b>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Contraseña actual</span>
          <input className="input" type="password" autoComplete="current-password" value={pwd.actual} onChange={(e) => setPwd({ ...pwd, actual: e.target.value })} /></label>
        <label className="stack" style={{ gap: 2 }}><span className="sm muted">Nueva contraseña</span>
          <input className="input" type="password" autoComplete="new-password" value={pwd.nueva} onChange={(e) => setPwd({ ...pwd, nueva: e.target.value })} />
          <PasswordMeter value={pwd.nueva} /></label>
        <div><button className="btn btn-accent" onClick={cambiarPwd}>Cambiar contraseña</button></div>
        <hr style={{ border: 0, borderTop: '1px solid var(--line)', margin: '4px 0' }} />
        <div className="sm muted">¿Dejaste la sesión abierta en otro lado?</div>
        <div><button className="btn btn-sm btn-danger" onClick={cerrarTodo}>Cerrar sesión en todos los dispositivos</button></div>
      </div>

      <div className="card stack">
        <div className="row between" style={{ alignItems: 'center' }}>
          <b>Verificación en dos pasos (2FA)</b>
          {v.twofa_enabled ? <span className="tag tag-ok">activado</span> : <span className="tag">desactivado</span>}
        </div>
        {v.twofa_enabled ? (
          <>
            <p className="muted sm" style={{ margin: 0 }}>Al iniciar sesión te pediremos un código de tu app de autenticación.</p>
            <div><button className="btn btn-danger btn-sm" onClick={tfDisable}>Desactivar 2FA</button></div>
          </>
        ) : tf ? (
          <>
            <p className="muted sm" style={{ margin: 0 }}>1) Escanea el QR con Google Authenticator / Authy:</p>
            {tf.qr && <img src={`data:image/png;base64,${tf.qr}`} alt="Código QR 2FA" width="180" height="180" style={{ borderRadius: 8, background: '#fff', padding: 6 }} />}
            <details><summary className="sm muted" style={{ cursor: 'pointer' }}>¿No puedes escanear? Usa la clave manual</summary>
              <code style={{ display: 'block', background: 'var(--bg)', padding: '8px 10px', borderRadius: 8, wordBreak: 'break-all', marginTop: 6 }}>{tf.secret}</code>
            </details>
            <label className="stack" style={{ gap: 2 }}><span className="sm muted">2) Ingresa el código de 6 dígitos</span>
              <input className="input" inputMode="numeric" value={tfCode} onChange={(e) => setTfCode(e.target.value)} placeholder="000000" /></label>
            <div className="row" style={{ gap: 8 }}>
              <button className="btn btn-accent btn-sm" onClick={tfEnable}>Activar</button>
              <button className="btn btn-sm" onClick={() => { setTf(null); setTfCode('') }}>Cancelar</button>
            </div>
          </>
        ) : (
          <>
            <p className="muted sm" style={{ margin: 0 }}>Agrega una capa extra de seguridad con una app de autenticación.</p>
            <div><button className="btn btn-accent btn-sm" onClick={tfSetup}>Activar 2FA</button></div>
          </>
        )}
      </div>
    </div>
  )
}
