import { useEffect, useState } from 'react'
import { get, money, put } from '../api'

const PLANES = ['free', 'pro', 'business']

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
  const [error, setError] = useState(null)

  const cargar = () => {
    get('/admin/metricas').then(setMetricas).catch((e) => setError(e.message))
    get('/admin/vendedores').then(setVends).catch((e) => setError(e.message))
  }
  useEffect(() => { cargar() }, [])

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
    </div>
  )
}
