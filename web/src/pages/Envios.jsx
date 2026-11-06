import { useEffect, useState } from 'react'
import { get, descargarArchivo } from '../api'

const ESTADO = {
  ready_to_ship: ['Listo para enviar', 'tag-ok'],
  pending: ['Preparando', 'tag'],
  handling: ['Preparando', 'tag'],
  shipped: ['Enviado', 'tag'],
  delivered: ['Entregado', 'tag-ok'],
  not_delivered: ['No entregado', 'tag'],
  cancelled: ['Cancelado', 'tag'],
}
const TERMINALES = ['shipped', 'delivered', 'not_delivered', 'cancelled']
const pendiente = (s) => !TERMINALES.includes(s)
const estadoBadge = (s) => {
  const [t, c] = ESTADO[s] || [s || '—', 'tag']
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
    if (incluir) p.set('incluir_enviados', '1')
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

  const pendientesCount = rows?.filter((o) => pendiente(o.envio_estado)).length || 0

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
        <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={incluir} onChange={(e) => setIncluir(e.target.checked)} /> Ver también los ya enviados</label>
      </div>

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!rows ? <div className="empty">Cargando…</div> : rows.length === 0 ? (
          <div className="empty">No hay envíos pendientes.<br /><span className="sm">Si acabas de vender, ve a <b>Ventas → Sincronizar ventas</b>. O activa «ver también los ya enviados».</span></div>
        ) : (
          <table style={{ minWidth: 760 }}>
            <thead><tr><th>Hora</th><th>Cuenta</th><th>Comprador</th><th>Productos</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {rows.map((o) => (
                <tr key={o.id} style={pendiente(o.envio_estado) ? undefined : { opacity: 0.6 }}>
                  <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>{new Date(o.fecha + 'Z').toLocaleTimeString('es-CL', { hour: '2-digit', minute: '2-digit' })}</td>
                  <td><b>{o.cuenta}</b></td>
                  <td className="muted sm">{o.comprador || '—'}</td>
                  <td className="sm" style={{ maxWidth: 320 }}>{o.productos}</td>
                  <td>{estadoBadge(o.envio_estado)}</td>
                  <td style={{ textAlign: 'right' }}>
                    {pendiente(o.envio_estado)
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
