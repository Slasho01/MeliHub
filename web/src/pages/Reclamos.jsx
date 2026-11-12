import { useEffect, useState } from 'react'
import { get, post } from '../api'

const fechaCorta = (iso) => { if (!iso) return ''; const d = new Date(iso); return isNaN(d) ? '' : d.toLocaleString('es-CL', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }) }

const estadoTxt = (s) => ({ opened: 'Abierto', closed: 'Cerrado', in_process: 'En proceso', pending: 'Pendiente' }[s] || s || '—')
const etapaTxt = (s) => ({ claim: 'Reclamo', dispute: 'Disputa', recontact: 'Recontacto', stale: 'Inactivo', none: '' }[s] || s || '')
const tipoTxt = (s) => ({ mediations: 'Mediación', return: 'Devolución', change: 'Cambio', cancel_purchase: 'Cancelación', fulfillment: 'Logística', service: 'Servicio' }[s] || s || 'Reclamo')
const estadoColor = (s) => (s === 'closed' ? 'var(--muted)' : s === 'opened' ? 'var(--bad, #d93025)' : 'var(--warn)')

const mensajesDe = (d) => { const m = d?.mensajes; return m?.data || m?.messages || m?.results || (Array.isArray(m) ? m : []) }
const textoMsg = (m) => (typeof m?.message === 'string' ? m.message : (m?.message?.text || m?.text || ''))
const rolMsg = (m) => (m?.sender_role || m?.from?.role || m?.stage || '')

export default function Reclamos() {
  const [cuenta, setCuenta] = useState('')
  const [cuentas, setCuentas] = useState([])
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)
  const [abierto, setAbierto] = useState(null)
  const [detalle, setDetalle] = useState({})
  const [respuestas, setRespuestas] = useState({})
  const [enviando, setEnviando] = useState(0)

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])

  const cargar = () => {
    setCargando(true); setError('')
    const qs = cuenta ? `?cuenta=${cuenta}` : ''
    return get(`/ml/reclamos${qs}`).then(setRows).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const abrir = async (r) => {
    if (abierto === r.id) { setAbierto(null); return }
    setAbierto(r.id)
    if (!detalle[r.id]) {
      try { const d = await get(`/ml/reclamos/${r.cuenta_id}/${r.id}`); setDetalle((m) => ({ ...m, [r.id]: d })) }
      catch (e) { setError(e.message) }
    }
  }

  const responder = async (r) => {
    const texto = (respuestas[r.id] || '').trim()
    if (!texto) return
    setEnviando(r.id); setError('')
    try {
      await post('/ml/reclamos/responder', { cuenta_id: r.cuenta_id, claim_id: r.id, texto })
      setRespuestas((m) => { const n = { ...m }; delete n[r.id]; return n })
      const d = await get(`/ml/reclamos/${r.cuenta_id}/${r.id}`); setDetalle((m) => ({ ...m, [r.id]: d }))
    } catch (e) { setError(e.message) } finally { setEnviando(0) }
  }

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Reclamos</h1>
          <p className="muted sm" style={{ margin: 0 }}>Reclamos post-venta de tus compradores, centralizados de todas tus cuentas de Mercado Libre.</p>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)} title="Cuenta de Mercado Libre">
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : rows.length === 0 ? (
        <div className="empty">No tienes reclamos. 🎉</div>
      ) : (
        <div className="stack" style={{ gap: 12 }}>
          {rows.map((r) => {
            const d = detalle[r.id]
            const msgs = mensajesDe(d)
            return (
              <div key={`${r.cuenta_id}-${r.id}`} className="card stack" style={{ gap: 8 }}>
                <div className="row between" style={{ flexWrap: 'wrap', gap: 6 }}>
                  <div className="row" style={{ gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
                    <span className="strong">{tipoTxt(r.tipo)}</span>
                    <span className="badge" style={{ color: estadoColor(r.estado) }}>{estadoTxt(r.estado)}</span>
                    {etapaTxt(r.etapa) && <span className="tag">{etapaTxt(r.etapa)}</span>}
                  </div>
                  <span className="tag">{r.cuenta || 'cuenta'}</span>
                </div>
                <div className="muted sm">
                  Reclamo #{r.id}{r.recurso_id ? ` · Orden ${r.recurso_id}` : ''}{r.motivo ? ` · Motivo ${r.motivo}` : ''}
                </div>
                <div className="muted xs">{fechaCorta(r.fecha)}{r.comprador_id ? ` · Comprador #${r.comprador_id}` : ''}</div>

                <div className="row" style={{ justifyContent: 'flex-end' }}>
                  <button className="btn btn-sm" onClick={() => abrir(r)}>{abierto === r.id ? 'Ocultar' : 'Ver conversación'}</button>
                </div>

                {abierto === r.id && (
                  <div className="stack" style={{ gap: 8, borderTop: '1px solid var(--line)', paddingTop: 8 }}>
                    {!d ? (
                      <div className="muted sm">Cargando conversación…</div>
                    ) : msgs.length === 0 ? (
                      <div className="muted sm">Sin mensajes en este reclamo todavía.</div>
                    ) : (
                      <div className="stack" style={{ gap: 6 }}>
                        {msgs.map((m, i) => (
                          <div key={i} className="sm" style={{ padding: '6px 10px', borderRadius: 8, background: 'var(--bg)' }}>
                            <div className="muted xs" style={{ marginBottom: 2 }}>{rolMsg(m) === 'respondent' || rolMsg(m) === 'seller' ? 'Tú' : 'Comprador'} · {fechaCorta(m.date_created || m.date)}</div>
                            {textoMsg(m) || <span className="muted">(adjunto / sin texto)</span>}
                          </div>
                        ))}
                      </div>
                    )}
                    {r.estado !== 'closed' && (
                      <>
                        <textarea className="input" rows={2} maxLength={2000} placeholder="Escribe un mensaje al comprador…"
                          value={respuestas[r.id] || ''} onChange={(e) => setRespuestas((m) => ({ ...m, [r.id]: e.target.value }))} />
                        <div className="row" style={{ justifyContent: 'flex-end' }}>
                          <button className="btn btn-sm btn-accent" disabled={enviando === r.id || !(respuestas[r.id] || '').trim()} onClick={() => responder(r)}>
                            {enviando === r.id ? 'Enviando…' : 'Enviar mensaje'}
                          </button>
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
