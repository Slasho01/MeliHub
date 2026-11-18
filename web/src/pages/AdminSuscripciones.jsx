import { useEffect, useState } from 'react'
import { get, post, del } from '../api'

const money = (n) => '$' + Number(n || 0).toLocaleString('es-CL')
const fecha = (s) => (s ? new Date(s).toLocaleDateString('es-CL') : '—')

// Período sugerido según el ciclo: mensual → "2026-11", anual → "2026".
function periodoDefault(ciclo) {
  const d = new Date()
  return ciclo === 'anual' ? String(d.getFullYear()) : `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
}

function leerDataUrl(file) {
  return new Promise((res, rej) => {
    const fr = new FileReader()
    fr.onload = () => res(fr.result)
    fr.onerror = rej
    fr.readAsDataURL(file)
  })
}

const estadoTag = (e) => {
  const ok = e === 'authorized' || e === 'active'
  return <span className={`tag ${ok ? 'tag-ok' : ''}`}>{e}</span>
}

export default function AdminSuscripciones() {
  const [subs, setSubs] = useState(null)
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [periodos, setPeriodos] = useState({}) // vendedor_id -> período escrito
  const [subiendo, setSubiendo] = useState('')

  const cargar = () => get('/admin/suscripciones').then(setSubs).catch((e) => setError(e.message))
  useEffect(() => { cargar() }, [])

  const periodoDe = (s) => (periodos[s.vendedor_id] ?? periodoDefault(s.ciclo))

  const subir = async (s, file) => {
    if (!file) return
    const periodo = periodoDe(s).trim()
    if (!periodo) { setError('Indica el período.'); return }
    if (file.size > 5 * 1024 * 1024) { setError('El archivo supera 5 MB.'); return }
    setSubiendo(s.vendedor_id); setError(''); setMsg('')
    try {
      const archivo = await leerDataUrl(file)
      await post(`/admin/vendedores/${s.vendedor_id}/boleta`, { periodo, nombre_archivo: file.name, archivo })
      setMsg(`Boleta subida para ${s.email} (${periodo}) ✓`); cargar()
    } catch (e) { setError(e.message) } finally { setSubiendo('') }
  }

  const descargar = async (b) => {
    try {
      const r = await get(`/admin/boletas/${b.id}`)
      const a = document.createElement('a'); a.href = r.archivo; a.download = r.nombre_archivo || 'boleta'; a.click()
    } catch (e) { setError(e.message) }
  }
  const borrar = async (id) => {
    if (!confirm('¿Borrar esta boleta?')) return
    try { await del(`/admin/boletas/${id}`); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div>
      <h1 style={{ margin: 0 }}>Suscripciones</h1>
      <p className="muted sm" style={{ margin: '2px 0 14px' }}>Cada cliente, su plan y ciclo. Sube la boleta/factura por período (mensual o anual).</p>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      {!subs ? <p className="muted">Cargando…</p> : subs.length === 0 ? (
        <p className="muted">Aún no hay suscripciones.</p>
      ) : (
        <div className="stack" style={{ '--gap': '14px' }}>
          {subs.map((s) => (
            <div key={s.vendedor_id + s.created_at} className="card">
              <div className="row" style={{ justifyContent: 'space-between', flexWrap: 'wrap', gap: 8, alignItems: 'flex-start' }}>
                <div>
                  <b>{s.nombre || s.email}</b> <span className="muted sm">· {s.email}</span>
                  <div className="muted sm" style={{ marginTop: 2 }}>
                    Plan <b style={{ textTransform: 'capitalize' }}>{s.plan}</b> · ciclo <b>{s.ciclo}</b> · {money(s.monto)} · {estadoTag(s.estado)}
                  </div>
                  <div className="muted sm">Desde {fecha(s.created_at)} · vigente hasta {fecha(s.vigente_hasta)}</div>
                </div>
                <div className="row" style={{ gap: 8, alignItems: 'center' }}>
                  <input
                    className="input" style={{ width: 120 }}
                    value={periodoDe(s)}
                    onChange={(e) => setPeriodos((p) => ({ ...p, [s.vendedor_id]: e.target.value }))}
                    title="Período (ej. 2026-11 o 2026)"
                  />
                  <label className="btn btn-sm" style={{ cursor: 'pointer' }}>
                    {subiendo === s.vendedor_id ? 'Subiendo…' : '⬆ Subir boleta'}
                    <input type="file" accept="application/pdf,image/*" style={{ display: 'none' }}
                      onChange={(e) => { subir(s, e.target.files?.[0]); e.target.value = '' }} />
                  </label>
                </div>
              </div>

              {s.boletas?.length > 0 && (
                <table className="table" style={{ marginTop: 10, marginBottom: 0 }}>
                  <tbody>
                    {s.boletas.map((b) => (
                      <tr key={b.id}>
                        <td style={{ width: 90 }}><b>{b.periodo}</b></td>
                        <td style={{ fontSize: 13 }}>{b.nombre_archivo}</td>
                        <td className="muted sm" style={{ whiteSpace: 'nowrap' }}>{fecha(b.subida_at)}</td>
                        <td style={{ whiteSpace: 'nowrap', textAlign: 'right' }}>
                          <button className="btn btn-sm" onClick={() => descargar(b)}>⬇ Descargar</button>
                          <button className="btn btn-sm" style={{ marginLeft: 6 }} onClick={() => borrar(b.id)}>✕</button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
