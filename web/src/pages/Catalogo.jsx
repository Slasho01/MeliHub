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
        <>
          <div className="sm muted">{rows.length} publicación(es) de catálogo · <b style={{ color: 'var(--ok)' }}>{ganando}</b> ganando la Buy Box</div>
          <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
            <table style={{ minWidth: 820 }}>
              <thead><tr>
                <th>Producto</th><th>Cuenta</th><th>Estado</th><th>Mi precio</th><th>Ganador</th><th>Para ganar</th><th></th>
              </tr></thead>
              <tbody>
                {rows.map((r) => {
                  const [txt, color] = estadoInfo(r.estado)
                  const puedeIgualar = r.precio_para_ganar && r.estado !== 'winning'
                  return (
                    <tr key={`${r.cuenta_id}-${r.item_id}`}>
                      <td style={{ maxWidth: 260 }}><b>{r.producto || r.item_id}</b><div className="muted xs">{r.item_id}</div></td>
                      <td className="sm muted">{r.cuenta || '—'}</td>
                      <td><span className="badge" style={{ color }}>{txt}</span></td>
                      <td>{r.mi_precio != null ? money(r.mi_precio) : '—'}</td>
                      <td>{r.precio_ganador != null ? money(r.precio_ganador) : '—'}</td>
                      <td style={{ color: puedeIgualar ? 'var(--bad, #d93025)' : undefined }}>{r.precio_para_ganar != null ? money(r.precio_para_ganar) : '—'}</td>
                      <td style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>
                        {puedeIgualar && (
                          <button className="btn btn-sm btn-accent" disabled={busy === r.item_id} onClick={() => igualar(r)}>
                            {busy === r.item_id ? '…' : 'Igualar'}
                          </button>
                        )}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
          <p className="xs muted" style={{ margin: 0 }}>"Igualar" cambia el precio de tu publicación al precio para ganar — no crea publicaciones nuevas.</p>
        </>
      )}
    </div>
  )
}
