import { useEffect, useState } from 'react'
import { get, post, money } from '../api'

const estadoInfo = (s) => ({
  winning: ['Ganando', 'var(--ok)'],
  sharing_first_place: ['Empatado 1º', 'var(--warn)'],
  competing: ['Perdiendo', 'var(--bad, #d93025)'],
  not_winning: ['Perdiendo', 'var(--bad, #d93025)'],
  listed: ['Listado', 'var(--muted)'],
  not_listed: ['No listado', 'var(--muted)'],
  unknown: ['Sin dato', 'var(--muted)'],
}[s] || [s || '—', 'var(--muted)'])

const Precio = ({ label, v, warn }) => (
  <div style={{ minWidth: 76 }}>
    <div className="muted xs">{label}</div>
    <div style={{ fontWeight: 600, color: warn ? 'var(--bad, #d93025)' : undefined, whiteSpace: 'nowrap' }}>{v != null ? money(v) : '—'}</div>
  </div>
)

export default function Catalogo() {
  const [cuenta, setCuenta] = useState('')
  const [cuentas, setCuentas] = useState([])
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)
  const [busy, setBusy] = useState('')
  const [msg, setMsg] = useState('')

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])

  const cargar = () => {
    setCargando(true); setError(''); setMsg('')
    const qs = cuenta ? `?cuenta=${cuenta}` : ''
    return get(`/ml/catalogo${qs}`).then(setRows).catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const igualar = async (r) => {
    const precio = Math.round(r.precio_para_ganar)
    if (!precio) return
    if (!window.confirm(`¿Dejar "${r.producto}" en ${money(precio)} para ganar la Buy Box en Mercado Libre?`)) return
    setBusy(r.item_id); setError(''); setMsg('')
    try {
      await post('/ml/catalogo/igualar', { cuenta_id: r.cuenta_id, item_id: r.item_id, precio })
      setMsg(`Precio actualizado a ${money(precio)}. La posición se recalcula en unos minutos.`)
      setTimeout(cargar, 1500)
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const ganando = (rows || []).filter((r) => r.estado === 'winning' || r.estado === 'sharing_first_place').length

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Publicaciones de catálogo</h1>
          <p className="muted sm" style={{ margin: 0 }}>Tus publicaciones de catálogo y si ganas la Buy Box. Iguala el precio para ganar sin crear nada nuevo.</p>
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
      {msg && <div className="alert alert-ok">{msg}</div>}

      {rows == null ? (
        <div className="muted">Consultando competencia en Mercado Libre…</div>
      ) : rows.length === 0 ? (
        <div className="empty">No tienes publicaciones de catálogo.</div>
      ) : (
        <div className="stack" style={{ gap: 10 }}>
          <div className="sm muted">{rows.length} publicación(es) de catálogo · <b style={{ color: 'var(--ok)' }}>{ganando}</b> ganando la Buy Box</div>
          {rows.map((r) => {
            const [txt, color] = estadoInfo(r.estado)
            const puedeIgualar = r.precio_para_ganar && r.estado !== 'winning'
            return (
              <div key={`${r.cuenta_id}-${r.item_id}`} className="card" style={{ display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
                {r.thumbnail ? (
                  <img src={r.thumbnail} alt="" width="52" height="52" style={{ borderRadius: 8, objectFit: 'cover', flexShrink: 0, background: 'var(--bg)' }} />
                ) : (
                  <div style={{ width: 52, height: 52, borderRadius: 8, flexShrink: 0, background: 'var(--bg)', display: 'grid', placeItems: 'center' }} className="muted xs">s/f</div>
                )}
                <div style={{ flex: '1 1 200px', minWidth: 0 }}>
                  <b style={{ display: 'block', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.producto || r.item_id}</b>
                  <div className="muted xs">{r.item_id} · {r.cuenta}</div>
                  <span className="badge" style={{ color, marginTop: 4, display: 'inline-block' }}>{txt}</span>
                </div>
                <div style={{ display: 'flex', gap: 14, flexWrap: 'wrap', rowGap: 6 }}>
                  <Precio label="Mi precio" v={r.mi_precio} />
                  <Precio label="Ganador" v={r.precio_ganador} />
                  <Precio label="Para ganar" v={r.precio_para_ganar} warn={puedeIgualar} />
                </div>
                {puedeIgualar && (
                  <button className="btn btn-sm btn-accent" style={{ flexShrink: 0 }} disabled={busy === r.item_id} onClick={() => igualar(r)}>
                    {busy === r.item_id ? '…' : 'Igualar'}
                  </button>
                )}
              </div>
            )
          })}
          <p className="xs muted" style={{ margin: 0 }}>"Igualar" cambia el precio de tu publicación al precio para ganar — no crea publicaciones nuevas.</p>
        </div>
      )}
    </div>
  )
}
