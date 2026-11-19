import { useEffect, useState } from 'react'
import { get, post } from '../api'

const qcBadge = (qc) => {
  const map = { approved: ['Aprobado', 'tag-ok'], pending: ['En revisión', 'tag'], rejected: ['Rechazado', 'tag'] }
  const [t, c] = map[qc] || [qc || '—', 'tag']
  const style = qc === 'rejected' ? { background: 'var(--bad, #d93025)', color: '#fff' } : undefined
  return <span className={`tag ${c}`} style={style}>{t}</span>
}

export default function FalabellaEstado({ cuenta }) {
  const [prods, setProds] = useState(null) // productos REALES de Falabella (de su API)
  const [mapa, setMapa] = useState({}) // seller_sku -> { producto_id, titulo } (mapeo local, para acciones)
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState('')
  const [msg, setMsg] = useState(null)

  const vincular = async () => {
    if (!cuenta) return
    setBusy('vincular'); setError(null); setMsg(null)
    try {
      const r = await post(`/falabella/vincular?cuenta=${cuenta.id}`)
      setMsg(`${r.vinculados} publicación(es) re-vinculada(s)${r.no_encontrados ? ` · ${r.no_encontrados} sin producto en el hub` : ''}. El stock ya vuelve a sincronizarse.`)
      cargar()
    } catch (e) { setError(e.message) } finally { setBusy('') }
  }

  const cargar = () => {
    if (!cuenta) return
    setError(null)
    // Mapeo local: para vincular acciones (republicar/pausar) y el título del hub.
    get(`/falabella/publicaciones?cuenta=${cuenta.id}`)
      .then((pubs) => {
        const m = {}
        ;(pubs || []).forEach((p) => { m[p.seller_sku] = { producto_id: p.producto_id, titulo: p.titulo } })
        setMapa(m)
      })
      .catch(() => {})
    // Catálogo real en Falabella (su API GetProducts).
    get(`/falabella/productos?cuenta=${cuenta.id}`)
      .then((d) => {
        let pr = d?.SuccessResponse?.Body?.Products?.Product
        pr = Array.isArray(pr) ? pr : pr ? [pr] : []
        setProds(
          pr.map((p) => {
            const bu = Array.isArray(p.BusinessUnits?.BusinessUnit) ? p.BusinessUnits.BusinessUnit[0] : p.BusinessUnits?.BusinessUnit
            return { sku: p.SellerSku, nombre: p.Name, qc: p.QCStatus, publicado: bu?.IsPublished === '1', stock: bu?.Stock, img: !!p.MainImage }
          })
        )
      })
      .catch((e) => { setError('No se pudieron traer tus productos de Falabella: ' + (e.message || 'error')); setProds([]) })
  }
  useEffect(() => { cargar() }, [cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const cambiar = async (producto_id, activo) => {
    setBusy(producto_id); setError(null)
    try { await post('/falabella/estado', { producto_id, cuenta_id: cuenta.id, activo }); setTimeout(cargar, 1800) }
    catch (e) { setError(e.message) } finally { setBusy('') }
  }
  const republicar = async (producto_id) => {
    setBusy(producto_id); setError(null)
    try { await post('/falabella/republicar', { producto_id, cuenta_id: cuenta.id }); setTimeout(cargar, 2500) }
    catch (e) { setError(e.message) } finally { setBusy('') }
  }

  if (!cuenta) return null

  return (
    <div className="card stack" style={{ gap: 10 }}>
      <div className="row between" style={{ flexWrap: 'wrap', gap: 8 }}><div className="strong">Mis publicaciones en Falabella</div>
        <div className="row" style={{ gap: 8 }}>
          <button className="btn btn-sm btn-accent" disabled={busy === 'vincular'} onClick={vincular} title="Re-enlaza las publicaciones de Falabella con tus productos por SKU (repara la sincronización de stock)">{busy === 'vincular' ? 'Vinculando…' : '🔗 Vincular por SKU'}</button>
          <button className="btn btn-sm" onClick={cargar}>↻ Actualizar</button>
        </div>
      </div>
      {msg && <div className="alert alert-ok">{msg}</div>}
      {error && <div className="alert alert-bad">{error}</div>}
      {prods == null ? (
        <div className="muted">Cargando…</div>
      ) : prods.length === 0 ? (
        <div className="muted sm">No se encontraron productos en tu cuenta de Falabella. Si recién publicaste, dale a «Actualizar» en unos minutos.</div>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%' }}>
            <thead><tr><th>Producto</th><th>SKU</th><th>Estado (QC)</th><th>Stock</th><th>Img</th><th>Publicado</th><th></th></tr></thead>
            <tbody>
              {prods.map((p) => {
                const link = mapa[p.sku] // vínculo con el hub (si se publicó desde Ventoki)
                const pid = link?.producto_id
                return (
                  <tr key={p.sku}>
                    <td className="strong">{link?.titulo || p.nombre || '—'}</td>
                    <td className="sm muted">{p.sku}</td>
                    <td>{qcBadge(p.qc)}</td>
                    <td className="sm">{p.stock ?? '—'}</td>
                    <td className="sm">{p.img ? '✓' : '—'}</td>
                    <td className="sm">{p.publicado ? 'Sí' : 'No'}</td>
                    <td style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>
                      {pid ? (
                        <>
                          <button className="btn btn-sm" disabled={busy === pid} onClick={() => republicar(pid)} title="Volver a publicar reusando la categoría/marca guardadas">↻ Republicar</button>{' '}
                          {p.publicado
                            ? <button className="btn btn-sm" disabled={busy === pid} onClick={() => cambiar(pid, false)}>Pausar</button>
                            : <button className="btn btn-sm btn-accent" disabled={busy === pid} onClick={() => cambiar(pid, true)}>Reactivar</button>}
                        </>
                      ) : (
                        <span className="muted xs" title="Publicado fuera de Ventoki — no se puede gestionar desde aquí">No vinculado</span>
                      )}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted xs" style={{ margin: 0 }}>«En revisión» = Falabella está validando; «Rechazado» = revisa la categoría/atributos y vuelve a publicar. «No vinculado» = lo publicaste fuera de Ventoki.</p>
    </div>
  )
}
