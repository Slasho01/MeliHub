import { useEffect, useState } from 'react'
import { get, post } from '../api'

const qcBadge = (qc) => {
  const map = { approved: ['Aprobado', 'tag-ok'], pending: ['En revisión', 'tag'], rejected: ['Rechazado', 'tag'] }
  const [t, c] = map[qc] || [qc || '—', 'tag']
  const style = qc === 'rejected' ? { background: 'var(--bad, #d93025)', color: '#fff' } : undefined
  return <span className={`tag ${c}`} style={style}>{t}</span>
}

export default function FalabellaEstado({ cuenta }) {
  const [pubs, setPubs] = useState(null)
  const [fp, setFp] = useState({})
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState('')

  const cargar = () => {
    if (!cuenta) return
    setError(null)
    get(`/falabella/publicaciones?cuenta=${cuenta.id}`).then(setPubs).catch((e) => setError(e.message))
    get(`/falabella/productos?cuenta=${cuenta.id}`)
      .then((d) => {
        let pr = d?.SuccessResponse?.Body?.Products?.Product
        pr = Array.isArray(pr) ? pr : pr ? [pr] : []
        const m = {}
        pr.forEach((p) => {
          const bu = Array.isArray(p.BusinessUnits?.BusinessUnit) ? p.BusinessUnits.BusinessUnit[0] : p.BusinessUnits?.BusinessUnit
          m[p.SellerSku] = { qc: p.QCStatus, publicado: bu?.IsPublished === '1', stock: bu?.Stock, img: !!(p.MainImage) }
        })
        setFp(m)
      })
      .catch(() => {})
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const cambiar = async (row, activo) => {
    setBusy(row.producto_id)
    setError(null)
    try { await post('/falabella/estado', { producto_id: row.producto_id, cuenta_id: cuenta.id, activo }); setTimeout(cargar, 1800) }
    catch (e) { setError(e.message) } finally { setBusy('') }
  }
  const republicar = async (row) => {
    setBusy(row.producto_id)
    setError(null)
    try { await post('/falabella/republicar', { producto_id: row.producto_id, cuenta_id: cuenta.id }); setTimeout(cargar, 2500) }
    catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (!cuenta) return null

  return (
    <div className="card stack" style={{ gap: 10 }}>
      <div className="row between"><div className="strong">Mis publicaciones en Falabella</div><button className="btn btn-sm" onClick={cargar}>↻ Actualizar</button></div>
      {error && <div className="alert alert-bad">{error}</div>}
      {pubs == null ? (
        <div className="muted">Cargando…</div>
      ) : pubs.length === 0 ? (
        <div className="muted sm">Aún no has publicado productos en Falabella.</div>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%' }}>
            <thead><tr><th>Producto</th><th>SKU</th><th>Estado (QC)</th><th>Img</th><th>Publicado</th><th></th></tr></thead>
            <tbody>
              {pubs.map((r) => {
                const s = fp[r.seller_sku] || {}
                return (
                  <tr key={r.producto_id}>
                    <td className="strong">{r.titulo}</td>
                    <td className="sm muted">{r.seller_sku}</td>
                    <td>{qcBadge(s.qc)}</td>
                    <td className="sm">{s.img ? '✓' : '—'}</td>
                    <td className="sm">{s.publicado ? 'Sí' : 'No'}</td>
                    <td style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>
                      <button className="btn btn-sm" disabled={busy === r.producto_id} onClick={() => republicar(r)} title="Volver a publicar reusando la categoría/marca guardadas">↻ Republicar</button>{' '}
                      {s.publicado
                        ? <button className="btn btn-sm" disabled={busy === r.producto_id} onClick={() => cambiar(r, false)}>Pausar</button>
                        : <button className="btn btn-sm btn-accent" disabled={busy === r.producto_id} onClick={() => cambiar(r, true)}>Reactivar</button>}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted xs" style={{ margin: 0 }}>«En revisión» = Falabella está validando; «Rechazado» = revisa la categoría/atributos y vuelve a publicar.</p>
    </div>
  )
}
