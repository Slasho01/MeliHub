import { useEffect, useState } from 'react'
import { get } from '../api'

const asList = (v) => v?.results || v?.data || (Array.isArray(v) ? v : [])

const badge = (st) => {
  const s = String(st || '').toLowerCase()
  const map = {
    approved: ['Aprobado', 'tag-ok'], aprobado: ['Aprobado', 'tag-ok'],
    published: ['Publicado', 'tag-ok'], active: ['Activo', 'tag-ok'], activo: ['Activo', 'tag-ok'],
    pending: ['En revisión', 'tag'], pendiente: ['En revisión', 'tag'], in_review: ['En revisión', 'tag'],
    rejected: ['Rechazado', 'tag'], rechazado: ['Rechazado', 'tag'],
  }
  const [t, c] = map[s] || [st || '—', 'tag']
  const style = (s.includes('rechaz') || s.includes('reject')) ? { background: 'var(--bad, #d93025)', color: '#fff' } : undefined
  return <span className={`tag ${c}`} style={style}>{t}</span>
}

export default function ParisEstado({ cuenta }) {
  const [prods, setProds] = useState(null)
  const [error, setError] = useState(null)

  const cargar = () => {
    setProds(null); setError(null)
    get(`/paris/cuentas/${cuenta.id}/productos`).then((d) => setProds(asList(d))).catch((e) => setError(e.message))
  }
  useEffect(cargar, [cuenta.id])

  if (error && !prods) return <div className="alert alert-bad">{error}</div>
  if (prods == null) return <div className="muted">Cargando…</div>

  return (
    <div className="stack" style={{ gap: 10 }}>
      <div className="row between">
        <span className="sm muted">{prods.length} producto(s) en Paris</span>
        <button className="btn btn-sm" onClick={cargar}>Actualizar</button>
      </div>
      {prods.length === 0 ? (
        <div className="muted sm">Aún no hay productos publicados en Paris.</div>
      ) : (
        <div className="card table-wrap" style={{ padding: 0 }}>
          <table className="table">
            <thead><tr><th>Producto</th><th>SKU</th><th>Estado</th></tr></thead>
            <tbody>
              {prods.map((p, i) => {
                const img = p.variants?.[0]?.medias?.[0]?.src
                return (
                  <tr key={p.id || i}>
                    <td>
                      <span className="row nowrap" style={{ '--gap': '8px' }}>
                        {img && <img src={img} alt="" style={{ width: 34, height: 34, borderRadius: 6, objectFit: 'cover' }} />}
                        <strong>{p.name}</strong>
                      </span>
                    </td>
                    <td className="sm muted">{p.id || p.sellerSku || '—'}</td>
                    <td>{badge(p.statusApproval || p.status)}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
