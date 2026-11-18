import { useEffect, useState } from 'react'
import { get, post, setToken } from '../api'
import PasswordMeter from '../components/PasswordMeter'
import Turnstile from '../components/Turnstile'

export default function Login({ onAuth }) {
  const [modo, setModo] = useState(() => (new URLSearchParams(window.location.search).get('r') === '1' ? 'registro' : 'login')) // login | registro | forgot
  const [form, setForm] = useState({ nombre: '', email: '', password: '' })
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)
  const [need2fa, setNeed2fa] = useState(false)
  const [codigo, setCodigo] = useState('')
  const [pending, setPending] = useState('') // token temporal del 2FA (no se reenvía la contraseña)
  const [reenviar, setReenviar] = useState(null) // email al que reenviar la verificación
  const [siteKey, setSiteKey] = useState('') // Turnstile site key (vacío = CAPTCHA off)
  const [captcha, setCaptcha] = useState('')
  const [capKey, setCapKey] = useState(0) // remonta el widget para resetear el token
  useEffect(() => { get('/config').then((c) => setSiteKey(c?.turnstile_site_key || '')).catch(() => {}) }, [])
  const resetCaptcha = () => { setCaptcha(''); setCapKey((k) => k + 1) }
  const set = (k) => (e) => setForm({ ...form, [k]: e.target.value })
  const ir = (m) => { setModo(m); setError(null); setMsg(null); setNeed2fa(false); setCodigo(''); setPending(''); setReenviar(null) }

  const reenviarVerif = async () => {
    try { await post('/auth/reenviar-verificacion', { email: reenviar }); setMsg('Te reenviamos el correo de verificación. Revisa tu bandeja (y la carpeta de spam).'); setError(null); setReenviar(null) }
    catch (e) { setError(e.message) }
  }

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
      // 2º paso del 2FA: solo el token temporal + el código (sin reenviar email/contraseña).
      if (modo === 'login' && need2fa) {
        const r = await post('/auth/login/2fa', { pending, codigo })
        setToken(r.token)
        onAuth()
        return
      }
      if (siteKey && !captcha) { setError('Completa el CAPTCHA para continuar.'); setBusy(false); return }
      const path = modo === 'login' ? '/auth/login' : '/auth/registro'
      const body = modo === 'login'
        ? { email: form.email, password: form.password, captcha }
        : { nombre: form.nombre, email: form.email, password: form.password, captcha }
      const r = await post(path, body)
      if (modo === 'login' && r.twofa_required) { setNeed2fa(true); setPending(r.pending || ''); setBusy(false); return }
      if (modo === 'registro' && r.requiere_verificacion) {
        setMsg(`¡Cuenta creada! Te enviamos un correo de verificación a ${r.email || form.email}. Revísalo (y la carpeta de spam) para activar tu cuenta.`)
        setModo('login'); setForm({ ...form, password: '' }); setBusy(false); return
      }
      setToken(r.token)
      onAuth()
    } catch (err) {
      setError(err.message)
      if (err.message && /verifica tu correo/i.test(err.message)) setReenviar(form.email)
      resetCaptcha()
    } finally { setBusy(false) }
  }

  const titulo = { login: 'Entrar', registro: 'Crear cuenta', forgot: 'Enviar enlace' }[modo]

  return (
    <div className="auth-wrap">
      <div className="card auth-card stack">
        <div className="center">
          <div className="brand" style={{ fontSize: 24 }}>Vent<b>oki</b></div>
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
          {siteKey && modo !== 'forgot' && !need2fa && (
            <Turnstile key={capKey} siteKey={siteKey} onToken={setCaptcha} />
          )}
          {error && <div className="alert alert-bad">{error}</div>}
          {reenviar && <div className="center sm"><button type="button" className="link-btn" onClick={reenviarVerif}>Reenviar correo de verificación</button></div>}
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
        <div className="center sm muted" style={{ borderTop: '1px solid var(--line)', paddingTop: 12, marginTop: 2 }}>
          <a className="link-btn" href="/docs">Documentación de la API</a>
          <span style={{ margin: '0 8px' }}>·</span>
          <a className="link-btn" href="/ventoki-sync.zip" download>⬇ Plugin de WordPress</a>
        </div>
      </div>
    </div>
  )
}
