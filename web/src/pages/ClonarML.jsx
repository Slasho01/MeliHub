import { useEffect, useMemo, useState } from 'react'
import { get, post, money } from '../api'

export default function ClonarML() {
  const [cuentas, setCuentas] = useState([])
  const [origen, setOrigen] = useState('')
  const [destino, setDestino] = useState('')
  const [productos, setProductos] = useState(null)
  const [sel, setSel] = useState(new Set())
  const [q, setQ] = useState('')
  const [clonando, setClonando] = useState(false)
  const [error, setError] = useState('')
  const [rep, setRep] = useState(null)
  const [agrupando, setAgrupando] = useState(false)
  const [agrMsg, setAgrMsg] = useState(null)
  // Clonar desde un link.
  const [linkInput, setLinkInput] = useState('')
  const [preview, setPreview] = useState(null)
  const [prevBusy, setPrevBusy] = useState(false)
  const [linkDestino, setLinkDestino] = useState('')
  const [linkTitulo, setLinkTitulo] = useState('')
  const [linkPrecio, setLinkPrecio] = useState('')
  const [linkBusy, setLinkBusy] = useState(false)
  const [linkRep, setLinkRep] = useState(null)

  const previsualizar = async () => {
    if (!linkInput.trim()) return
    setPrevBusy(true); setError(''); setPreview(null); setLinkRep(null)
    try {
      const d = await post('/ml/link/previsualizar', { link: linkInput.trim() })
      setPreview(d); setLinkTitulo(d.titulo || ''); setLinkPrecio(d.precio ? String(Math.round(d.precio)) : '')
    } catch (e) { setError(e.message) } finally { setPrevBusy(false) }
  }
  const clonarLink = async () => {
    if (!preview) return
    if (!linkDestino) { setError('Elige la cuenta donde crear la publicación.'); return }
    if (!linkTitulo.trim()) { setError('Ponle un título.'); return }
    setLinkBusy(true); setError(''); setLinkRep(null)
    try {
      const r = await post('/ml/link/clonar', {
        link: linkInput.trim(),
        destino_cuenta_id: linkDestino,
        titulo: linkTitulo.trim(),
        precio: Number(linkPrecio) || 0,
      })
      setLinkRep(r)
    } catch (e) { setError(e.message) } finally { setLinkBusy(false) }
  }

  const agrupar = async () => {
    setAgrupando(true); setError(''); setAgrMsg(null)
    try {
      const r = await post('/productos/agrupar-clones')
      setAgrMsg(r.grupos > 0
        ? `✅ ${r.grupos} grupo(s) · ${r.agrupados} producto(s) enlazados por título. Ya comparten stock entre cuentas.`
        : 'No se encontraron clones con el mismo título en distintas cuentas para agrupar.')
    } catch (e) { setError(e.message) } finally { setAgrupando(false) }
  }

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch((e) => setError(e.message)) }, [])

  useEffect(() => {
    setProductos(null); setSel(new Set()); setRep(null)
    if (!origen) return
    get(`/productos?cuenta_id=${origen}`).then((d) => setProductos(d || [])).catch((e) => setError(e.message))
  }, [origen])

  const visibles = useMemo(() => {
    const t = q.trim().toLowerCase()
    return (productos || []).filter((p) => !t || (p.titulo || '').toLowerCase().includes(t) || (p.ml_item_id || '').toLowerCase().includes(t))
  }, [productos, q])

  const nombre = (id) => { const c = cuentas.find((x) => String(x.id) === String(id)); return c ? (c.nickname || `Usuario ${c.ml_user_id}`) : '' }
  const toggle = (id) => setSel((s) => { const n = new Set(s); n.has(id) ? n.delete(id) : n.add(id); return n })
  const allVis = visibles.length > 0 && visibles.every((p) => sel.has(p.id))
  const toggleAll = () => setSel((s) => { const n = new Set(s); allVis ? visibles.forEach((p) => n.delete(p.id)) : visibles.forEach((p) => n.add(p.id)); return n })

  const clonar = async () => {
    const ids = [...sel]
    if (!origen || !destino) { setError('Elige cuenta de origen y destino.'); return }
    if (origen === destino) { setError('La cuenta destino debe ser distinta.'); return }
    if (!ids.length) { setError('Selecciona al menos un producto.'); return }
    if (!window.confirm(`¿Clonar ${ids.length} publicación(es) de "${nombre(origen)}" a "${nombre(destino)}"? Se crearán como publicaciones nuevas en Mercado Libre.`)) return
    setError(''); setRep(null); setClonando(true)
    try {
      const r = await post('/ml/clonar', { origen_cuenta_id: origen, destino_cuenta_id: destino, producto_ids: ids })
      setRep(r)
    } catch (e) { setError(e.message) } finally { setClonando(false) }
  }

  const destinos = cuentas.filter((c) => String(c.id) !== String(origen))

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-start' }}>
        <div>
          <h1 style={{ margin: 0 }}>Clonar publicaciones</h1>
          <p className="muted sm" style={{ margin: 0 }}>Copia publicaciones de una cuenta de Mercado Libre a otra. Se crean como publicaciones nuevas en la cuenta destino.</p>
        </div>
        <button className="btn btn-sm" disabled={agrupando} onClick={agrupar} title="Enlaza los clones que YA hiciste: agrupa productos con el mismo título en distintas cuentas para que compartan stock">
          {agrupando ? 'Agrupando…' : '🔗 Agrupar clones existentes'}
        </button>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {agrMsg && <div className="alert alert-ok">{agrMsg}</div>}

      <div className="card stack" style={{ gap: 10 }}>
        <div>
          <b>🔗 Clonar desde un link</b>
          <p className="muted xs" style={{ margin: 0 }}>Pega el link de cualquier publicación de Mercado Libre y créala en tu cuenta. Puedes editar título y precio antes de publicar.</p>
        </div>
        <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
          <input className="input" style={{ flex: '1 1 320px' }} placeholder="https://articulo.mercadolibre.cl/MLC-..." value={linkInput} onChange={(e) => setLinkInput(e.target.value)} />
          <button className="btn" disabled={prevBusy || !linkInput.trim()} onClick={previsualizar}>{prevBusy ? 'Leyendo…' : 'Ver'}</button>
        </div>

        {preview && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="row" style={{ gap: 10, alignItems: 'center', flexWrap: 'wrap' }}>
              {preview.thumbnail && <img src={preview.thumbnail} alt="" width="54" height="54" style={{ borderRadius: 8, objectFit: 'cover', flexShrink: 0 }} />}
              <span className="muted xs">Origen: {preview.item_id} · {preview.categoria || 'sin categoría'}{preview.permalink && <> · <a href={preview.permalink} target="_blank" rel="noreferrer">ver original ↗</a></>}</span>
            </div>
            <div className="row" style={{ gap: 10, flexWrap: 'wrap', alignItems: 'flex-end' }}>
              <label className="stack" style={{ gap: 4, flex: '1 1 260px' }}><span className="sm">Título (editable)</span>
                <input className="input" value={linkTitulo} onChange={(e) => setLinkTitulo(e.target.value)} />
              </label>
              <label className="stack" style={{ gap: 4, flex: '0 1 140px' }}><span className="sm">Precio</span>
                <input className="input" type="number" min="1" value={linkPrecio} onChange={(e) => setLinkPrecio(e.target.value)} />
              </label>
              <label className="stack" style={{ gap: 4, flex: '0 1 220px' }}><span className="sm">Crear en la cuenta</span>
                <select className="input" value={linkDestino} onChange={(e) => setLinkDestino(e.target.value)}>
                  <option value="">— Elige —</option>
                  {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
                </select>
              </label>
              <button className="btn btn-accent" disabled={linkBusy} onClick={clonarLink}>{linkBusy ? 'Creando…' : 'Crear en mi cuenta'}</button>
            </div>
            <span className="muted xs">⚠️ Copiar la publicación de otro vendedor (fotos/marca) puede chocar con las reglas de Mercado Libre. Ideal para tus propios productos o editando fotos/título.</span>
          </div>
        )}

        {linkRep && (
          <div className="alert alert-ok" style={{ display: 'block' }}>
            ✅ Publicación creada {linkRep.permalink ? <>· <a href={linkRep.permalink} target="_blank" rel="noreferrer">ver en ML ↗</a></> : `(${linkRep.item_id})`}
          </div>
        )}
      </div>

      <div className="card row" style={{ gap: 12, flexWrap: 'wrap', alignItems: 'flex-end' }}>
        <label className="stack" style={{ gap: 4, flex: '1 1 220px' }}><span className="sm">Cuenta de origen</span>
          <select className="input" value={origen} onChange={(e) => setOrigen(e.target.value)}>
            <option value="">— Elige —</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
        </label>
        <div style={{ fontSize: 22, color: 'var(--muted)', paddingBottom: 6 }}>→</div>
        <label className="stack" style={{ gap: 4, flex: '1 1 220px' }}><span className="sm">Cuenta de destino</span>
          <select className="input" value={destino} onChange={(e) => setDestino(e.target.value)} disabled={!origen}>
            <option value="">— Elige —</option>
            {destinos.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
        </label>
      </div>

      {origen && (
        <div className="card stack" style={{ gap: 10 }}>
          <div className="row between" style={{ flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>
            <b>{productos == null ? 'Cargando…' : `${visibles.length} publicación(es)`} {sel.size > 0 && <span className="muted">· {sel.size} seleccionada(s)</span>}</b>
            <input className="input" style={{ width: 220 }} placeholder="Buscar…" value={q} onChange={(e) => setQ(e.target.value)} />
          </div>

          {productos != null && visibles.length > 0 && (
            <>
              <label className="row sm" style={{ gap: 6, alignItems: 'center' }}>
                <input type="checkbox" checked={allVis} onChange={toggleAll} /> Seleccionar todo
              </label>
              <div style={{ maxHeight: 420, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
                {visibles.map((p) => (
                  <label key={p.id} className="row" style={{ gap: 10, alignItems: 'center', padding: '6px 8px', borderRadius: 8, background: sel.has(p.id) ? 'var(--surface-2)' : 'transparent' }}>
                    <input type="checkbox" checked={sel.has(p.id)} onChange={() => toggle(p.id)} />
                    {p.thumbnail && <img src={p.thumbnail} alt="" width="34" height="34" style={{ borderRadius: 6, objectFit: 'cover', flexShrink: 0 }} />}
                    <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.titulo}</span>
                    <span className="muted sm" style={{ whiteSpace: 'nowrap' }}>{money(p.precio)} · {p.stock} u.</span>
                  </label>
                ))}
              </div>
            </>
          )}
          {productos != null && visibles.length === 0 && <div className="empty">Esta cuenta no tiene publicaciones sincronizadas{q ? ' que coincidan' : ''}.</div>}

          <div className="row between" style={{ flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>
            <span className="muted xs">Las publicaciones de catálogo (Buy Box) o con atributos faltantes pueden fallar y se marcan para revisar.</span>
            <button className="btn btn-accent" disabled={clonando || !destino || sel.size === 0} onClick={clonar}>
              {clonando ? 'Clonando… (puede tardar)' : `Clonar ${sel.size || ''} → ${nombre(destino) || 'destino'}`}
            </button>
          </div>
        </div>
      )}

      {rep && (
        <div className="card stack" style={{ gap: 8 }}>
          <b>Resultado: {rep.creados} de {rep.total} creada(s){rep.total - rep.creados > 0 ? ` · ${rep.total - rep.creados} con error` : ''}</b>
          <div style={{ maxHeight: 360, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
            {(rep.resultados || []).map((r, i) => (
              <div key={i} className="row" style={{ gap: 8, alignItems: 'baseline', flexWrap: 'wrap' }}>
                <span style={{ color: r.ok ? 'var(--ok)' : 'var(--bad, #d93025)', fontWeight: 800 }}>{r.ok ? '✓' : '✗'}</span>
                <span className="sm" style={{ flex: '1 1 200px', minWidth: 0 }}>{r.producto}</span>
                {r.ok
                  ? (r.permalink ? <a className="sm" href={r.permalink} target="_blank" rel="noreferrer">ver en ML ↗</a> : <span className="muted sm">creada</span>)
                  : <span className="muted xs" style={{ flex: '2 1 240px' }}>{r.error}</span>}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
