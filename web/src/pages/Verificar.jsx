import { useEffect, useState } from 'react'
import { post, setToken } from '../api'

export default function Verificar({ onAuth }) {
  const [estado, setEstado] = useState('verificando') // verificando | ok | error
  const [error, setError] = useState('')

  useEffect(() => {
    const token = new URLSearchParams(window.location.search).get('token')
    if (!token) { setEstado('error'); setError('Falta el token de verificación en el enlace.'); return }
    post('/auth/verificar', { token })
      .then((r) => {
        setEstado('ok')
        if (r.token) { setToken(r.token); setTimeout(() => onAuth && onAuth(), 1300) }
      })
      .catch((e) => { setEstado('error'); setError(e.message || 'Enlace inválido o expirado.') })
  }, [])

  return (
    <div className="auth-wrap">
      <div className="card auth-card stack center">
        <div className="brand" style={{ fontSize: 24 }}>Vent<b>oki</b></div>
        {estado === 'verificando' && <p className="muted" style={{ margin: 0 }}>Verificando tu cuenta…</p>}
        {estado === 'ok' && (
          <>
            <div style={{ fontSize: 42 }}>✅</div>
            <h2 style={{ margin: 0 }}>¡Cuenta verificada!</h2>
            <p className="muted sm" style={{ margin: 0 }}>Entrando a tu panel…</p>
          </>
        )}
        {estado === 'error' && (
          <>
            <div style={{ fontSize: 42 }}>⚠️</div>
            <h2 style={{ margin: 0 }}>No se pudo verificar</h2>
            <div className="alert alert-bad" style={{ width: '100%' }}>{error}</div>
            <p className="muted sm" style={{ margin: 0 }}>El enlace pudo expirar. Inicia sesión y reenvía la verificación.</p>
            <a className="btn btn-accent" href="/login" style={{ width: '100%', textAlign: 'center' }}>Ir a iniciar sesión</a>
          </>
        )}
      </div>
    </div>
  )
}
