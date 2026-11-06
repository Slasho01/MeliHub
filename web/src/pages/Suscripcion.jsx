import { useEffect, useState } from 'react'
import { get, post, money } from '../api'

const fecha = (s) => (s ? new Date(s + 'Z').toLocaleDateString('es-CL', { day: '2-digit', month: 'long', year: 'numeric' }) : '—')
const ESTADO = { authorized: ['Activa', 'tag-ok'], pending: ['Pendiente de pago', 'tag'], paused: ['Pausada', 'tag'], cancelled: ['Cancelada', 'tag'] }

function Fila({ k, children }) {
  return (
    <div className="row" style={{ justifyContent: 'space-between', borderBottom: '1px solid var(--line)', padding: '8px 0' }}>
      <span className="muted sm">{k}</span>
      <span style={{ textAlign: 'right' }}>{children}</span>
    </div>
  )
}

export default function Suscripcion() {
  const [sub, setSub] = useState(undefined) // undefined = cargando, null = sin suscripción
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)

  const cargar = () => get('/suscripcion').then((r) => setSub(r.suscripcion)).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const cancelar = async () => {
    if (!window.confirm('¿Cancelar tu suscripción? Volverás al plan Free al finalizar el período vigente.')) return
    setBusy(true); setError(null); setMsg(null)
    try { await post('/suscripcion/cancelar', {}); setMsg('Suscripción cancelada.'); cargar() }
    catch (e) { setError(e.message) } finally { setBusy(false) }
  }

  if (sub === undefined) return <div className="empty">Cargando…</div>
  const [estLabel, estTag] = ESTADO[sub?.estado] || [sub?.estado, 'tag']
  const activa = sub && sub.estado === 'authorized'

  return (
    <div className="stack" style={{ maxWidth: 520 }}>
      <div className="row between" style={{ alignItems: 'flex-start' }}>
        <div><h1>Suscripción</h1><p className="muted sm" style={{ margin: 0 }}>Estado de tu plan de pago.</p></div>
        <button className="btn btn-sm" disabled={busy} onClick={cargar}>Actualizar estado</button>
      </div>
      <p className="muted sm" style={{ marginTop: -6 }}>Si acabas de pagar y aún aparece pendiente, pulsa «Actualizar estado».</p>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {!sub ? (
        <div className="card stack">
          <b>No tienes una suscripción activa.</b>
          <p className="muted sm" style={{ margin: 0 }}>Estás en el plan gratuito.</p>
          <div><a className="btn btn-accent btn-sm" href="/plan">Ver planes</a></div>
        </div>
      ) : (
        <div className="card stack">
          <Fila k="Plan"><b style={{ textTransform: 'capitalize' }}>{sub.plan}</b></Fila>
          <Fila k="Estado"><span className={`tag ${estTag}`}>{estLabel}</span></Fila>
          <Fila k="Ciclo"><span style={{ textTransform: 'capitalize' }}>{sub.ciclo || 'mensual'}</span></Fila>
          <Fila k="Monto">{money(sub.monto)} {sub.ciclo === 'anual' ? '/año' : '/mes'}</Fila>
          {sub.cupon && <Fila k="Cupón aplicado">{sub.cupon}</Fila>}
          <Fila k="Fecha de inicio">{fecha(sub.created_at)}</Fila>
          <Fila k={activa ? 'Vigente hasta' : 'Venció / vence'}>{fecha(sub.vigente_hasta)}</Fila>
          {activa && (
            <div style={{ paddingTop: 10 }}>
              <button className="btn btn-danger" disabled={busy} onClick={cancelar}>{busy ? 'Cancelando…' : 'Cancelar suscripción'}</button>
            </div>
          )}
          {!activa && (
            <div style={{ paddingTop: 10 }}><a className="btn btn-accent btn-sm" href="/plan">Ver planes</a></div>
          )}
        </div>
      )}
    </div>
  )
}
