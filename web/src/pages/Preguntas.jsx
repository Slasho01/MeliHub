import { useEffect, useState } from 'react'
import { get, post } from '../api'

function fechaCorta(iso) {
  if (!iso) return ''
  const d = new Date(iso)
  if (isNaN(d)) return ''
  return d.toLocaleString('es-CL', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' })
}

export default function Preguntas() {
  const [cuenta, setCuenta] = useState('') // '' = todas
  const [cuentas, setCuentas] = useState([])
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)
  const [respuestas, setRespuestas] = useState({}) // id -> texto
  const [enviando, setEnviando] = useState(0) // id de la pregunta en curso

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])

  const cargar = () => {
    setCargando(true)
    setError('')
    const qs = cuenta ? `?cuenta=${cuenta}` : ''
    return get(`/ml/preguntas${qs}`)
      .then(setRows)
      .catch((e) => setError(e.message))
      .finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const responder = async (q) => {
    const texto = (respuestas[q.id] || '').trim()
    if (!texto) return
    setEnviando(q.id)
    setError('')
    try {
      await post('/ml/preguntas/responder', { cuenta_id: q.cuenta_id, question_id: q.id, texto })
      setRows((rs) => (rs || []).filter((x) => x.id !== q.id))
      setRespuestas((m) => { const n = { ...m }; delete n[q.id]; return n })
    } catch (e) {
      setError(e.message)
    } finally {
      setEnviando(0)
    }
  }

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Preguntas</h1>
          <p className="muted sm" style={{ margin: 0 }}>Preguntas de compradores sin responder, centralizadas de todas tus cuentas de Mercado Libre.</p>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)} title="Cuenta de Mercado Libre">
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <button className="btn btn-sm" onClick={cargar} disabled={cargando} title="Vuelve a consultar a Mercado Libre">↻ Actualizar</button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : rows.length === 0 ? (
        <div className="empty">No tienes preguntas sin responder. 🎉</div>
      ) : (
        <div className="stack" style={{ gap: 12 }}>
          {rows.map((q) => (
            <div key={`${q.cuenta_id}-${q.id}`} className="card stack" style={{ gap: 8 }}>
              <div className="row between" style={{ flexWrap: 'wrap', gap: 6 }}>
                <span className="strong">{q.producto || q.item_id}</span>
                <span className="tag">{q.cuenta || 'cuenta'}</span>
              </div>
              <p style={{ margin: 0 }}>{q.texto}</p>
              <div className="muted xs">{fechaCorta(q.fecha)}{q.comprador_id ? ` · Comprador #${q.comprador_id}` : ''}</div>
              <textarea
                className="input"
                rows={2}
                maxLength={2000}
                placeholder="Escribe tu respuesta…"
                value={respuestas[q.id] || ''}
                onChange={(e) => setRespuestas((m) => ({ ...m, [q.id]: e.target.value }))}
              />
              <div className="row" style={{ justifyContent: 'flex-end' }}>
                <button
                  className="btn btn-sm btn-accent"
                  disabled={enviando === q.id || !(respuestas[q.id] || '').trim()}
                  onClick={() => responder(q)}
                >
                  {enviando === q.id ? 'Enviando…' : 'Responder'}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
