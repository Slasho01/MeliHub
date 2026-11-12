import { useEffect, useState } from 'react'
import { get, post } from '../api'

const asList = (v) => v?.results || v?.data || (Array.isArray(v) ? v : [])

const badge = (st) => {
  const s = String(st || '').toLowerCase()
  const map = {
    approved: ['Aprobado', 'tag-ok'], aprobado: ['Aprobado', 'tag-ok'],
    published: ['Publicado', 'tag-ok'], active: ['Activo', 'tag-ok'], enabled: ['Activo', 'tag-ok'],
    pending: ['En revisión', 'tag'], pendiente: ['En revisión', 'tag'], in_review: ['En revisión', 'tag'],
    rejected: ['Rechazado', 'tag'], rechazado: ['Rechazado', 'tag'], disabled: ['Pausado', 'tag'], deleted: ['Eliminado', 'tag'],
  }
  const [t, c] = map[s] || [st || '—', 'tag']
  const style = (s.includes('rechaz') || s.includes('reject')) ? { background: 'var(--bad, #d93025)', color: '#fff' } : undefined
  return <span className={`tag ${c}`} style={style}>{t}</span>
}

const LIMIT = 50

export default function ParisEstado({ cuenta }) {
  const [prods, setProds] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState('')
  const [off, setOff] = useState(0)
  const [more, setMore] = useState(false)

  const cargar = (reset = true) => {
    const start = reset ? 0 : off
    if (reset) { setProds(null); setError(null) }
    get(`/paris/cuentas/${cuenta.id}/productos?query=${encodeURIComponent(`limit=${LIMIT}&offset=${start}`)}`)
      .then((d) => { const l = asList(d); setProds((p) => (reset ? l : [...(p || []), ...l])); setOff(start + l.length); setMore(l.length === LIMIT) })
      .catch((e) => setError(e.message))
  }
  useEffect(() => { cargar(true) }, [cuenta.id]) // eslint-disable-line react-hooks/exhaustive-deps

  const cambiar = async (p, status) => {
    const sku = p.id || p.sellerSku
    const lbl = { DISABLED: 'pausar', DELETED: 'eliminar', ENABLED: 'activar' }[status] || status
    if (!window.confirm(`¿${lbl} "${p.name}" en Paris?`)) return
    setBusy(sku); setError(null); setMsg(null)
    try { await post(`/paris/cuentas/${cuenta.id}/producto-estado`, { sku, status }); setMsg(`Producto ${lbl === 'activar' ? 'activado' : lbl === 'pausar' ? 'pausado' : 'eliminado'}.`); setTimeout(() => cargar(true), 1200) }
    catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (error && !prods) return <div className="alert alert-bad">{error}</div>
  if (prods == null) return <div className="muted">Cargando…</div>

  return (
    <div className="stack" style={{ gap: 10 }}>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      <div className="row between">
        <span className="sm muted">{prods.length} producto(s) en Paris</span>
        <button className="btn btn-sm" onClick={() => cargar(true)}>Actualizar</button>
      </div>
      {prods.length === 0 ? (
        <div className="muted sm">Aún no hay productos publicados en Paris.</div>
      ) : (
        <div className="card table-wrap" style={{ padding: 0 }}>
          <table className="table">
            <thead><tr><th>Producto</th><th>SKU</th><th>Estado</th><th /></tr></thead>
            <tbody>
              {prods.map((p, i) => {
                const img = p.variants?.[0]?.medias?.[0]?.src
                const sku = p.id || p.sellerSku
                return (
                  <tr key={sku || i}>
                    <td>
                      <span className="row nowrap" style={{ '--gap': '8px' }}>
                        {img && <img src={img} alt="" style={{ width: 34, height: 34, borderRadius: 6, objectFit: 'cover' }} />}
                        <strong>{p.name}</strong>
                      </span>
                    </td>
                    <td className="sm muted">{sku || '—'}</td>
                    <td>{badge(p.statusApproval || p.status)}</td>
                    <td className="r" style={{ whiteSpace: 'nowrap' }}>
                      <button className="btn btn-sm" disabled={busy === sku} onClick={() => cambiar(p, 'ENABLED')}>Activar</button>{' '}
                      <button className="btn btn-sm" disabled={busy === sku} onClick={() => cambiar(p, 'DISABLED')}>Pausar</button>{' '}
                      <button className="btn btn-sm btn-danger" disabled={busy === sku} onClick={() => cambiar(p, 'DELETED')}>Eliminar</button>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
          <p className="xs muted" style={{ padding: '10px 14px', margin: 0 }}>Paris no permite borrar del todo: "Eliminar" deshabilita la publicación (status DELETED); puedes "Activar" de nuevo.</p>
        </div>
      )}
      {more && <button className="btn btn-sm" style={{ alignSelf: 'center' }} onClick={() => cargar(false)}>Cargar más</button>}
    </div>
  )
}
