import { useEffect, useState } from 'react'
import { get, post, put, del } from '../api'

const ROLES = [
  ['operador', 'Operador — edita inventario, precios y sincroniza'],
  ['lectura', 'Solo lectura — solo consulta, no modifica'],
]
const rolLabel = (r) => (r === 'operador' ? 'Operador' : r === 'lectura' ? 'Solo lectura' : r)

export default function Equipo() {
  const [data, setData] = useState(null)
  const [form, setForm] = useState({ nombre: '', email: '', rol: 'operador' })
  const [msg, setMsg] = useState(null)
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState(false)

  const load = () => get('/equipo').then(setData).catch((e) => setError(e.message))
  useEffect(() => { load() }, [])

  const invitar = async (e) => {
    e.preventDefault(); setBusy(true); setError(null); setMsg(null)
    try {
      const r = await post('/equipo', form)
      setForm({ nombre: '', email: '', rol: 'operador' })
      setMsg(r.correo_enviado ? `Invitación enviada a ${r.miembro.email}.` : `Invitación creada. Comparte este enlace con la persona: ${r.enlace}`)
      load()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }
  const cambiarRol = async (id, rol) => { try { await put(`/equipo/${id}`, { rol }); load() } catch (e) { setError(e.message) } }
  const eliminar = async (id, nombre) => {
    if (!confirm(`¿Eliminar a ${nombre}? Perderá el acceso de inmediato.`)) return
    try { await del(`/equipo/${id}`); load() } catch (e) { setError(e.message) }
  }
  const reinvitar = async (id) => {
    setError(null); setMsg(null)
    try { const r = await post(`/equipo/${id}/reinvitar`); setMsg(r.correo_enviado ? 'Invitación reenviada por correo.' : `Nuevo enlace: ${r.enlace}`) } catch (e) { setError(e.message) }
  }

  if (!data) return <div className="muted">Cargando…</div>
  const gest = data.puede_gestionar
  const set = (k) => (e) => setForm({ ...form, [k]: e.target.value })

  return (
    <div className="stack">
      <h1>Equipo</h1>
      <p className="muted sm" style={{ marginTop: -8 }}>Invita a personas a tu cuenta con permisos acotados. Tú (dueño) no apareces en la lista.</p>
      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok" style={{ wordBreak: 'break-all' }}>{msg}</div>}

      {gest && (
        <form className="card stack" onSubmit={invitar}>
          <b>Invitar a alguien</b>
          <div className="row" style={{ gap: 8, flexWrap: 'wrap', alignItems: 'flex-end' }}>
            <div style={{ flex: '1 1 160px' }}><label>Nombre</label><input className="input" value={form.nombre} onChange={set('nombre')} required /></div>
            <div style={{ flex: '1 1 200px' }}><label>Email</label><input className="input" type="email" value={form.email} onChange={set('email')} required /></div>
            <div style={{ flex: '1 1 220px' }}><label>Rol</label>
              <select className="input" value={form.rol} onChange={set('rol')}>
                {ROLES.map(([v, t]) => <option key={v} value={v}>{t}</option>)}
              </select>
            </div>
            <button className="btn btn-accent" disabled={busy}>{busy ? '…' : 'Invitar'}</button>
          </div>
        </form>
      )}

      <div className="card">
        {data.miembros.length === 0 ? (
          <div className="muted">Aún no has invitado a nadie.</div>
        ) : (
          <table className="table">
            <thead><tr><th>Nombre</th><th>Email</th><th>Rol</th><th>Estado</th>{gest && <th></th>}</tr></thead>
            <tbody>
              {data.miembros.map((m) => (
                <tr key={m.id}>
                  <td>{m.nombre}</td>
                  <td className="sm">{m.email}</td>
                  <td>
                    {gest
                      ? <select className="input input-sm" value={m.rol} onChange={(e) => cambiarRol(m.id, e.target.value)}>
                          {ROLES.map(([v]) => <option key={v} value={v}>{rolLabel(v)}</option>)}
                        </select>
                      : rolLabel(m.rol)}
                  </td>
                  <td>{m.estado === 'activo'
                    ? <span className="tag tag-ok">activo</span>
                    : <span className="tag">invitado</span>}</td>
                  {gest && (
                    <td className="row" style={{ gap: 6, justifyContent: 'flex-end' }}>
                      {m.estado === 'invitado' && <button className="btn btn-sm" onClick={() => reinvitar(m.id)}>Reenviar</button>}
                      <button className="btn btn-sm btn-bad" onClick={() => eliminar(m.id, m.nombre)}>Eliminar</button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
