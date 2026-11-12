import { useEffect, useState } from 'react'
import { get, put, post } from '../api'

export default function Correo() {
  const [cfg, setCfg] = useState(null)
  const [form, setForm] = useState({ proveedor: 'resend', resend_api_key: '', smtp_host: '', smtp_port: 587, smtp_usuario: '', smtp_password: '', smtp_tls: true, remitente: '' })
  const [destino, setDestino] = useState('')
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)
  const [probando, setProbando] = useState(false)

  useEffect(() => {
    get('/admin/correo').then((c) => {
      setCfg(c)
      setForm((f) => ({ ...f, proveedor: c.proveedor || 'resend', smtp_host: c.smtp_host || '', smtp_port: c.smtp_port || 587, smtp_usuario: c.smtp_usuario || '', smtp_tls: c.smtp_tls ?? true, remitente: c.remitente || '' }))
    }).catch((e) => setError(e.message))
  }, [])

  const set = (k, v) => setForm((f) => ({ ...f, [k]: v }))

  const guardar = async (e) => {
    e.preventDefault()
    setBusy(true); setError(''); setMsg('')
    try {
      // Secretos vacíos no se envían (se conserva lo guardado).
      const body = { ...form, smtp_port: Number(form.smtp_port) || 587 }
      if (!body.resend_api_key) delete body.resend_api_key
      if (!body.smtp_password) delete body.smtp_password
      await put('/admin/correo', body)
      setMsg('Configuración guardada.')
      const c = await get('/admin/correo'); setCfg(c); setForm((f) => ({ ...f, resend_api_key: '', smtp_password: '' }))
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  const probar = async () => {
    if (!destino.trim()) { setError('Indica un correo de destino para la prueba.'); return }
    setProbando(true); setError(''); setMsg('')
    try { await post('/admin/correo/test', { destino }); setMsg(`Correo de prueba enviado a ${destino}. Revisa la bandeja (y spam).`) }
    catch (e) { setError(e.message) } finally { setProbando(false) }
  }

  const esSmtp = form.proveedor === 'smtp'

  return (
    <div className="stack" style={{ maxWidth: 640 }}>
      <div>
        <h1 style={{ margin: 0 }}>Correo</h1>
        <p className="muted sm" style={{ margin: 0 }}>Configura cómo MeliHub envía correos (notificaciones, invitaciones, reset de clave).</p>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <form className="card stack" style={{ gap: 12 }} onSubmit={guardar}>
        <label>Proveedor
          <select className="input" value={form.proveedor} onChange={(e) => set('proveedor', e.target.value)}>
            <option value="resend">Resend (HTTP · funciona en Render)</option>
            <option value="smtp">SMTP propio (requiere servidor sin bloqueo SMTP, p. ej. VPS)</option>
          </select>
        </label>

        {esSmtp && (
          <div className="alert" style={{ borderLeft: '3px solid var(--warn)' }}>
            ⚠️ Muchos hosts (como <b>Render</b>) <b>bloquean SMTP saliente</b>. Si MeliHub corre ahí, usa Resend; SMTP solo funciona en un servidor que lo permita (VPS).
          </div>
        )}

        <label>Remitente <span className="muted xs">(formato: Nombre &lt;correo@tudominio.cl&gt;)</span>
          <input className="input" placeholder="MeliHub <ventas@tudominio.cl>" value={form.remitente} onChange={(e) => set('remitente', e.target.value)} />
        </label>

        {!esSmtp ? (
          <label>API key de Resend {cfg?.tiene_resend_key && <span className="muted xs">· guardada (deja vacío para conservar)</span>}
            <input className="input" type="password" autoComplete="off" placeholder={cfg?.tiene_resend_key ? '•••••••• (guardada)' : 're_...'} value={form.resend_api_key} onChange={(e) => set('resend_api_key', e.target.value)} />
          </label>
        ) : (
          <>
            <div className="row" style={{ gap: 8 }}>
              <label style={{ flex: 2 }}>Host SMTP<input className="input" placeholder="mail.tudominio.cl" value={form.smtp_host} onChange={(e) => set('smtp_host', e.target.value)} /></label>
              <label style={{ flex: 1 }}>Puerto<input className="input" type="number" value={form.smtp_port} onChange={(e) => set('smtp_port', e.target.value)} /></label>
            </div>
            <label>Usuario<input className="input" autoComplete="off" placeholder="usuario@tudominio.cl" value={form.smtp_usuario} onChange={(e) => set('smtp_usuario', e.target.value)} /></label>
            <label>Contraseña {cfg?.tiene_smtp_password && <span className="muted xs">· guardada (deja vacío para conservar)</span>}
              <input className="input" type="password" autoComplete="off" placeholder={cfg?.tiene_smtp_password ? '•••••••• (guardada)' : ''} value={form.smtp_password} onChange={(e) => set('smtp_password', e.target.value)} />
            </label>
            <label className="row" style={{ gap: 8, alignItems: 'center', fontWeight: 400 }}>
              <input type="checkbox" checked={form.smtp_tls} onChange={(e) => set('smtp_tls', e.target.checked)} />
              <span className="sm">Usar TLS/SSL (587 = STARTTLS · 465 = TLS implícito)</span>
            </label>
          </>
        )}

        <button className="btn btn-dark" disabled={busy}>{busy ? 'Guardando…' : 'Guardar'}</button>
      </form>

      <div className="card stack" style={{ gap: 8 }}>
        <b className="sm">Enviar correo de prueba</b>
        <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
          <input className="input" style={{ flex: 1, minWidth: 180 }} placeholder="tucorreo@ejemplo.com" value={destino} onChange={(e) => setDestino(e.target.value)} />
          <button className="btn btn-accent" disabled={probando} onClick={probar}>{probando ? 'Enviando…' : 'Enviar prueba'}</button>
        </div>
        <span className="muted xs">Guarda la configuración antes de probar.</span>
      </div>
    </div>
  )
}
