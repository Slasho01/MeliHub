import { useEffect, useState } from 'react'
import { get, descargarArchivo } from '../api'

// Pendiente (hay etiqueta): el vendedor aún tiene que imprimir/entregar.
const IMPRIMIBLE = ['ready_to_print', 'printed']
const pendiente = (estado, sub) => {
  if (estado === 'pending' || estado === 'handling') return true
  if (estado === 'ready_to_ship') return sub ? IMPRIMIBLE.includes(sub) : true
  return false
}
const estadoBadge = (estado, sub) => {
  let t, c
  if (estado === 'shipped') [t, c] = ['Enviado', 'tag']
  else if (estado === 'delivered') [t, c] = ['Entregado', 'tag-ok']
  else if (estado === 'not_delivered') [t, c] = ['No entregado', 'tag']
  else if (estado === 'cancelled') [t, c] = ['Cancelado', 'tag']
  else if (estado === 'pending' || estado === 'handling') [t, c] = ['Preparando', 'tag']
  else if (sub === 'ready_to_print') [t, c] = ['Por imprimir', 'tag-ok']
  else if (sub === 'printed') [t, c] = ['Impreso, por entregar', 'tag-ok']
  else if (!sub) [t, c] = ['Listo para enviar', 'tag-ok']
  else [t, c] = ['En camino', 'tag']
  return <span className={`tag ${c}`}>{t}</span>
}

export default function Envios() {
  const [dia, setDia] = useState('') // vacío = últimos 15 días
  const [cuenta, setCuenta] = useState('') // '' = todas las cuentas
  const [cuentas, setCuentas] = useState([])
  const [incluir, setIncluir] = useState(false) // mostrar también los ya enviados
  const [rows, setRows] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => { get('/ml/cuentas').then(setCuentas).catch(() => {}) }, [])

  const cargar = () => {
    setRows(null); setError(null)
    const p = new URLSearchParams()
    if (dia) p.set('dia', dia)
    if (cuenta) p.set('cuenta', cuenta)
    if (incluir) p.set('incluir_entregados', '1')
    const qs = p.toString()
    return get(`/ventas/envios${qs ? `?${qs}` : ''}`).then(setRows).catch((e) => setError(e.message))
  }
  useEffect(() => { cargar() }, [dia, incluir, cuenta]) // eslint-disable-line react-hooks/exhaustive-deps

  const descargarDia = async () => {
    setBusy(true); setError(null); setMsg(null)
    const p = new URLSearchParams()
    if (dia) p.set('dia', dia)
    if (cuenta) p.set('cuenta', cuenta)
    const qs = p.toString()
    const nombre = cuenta ? (cuentas.find((c) => c.id === cuenta)?.nickname || 'cuenta') : 'todas'
    try { await descargarArchivo(`/ventas/etiquetas${qs ? `?${qs}` : ''}`, `etiquetas-${nombre}-${dia || 'pendientes'}.pdf`); setMsg('Descarga lista.') }
    catch (e) { setError(e.message) } finally { setBusy(false) }
  }
  const descargarUna = async (o) => {
    setError(null); setMsg(null)
    try { await descargarArchivo(`/ventas/${o.id}/etiqueta`, `etiqueta-${o.ml_order_id}.pdf`) }
    catch (e) { setError(e.message) }
  }

  const pendientesCount = rows?.filter((o) => pendiente(o.envio_estado, o.envio_substatus)).length || 0

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div><h1 style={{ margin: 0 }}>Envíos</h1>
          <p className="muted sm" style={{ margin: 0 }}>Etiquetas de Flex y Centro de envío (colecta) pendientes de despacho, de todas tus cuentas.</p></div>
        <div className="row" style={{ gap: 8 }}>
          <select className="input" value={cuenta} onChange={(e) => setCuenta(e.target.value)} title="Cuenta de Mercado Libre">
            <option value="">Todas las cuentas</option>
            {cuentas.map((c) => <option key={c.id} value={c.id}>{c.nickname || `Usuario ${c.ml_user_id}`}</option>)}
          </select>
          <input className="input" type="date" value={dia} onChange={(e) => setDia(e.target.value)} title="Filtra por día (vacío = últimos 15 días)" />
          {dia && <button className="btn btn-sm" onClick={() => setDia('')}>Ver recientes</button>}
          <button className="btn btn-sm" onClick={cargar} title="Vuelve a consultar el estado a Mercado Libre">↻ Actualizar</button>
          <button className="btn btn-accent" disabled={busy || !pendientesCount} onClick={descargarDia}>
            {busy ? 'Preparando…' : `⬇ Descargar etiquetas${pendientesCount ? ` (${pendientesCount})` : ''}`}
          </button>
        </div>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}
      <div className="row between sm muted" style={{ marginTop: -4, flexWrap: 'wrap', gap: 8 }}>
        <span>{dia ? `Mostrando el ${dia}.` : 'Mostrando los últimos 15 días.'} Las ventas FULL no aparecen (las despacha Mercado Libre).</span>
        <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={incluir} onChange={(e) => setIncluir(e.target.checked)} /> Ver también los entregados</label>
      </div>

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!rows ? <div className="empty">Cargando…</div> : rows.length === 0 ? (
          <div className="empty">No hay envíos activos.<br /><span className="sm">Si acabas de vender, ve a <b>Ventas → Sincronizar ventas</b>. O activa «ver también los entregados».</span></div>
        ) : (
          <table style={{ minWidth: 760 }}>
            <thead><tr><th>Hora</th><th>Cuenta</th><th>Comprador</th><th>Productos</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {rows.map((o) => (
                <tr key={o.id} style={pendiente(o.envio_estado, o.envio_substatus) ? undefined : { opacity: 0.6 }}>
                  <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>{new Date(o.fecha + 'Z').toLocaleTimeString('es-CL', { hour: '2-digit', minute: '2-digit' })}</td>
                  <td><b>{o.cuenta}</b></td>
                  <td className="muted sm">{o.comprador || '—'}</td>
                  <td className="sm" style={{ maxWidth: 320 }}>{o.productos}</td>
                  <td>{estadoBadge(o.envio_estado, o.envio_substatus)}</td>
                  <td style={{ textAlign: 'right' }}>
                    {pendiente(o.envio_estado, o.envio_substatus)
                      ? <button className="btn btn-sm" onClick={() => descargarUna(o)}>Etiqueta</button>
                      : <span className="muted sm">—</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
