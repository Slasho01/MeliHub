import { Fragment, useEffect, useState } from 'react'
import { get, money, post, put, csvLocal } from '../api'

const CAMPO_LABEL = { precio: 'Precio', stock: 'Stock', estado: 'Estado' }
const FUENTE_LABEL = { sync: 'sincronización', venta: 'venta', manual: 'manual' }

function GanCell({ g }) {
  if (!g) return <span className="muted">—</span>
  const tone = g.ganancia > 0 ? 'ok' : 'bad'
  return (
    <div>
      <b style={{ color: `var(--${tone})` }}>{money(g.ganancia)}</b>
      <div className="muted sm">{g.margen}% · comis. {money(g.comision)}</div>
    </div>
  )
}

export default function Inventario({ me }) {
  const soloLectura = me?.rol_equipo === 'lectura'
  const [items, setItems] = useState(null)
  const [cuentas, setCuentas] = useState([])
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [q, setQ] = useState('')
  const [cuenta, setCuenta] = useState('')
  const [estado, setEstado] = useState('')
  const [edit, setEdit] = useState({})        // { [id]: {costo, precio_jarstore} }
  const [precioMl, setPrecioMl] = useState({}) // { [id]: valor editado del precio ML }
  const [sel, setSel] = useState(() => new Set())
  const [bulk, setBulk] = useState({ costo: '', precio_jarstore: '' })
  const [alertas, setAlertas] = useState([])
  const [verAlertas, setVerAlertas] = useState(false)
  const [ocultarSinStock, setOcultarSinStock] = useState(true)
  const [hist, setHist] = useState({})   // { [id]: [...] }
  const [histId, setHistId] = useState(null)

  const verHistorial = async (p) => {
    if (histId === p.id) { setHistId(null); return }
    try {
      if (!hist[p.id]) { const h = await get(`/productos/${p.id}/historial`); setHist((prev) => ({ ...prev, [p.id]: h })) }
      setHistId(p.id)
    } catch (e) { setError(e.message) }
  }

  const cargar = () => {
    const params = new URLSearchParams()
    if (q.trim()) params.set('q', q.trim())
    if (cuenta) params.set('cuenta_id', cuenta)
    if (estado) params.set('estado', estado)
    const s = params.toString()
    get(`/productos${s ? `?${s}` : ''}`).then((d) => { setItems(d); setEdit({}); setPrecioMl({}); setSel(new Set()) }).catch((e) => setError(e.message))
  }
  useEffect(() => {
    get('/ml/cuentas').then(setCuentas).catch(() => {})
    get('/productos/alertas?umbral=5').then(setAlertas).catch(() => {})
  }, [])
  useEffect(() => { cargar() }, [q, cuenta, estado]) // eslint-disable-line react-hooks/exhaustive-deps

  const val = (p, campo) => edit[p.id]?.[campo] ?? (p[campo] ?? '')
  const onChange = (p, campo, v) => setEdit({ ...edit, [p.id]: { ...edit[p.id], [campo]: v } })

  const guardar = async (p) => {
    if (!edit[p.id]) return
    // Mandamos los 3 valores actuales (con fallback al guardado) para no borrar los otros.
    const num = (c) => { const v = val(p, c); return v === '' || v == null ? null : Number(v) }
    const body = { costo: num('costo'), precio_jarstore: num('precio_jarstore'), precio_falabella: num('precio_falabella') }
    try { await put(`/productos/${p.id}/inventario`, body); cargar() } catch (err) { setError(err.message) }
  }

  // --- Acciones de publicación (ML) ---
  const toggleEstado = async (p) => {
    const estado = p.estado === 'active' ? 'paused' : 'active'
    const txt = estado === 'paused' ? 'pausar' : 'reactivar'
    if (!window.confirm(`¿${txt[0].toUpperCase() + txt.slice(1)} esta publicación en Mercado Libre?`)) return
    setError(null); setMsg(null)
    try { await post(`/productos/${p.id}/ml/estado`, { estado }); setMsg(`Publicación ${estado === 'paused' ? 'pausada' : 'reactivada'} en ML.`); cargar() }
    catch (err) { setError(err.message) }
  }

  const usarSugerido = async (p) => {
    setError(null); setMsg(null)
    try { await put(`/productos/${p.id}/inventario`, { precio_jarstore: p.precio_sugerido }); cargar() }
    catch (err) { setError(err.message) }
  }

  const pushPrecio = async (p) => {
    const v = precioMl[p.id]
    if (v === undefined || v === '' || Number(v) === p.precio) return
    if (!window.confirm(`¿Cambiar el precio en Mercado Libre a ${money(Number(v))}?`)) return
    setError(null); setMsg(null)
    try { await put(`/productos/${p.id}/ml/precio`, { precio: Number(v) }); setMsg('Precio actualizado en Mercado Libre.'); cargar() }
    catch (err) { setError(err.message) }
  }

  // --- Selección y edición masiva ---
  const toggleSel = (id) => setSel((s) => { const n = new Set(s); n.has(id) ? n.delete(id) : n.add(id); return n })
  // Oculta (opcional) las publicaciones sin stock en Mercado Libre. Al filtrar "Pausadas"
  // se muestran igual aunque no tengan stock (suelen estar en 0, es justo lo que se busca ver).
  const visibles = (items || []).filter((p) => !ocultarSinStock || estado === 'paused' || (p.stock || 0) > 0)
  const allIds = visibles.map((p) => p.id)
  const allSel = allIds.length > 0 && allIds.every((id) => sel.has(id))
  const toggleAll = () => setSel((s) => { const n = new Set(s); allSel ? allIds.forEach((id) => n.delete(id)) : allIds.forEach((id) => n.add(id)); return n })

  const exportarCsv = () => {
    const rows = [['ML ID', 'Título', 'Estado', 'Stock', 'Precio ML', 'Costo', 'Mi precio', 'Ganancia ML', 'Ganancia propio', 'Margen ML %']]
    for (const p of items || []) {
      rows.push([p.ml_item_id, p.titulo, p.estado, p.stock, p.precio, p.costo ?? '', p.precio_jarstore ?? '', p.ml?.ganancia ?? '', p.jarstore?.ganancia ?? '', p.ml?.margen ?? ''])
    }
    csvLocal('inventario-melihub.csv', rows)
  }

  const aplicarMasivo = async () => {
    const ids = [...sel]
    if (!ids.length) return
    const body = { ids }
    if (bulk.costo !== '') body.costo = Number(bulk.costo)
    if (bulk.precio_jarstore !== '') body.precio_jarstore = Number(bulk.precio_jarstore)
    if (!('costo' in body) && !('precio_jarstore' in body)) { setError('Indica costo y/o tu precio a aplicar'); return }
    setError(null); setMsg(null)
    try { const r = await post('/productos/inventario-masivo', body); setMsg(`${r.afectados} producto(s) actualizados.`); setBulk({ costo: '', precio_jarstore: '' }); cargar() }
    catch (err) { setError(err.message) }
  }

  return (
    <div className="stack">
      <div className="row between">
        <div><h1>Inventario</h1><p className="muted sm" style={{ margin: 0 }}>Costo/precio propio, y control de la publicación en Mercado Libre.</p></div>
        <div className="row">
          <input className="input" style={{ width: 200 }} placeholder="Buscar por título o SKU…" value={q} onChange={(e) => setQ(e.target.value)} />
          <select className="input" value={estado} onChange={(e) => setEstado(e.target.value)} title="Filtra por estado de la publicación">
            <option value="">Todos los estados</option>
            <option value="active">Activas</option>
            <option value="paused">Pausadas</option>
          </select>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)}>
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <label className="row sm muted" style={{ gap: 6, whiteSpace: 'nowrap' }} title="Oculta las publicaciones sin stock en Mercado Libre">
            <input type="checkbox" checked={ocultarSinStock} onChange={(e) => setOcultarSinStock(e.target.checked)} /> Ocultar sin stock
          </label>
          <button className="btn btn-sm" disabled={!items?.length} onClick={exportarCsv}>Exportar CSV</button>
        </div>
      </div>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      {soloLectura && <div className="alert">👁️ Modo solo lectura: puedes consultar, pero no modificar.</div>}

      {alertas.length > 0 && (
        <div className="card" style={{ borderColor: 'var(--warn)' }}>
          <div className="row between" style={{ cursor: 'pointer' }} onClick={() => setVerAlertas((v) => !v)}>
            <b style={{ color: 'var(--warn)' }}>⚠️ {alertas.length} producto(s) con stock bajo (≤ 5)</b>
            <span className="link-btn">{verAlertas ? 'ocultar' : 'ver'}</span>
          </div>
          {verAlertas && (
            <ul className="muted sm" style={{ margin: '8px 0 0', paddingLeft: 18, lineHeight: 1.7 }}>
              {alertas.map((p) => (
                <li key={p.id}><b style={{ color: p.stock <= 0 ? 'var(--bad)' : 'var(--warn)' }}>{p.stock} u.</b> — {p.titulo} <span className="muted">({p.ml_item_id})</span></li>
              ))}
            </ul>
          )}
        </div>
      )}

      {sel.size > 0 && (
        <div className="card row" style={{ alignItems: 'flex-end', gap: 12, flexWrap: 'wrap', borderColor: 'var(--accent)' }}>
          <b>{sel.size} seleccionado(s)</b>
          <label className="stack" style={{ gap: 2 }}><span className="sm muted">Costo</span>
            <input className="input" style={{ width: 110 }} type="number" min="0" value={bulk.costo} onChange={(e) => setBulk({ ...bulk, costo: e.target.value })} /></label>
          <label className="stack" style={{ gap: 2 }}><span className="sm muted">Mi precio</span>
            <input className="input" style={{ width: 120 }} type="number" min="0" value={bulk.precio_jarstore} onChange={(e) => setBulk({ ...bulk, precio_jarstore: e.target.value })} /></label>
          <button className="btn btn-accent" onClick={aplicarMasivo}>Aplicar a seleccionados</button>
          <button className="btn btn-sm" onClick={() => setSel(new Set())}>Limpiar</button>
        </div>
      )}

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!items ? <div className="empty">Cargando…</div> : items.length === 0 ? (
          <div className="empty">Sin publicaciones. Conecta una cuenta y sincroniza.</div>
        ) : visibles.length === 0 ? (
          <div className="empty">Todas las publicaciones están sin stock. <button className="link-btn" onClick={() => setOcultarSinStock(false)}>Mostrarlas</button></div>
        ) : (
          <table style={{ minWidth: 1000 }}>
            <thead><tr>
              <th><input type="checkbox" checked={allSel} onChange={toggleAll} disabled={soloLectura} /></th>
              <th>Publicación</th><th>Precio ML</th><th>Costo</th><th>Mi precio</th>
              <th>Ganancia ML</th><th>Ganancia (mi precio)</th><th>Stock</th><th>Estado</th>
            </tr></thead>
            <tbody>
              {visibles.map((p) => {
                const activo = p.estado === 'active'
                const mlVal = precioMl[p.id] ?? p.precio
                const cambiado = precioMl[p.id] !== undefined && Number(precioMl[p.id]) !== p.precio
                return (
                  <Fragment key={p.id}>
                  <tr style={sel.has(p.id) ? { background: 'color-mix(in srgb, var(--accent) 10%, transparent)' } : undefined}>
                    <td><input type="checkbox" checked={sel.has(p.id)} onChange={() => toggleSel(p.id)} disabled={soloLectura} /></td>
                    <td style={{ maxWidth: 220 }}>
                      <div className="row" style={{ gap: 8, alignItems: 'center' }}>
                        {p.thumbnail && <img src={p.thumbnail} alt="" width="36" height="36" style={{ borderRadius: 6, objectFit: 'cover' }} />}
                        <div style={{ minWidth: 0 }}><b>{p.titulo}</b><div className="muted sm">{p.ml_item_id}</div></div>
                      </div>
                    </td>
                    <td>
                      <div className="row" style={{ gap: 4 }}>
                        <input className="input" style={{ width: 92 }} type="number" min="0" value={mlVal} disabled={soloLectura}
                          onChange={(e) => setPrecioMl({ ...precioMl, [p.id]: e.target.value })} />
                        {cambiado && !soloLectura && <button className="btn btn-sm btn-accent" title="Empujar a Mercado Libre" onClick={() => pushPrecio(p)}>↑ML</button>}
                      </div>
                    </td>
                    <td><input className="input" style={{ width: 92 }} type="number" min="0" placeholder="—" disabled={soloLectura}
                      value={val(p, 'costo')} onChange={(e) => onChange(p, 'costo', e.target.value)} onBlur={() => guardar(p)} /></td>
                    <td>
                      <input className="input" style={{ width: 100 }} type="number" min="0" placeholder="—" disabled={soloLectura}
                        value={val(p, 'precio_jarstore')} onChange={(e) => onChange(p, 'precio_jarstore', e.target.value)} onBlur={() => guardar(p)} />
                      {p.precio_sugerido != null && (
                        <div className="muted sm" style={{ marginTop: 2, whiteSpace: 'nowrap' }}>
                          sug. {money(p.precio_sugerido)} {!soloLectura && <button className="link-btn" onClick={() => usarSugerido(p)}>usar</button>}
                        </div>
                      )}
                      <div className="row" style={{ gap: 4, marginTop: 4, alignItems: 'center' }}>
                        <span className="muted xs" style={{ whiteSpace: 'nowrap' }}>Falabella</span>
                        <input className="input" style={{ width: 86 }} type="number" min="0" placeholder="= ML" disabled={soloLectura}
                          title="Precio para Falabella (vacío = usa el de Mercado Libre)"
                          value={val(p, 'precio_falabella')} onChange={(e) => onChange(p, 'precio_falabella', e.target.value)} onBlur={() => guardar(p)} />
                      </div>
                    </td>
                    <td><GanCell g={p.ml} /></td>
                    <td><GanCell g={p.jarstore} /></td>
                    <td style={{ color: p.stock <= 0 ? 'var(--bad)' : undefined }}>{p.stock}</td>
                    <td>
                      <div className="stack" style={{ gap: 4 }}>
                        <span className="badge" style={{ color: activo ? 'var(--ok)' : 'var(--muted)' }}>{activo ? 'Activa' : p.estado === 'paused' ? 'Pausada' : p.estado}</span>
                        <div className="row" style={{ gap: 4 }}>
                          {!soloLectura && <button className="btn btn-sm" onClick={() => toggleEstado(p)}>{activo ? 'Pausar' : 'Reactivar'}</button>}
                          <button className="btn btn-sm" title="Historial" onClick={() => verHistorial(p)}>📈</button>
                        </div>
                      </div>
                    </td>
                  </tr>
                  {histId === p.id && (
                    <tr>
                      <td colSpan={9} style={{ background: 'var(--bg)' }}>
                        <b className="sm">Historial de cambios</b>
                        {(hist[p.id] || []).length === 0 ? (
                          <div className="muted sm">Sin cambios registrados todavía.</div>
                        ) : (
                          <ul className="muted sm" style={{ margin: '6px 0 0', paddingLeft: 18, lineHeight: 1.7 }}>
                            {(hist[p.id] || []).map((h) => (
                              <li key={h.id}>
                                <span className="muted">{new Date(h.created_at + 'Z').toLocaleString('es-CL')}</span> ·{' '}
                                <b>{CAMPO_LABEL[h.campo] || h.campo}</b>: {h.campo === 'precio' ? money(Number(h.anterior)) : (h.anterior ?? '—')} → {h.campo === 'precio' ? money(Number(h.nuevo)) : h.nuevo}{' '}
                                <span className="badge">{FUENTE_LABEL[h.fuente] || h.fuente}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </td>
                    </tr>
                  )}
                  </Fragment>
                )
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
