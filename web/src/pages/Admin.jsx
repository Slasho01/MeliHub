import { useEffect, useState } from 'react'
import { get, money, put, post, del } from '../api'

const PLANES = ['free', 'pro', 'business']
const CUPON_INI = { codigo: '', tipo: 'porcentaje', valor: '', vence: '', usos_max: '' }

function Kpi({ label, value }) {
  return (
    <div className="card" style={{ flex: '1 1 130px' }}>
      <div className="muted sm">{label}</div>
      <div style={{ fontSize: 20, fontWeight: 700 }}>{value}</div>
    </div>
  )
}

export default function Admin() {
  const [metricas, setMetricas] = useState(null)
  const [vends, setVends] = useState(null)
  const [cupones, setCupones] = useState(null)
  const [cupForm, setCupForm] = useState(CUPON_INI)
  const [error, setError] = useState(null)

  const cargar = () => {
    get('/admin/metricas').then(setMetricas).catch((e) => setError(e.message))
    get('/admin/vendedores').then(setVends).catch((e) => setError(e.message))
    get('/admin/cupones').then(setCupones).catch(() => {})
  }
  useEffect(() => { cargar() }, [])

  const setCup = (k) => (e) => setCupForm({ ...cupForm, [k]: e.target.value })
  const crearCupon = async (e) => {
    e.preventDefault(); setError(null)
    try {
      await post('/admin/cupones', {
        codigo: cupForm.codigo,
        tipo: cupForm.tipo,
        valor: Number(cupForm.valor),
        vence: cupForm.vence || null,
        usos_max: cupForm.usos_max === '' ? null : Number(cupForm.usos_max),
      })
      setCupForm(CUPON_INI); cargar()
    } catch (err) { setError(err.message) }
  }
  const toggleCupon = async (c) => { try { await put(`/admin/cupones/${c.id}`, { activo: !c.activo }); cargar() } catch (e) { setError(e.message) } }
  const borrarCupon = async (c) => { if (!window.confirm(`¿Eliminar el cupón ${c.codigo}?`)) return; try { await del(`/admin/cupones/${c.id}`); cargar() } catch (e) { setError(e.message) } }

  const cambiarPlan = async (v, plan) => {
    try { await put(`/admin/vendedores/${v.id}/plan`, { plan }); cargar() } catch (e) { setError(e.message) }
  }
  const toggleActivo = async (v) => {
    const accion = v.activo ? 'suspender' : 'reactivar'
    if (!window.confirm(`¿${accion} a ${v.nombre}?`)) return
    try { await put(`/admin/vendedores/${v.id}/estado`, { activo: !v.activo }); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div><h1>Administración (plataforma)</h1><p className="muted sm" style={{ margin: 0 }}>Clientes, planes y métricas globales.</p></div>
      {error && <div className="alert alert-bad">{error}</div>}

      {metricas && (
        <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
          <Kpi label="Clientes" value={metricas.vendedores} />
          <Kpi label="Cuentas ML" value={metricas.cuentas} />
          <Kpi label="Productos" value={metricas.productos} />
          <Kpi label="Órdenes" value={metricas.ordenes} />
          <Kpi label="Ventas" value={money(metricas.ventas_total)} />
        </div>
      )}

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!vends ? <div className="empty">Cargando…</div> : vends.length === 0 ? (
          <div className="empty">Sin clientes todavía.</div>
        ) : (
          <table style={{ minWidth: 820 }}>
            <thead><tr><th>Cliente</th><th>Rol</th><th>Plan</th><th>Cuentas</th><th>Productos</th><th>Estado</th></tr></thead>
            <tbody>
              {vends.map((v) => (
                <tr key={v.id} style={!v.activo ? { opacity: 0.5 } : undefined}>
                  <td><b>{v.nombre}</b><div className="muted sm">{v.email}</div></td>
                  <td><span className="badge">{v.rol}</span></td>
                  <td>
                    <select className="input" value={v.plan} onChange={(e) => cambiarPlan(v, e.target.value)}>
                      {PLANES.map((p) => <option key={p} value={p}>{p}</option>)}
                    </select>
                  </td>
                  <td>{v.cuentas}</td>
                  <td>{v.productos}</td>
                  <td>
                    <button className={`btn btn-sm ${v.activo ? 'btn-danger' : 'btn-accent'}`} onClick={() => toggleActivo(v)}>
                      {v.activo ? 'Suspender' : 'Reactivar'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <h2 style={{ fontSize: 16, margin: '8px 0 0' }}>Cupones de descuento</h2>
      <form className="card row" onSubmit={crearCupon} style={{ gap: 8, flexWrap: 'wrap', alignItems: 'flex-end' }}>
        <div style={{ flex: '1 1 130px' }}><label>Código</label><input className="input" style={{ textTransform: 'uppercase' }} value={cupForm.codigo} onChange={setCup('codigo')} required /></div>
        <div style={{ flex: '0 1 140px' }}><label>Tipo</label>
          <select className="input" value={cupForm.tipo} onChange={setCup('tipo')}>
            <option value="porcentaje">Porcentaje (%)</option>
            <option value="monto">Monto fijo (CLP)</option>
          </select>
        </div>
        <div style={{ flex: '0 1 110px' }}><label>Valor</label><input className="input" type="number" min="1" value={cupForm.valor} onChange={setCup('valor')} required /></div>
        <div style={{ flex: '0 1 150px' }}><label>Vence (opcional)</label><input className="input" type="date" value={cupForm.vence} onChange={setCup('vence')} /></div>
        <div style={{ flex: '0 1 110px' }}><label>Usos máx.</label><input className="input" type="number" min="1" placeholder="∞" value={cupForm.usos_max} onChange={setCup('usos_max')} /></div>
        <button className="btn btn-accent">Crear cupón</button>
      </form>

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!cupones ? <div className="empty">Cargando…</div> : cupones.length === 0 ? (
          <div className="empty">Sin cupones. Crea uno arriba.</div>
        ) : (
          <table style={{ minWidth: 720 }}>
            <thead><tr><th>Código</th><th>Descuento</th><th>Vence</th><th>Usos</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {cupones.map((c) => (
                <tr key={c.id} style={!c.activo ? { opacity: 0.55 } : undefined}>
                  <td><b>{c.codigo}</b></td>
                  <td>{c.tipo === 'porcentaje' ? `${c.valor}%` : money(c.valor)}</td>
                  <td className="muted sm">{c.vence ? new Date(c.vence + 'Z').toLocaleDateString('es-CL') : '—'}</td>
                  <td>{c.usos}{c.usos_max != null ? ` / ${c.usos_max}` : ''}</td>
                  <td>{c.activo ? <span className="tag tag-ok">activo</span> : <span className="tag">inactivo</span>}</td>
                  <td className="row" style={{ gap: 6, justifyContent: 'flex-end' }}>
                    <button className="btn btn-sm" onClick={() => toggleCupon(c)}>{c.activo ? 'Desactivar' : 'Activar'}</button>
                    <button className="btn btn-sm btn-danger" onClick={() => borrarCupon(c)}>Eliminar</button>
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
