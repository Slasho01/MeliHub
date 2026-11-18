import { useState } from 'react'
import { Link } from 'react-router-dom'
import { post } from '../api'

export default function Baja() {
  const token = new URLSearchParams(window.location.search).get('t') || ''
  const [estado, setEstado] = useState('idle') // idle | busy | ok | error
  const [msg, setMsg] = useState('')

  const darDeBaja = async () => {
    setEstado('busy'); setMsg('')
    try {
      await post('/promo/baja', { token })
      setEstado('ok')
    } catch (e) { setEstado('error'); setMsg(e.message) }
  }

  return (
    <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', padding: 24 }}>
      <div className="card stack" style={{ maxWidth: 420, textAlign: 'center', gap: 14 }}>
        <div style={{ fontSize: 24, fontWeight: 800 }}>Vent<b style={{ color: 'var(--accent)' }}>oki</b></div>
        {estado === 'ok' ? (
          <>
            <h2 style={{ margin: 0 }}>Listo ✓</h2>
            <p className="muted">Te diste de baja. No volverás a recibir correos de promoción de Ventoki.</p>
          </>
        ) : !token ? (
          <p className="muted">Enlace inválido o incompleto.</p>
        ) : (
          <>
            <h2 style={{ margin: 0 }}>¿Darte de baja?</h2>
            <p className="muted">Dejarás de recibir correos de promoción de Ventoki. Los correos de tu cuenta (si tienes una) no se ven afectados.</p>
            {estado === 'error' && <div className="alert alert-bad">{msg}</div>}
            <button className="btn btn-accent" disabled={estado === 'busy'} onClick={darDeBaja}>{estado === 'busy' ? 'Procesando…' : 'Confirmar baja'}</button>
          </>
        )}
        <Link to="/" className="muted sm">Ir a Ventoki</Link>
      </div>
    </div>
  )
}
