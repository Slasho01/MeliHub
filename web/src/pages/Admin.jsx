import { useEffect, useState } from 'react'
import { get, money, put, post, del } from '../api'

const PLANES = ['free', 'pro', 'business']
const CUPON_INI = { codigo: '', tipo: 'porcentaje', valor: '', ciclo: 'ambos', vence: '', usos_max: '' }
const PROMO_INI = { asunto: '', titulo: '', cuerpo: '', cta_texto: '', cta_url: '' }
const cicloLabel = (c) => ({ ambos: 'Mensual y anual', mensual: 'Solo mensual', anual: 'Solo anual' }[c] || c || '—')

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
  const [promo, setPromo] = useState(PROMO_INI)
  const [promoMsg, setPromoMsg] = useState(null)
  const [promoBusy, setPromoBusy] = useState(false)
  const [pruebaEmail, setPruebaEmail] = useState('')
  const [destino, setDestino] = useState('usuarios') // 'usuarios' | 'lista'
  const [listaEmails, setListaEmails] = useState('')
  const [diag, setDiag] = useState(null)

  const cargar = () => {
    get('/admin/metricas').then(setMetricas).catch((e) => setError(e.message))
    get('/admin/vendedores').then(setVends).catch((e) => setError(e.message))
    get('/admin/cupones').then(setCupones).catch(() => {})
    get('/admin/diag').then(setDiag).catch(() => {})
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
        ciclo: cupForm.ciclo,
        vence: cupForm.vence || null,
        usos_max: cupForm.usos_max === '' ? null : Number(cupForm.usos_max),
      })
      setCupForm(CUPON_INI); cargar()
    } catch (err) { setError(err.message) }
  }
  const toggleCupon = async (c) => { try { await put(`/admin/cupones/${c.id}`, { activo: !c.activo }); cargar() } catch (e) { setError(e.message) } }
  const borrarCupon = async (c) => { if (!window.confirm(`¿Eliminar el cupón ${c.codigo}?`)) return; try { await del(`/admin/cupones/${c.id}`); cargar() } catch (e) { setError(e.message) } }

  const setPromoK = (k) => (e) => setPromo({ ...promo, [k]: e.target.value })
  const parseEmails = (s) => [...new Set(s.split(/[\s,;]+/).map((x) => x.trim().toLowerCase()).filter((x) => x.includes('@') && x.length >= 5))]
  const enviarPromo = async (prueba_a) => {
    if (!promo.asunto.trim() || !promo.titulo.trim() || !promo.cuerpo.trim()) { setError('Completa asunto, título y cuerpo.'); return }
    let emails
    if (!prueba_a) {
      if (destino === 'lista') {
        emails = parseEmails(listaEmails)
        if (!emails.length) { setError('Agrega al menos un correo válido en la lista de posibles clientes.'); return }
        if (!window.confirm(`¿Enviar este correo a ${emails.length} posible(s) cliente(s)?`)) return
      } else if (!window.confirm('¿Enviar este correo a TODAS las cuentas activas? No se puede deshacer.')) return
    }
    setError(null); setPromoMsg(null); setPromoBusy(true)
    try {
      const r = await post('/admin/correo/promo', { ...promo, prueba_a: prueba_a || undefined, destino, emails })
      setPromoMsg(r.prueba ? `✅ Prueba enviada a ${prueba_a}.` : `✅ Enviado a ${r.enviados} de ${r.total} destinatario(s)${r.fallidos ? ` · ${r.fallidos} con error` : ''}.`)
    } catch (e) { setError(e.message) } finally { setPromoBusy(false) }
  }

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

      {diag && (
        <div className="card" style={{ borderLeft: '4px solid var(--accent)' }}>
          <div className="row between" style={{ flexWrap: 'wrap', gap: 8, alignItems: 'baseline' }}>
            <b className="sm">🩺 Diagnóstico de sincronización</b>
            <span className="muted xs">build: <code>{diag.build}</code> · {diag.productos} productos</span>
          </div>
          <div className="row" style={{ gap: 14, flexWrap: 'wrap', marginTop: 8 }}>
            {['falabella', 'shopify', 'paris'].map((canal) => {
              const c = diag[canal] || { total: 0, huerfanas: 0 }
              const malo = c.huerfanas > 0
              return (
                <div key={canal} style={{ minWidth: 150 }}>
                  <div className="muted xs" style={{ textTransform: 'capitalize' }}>{canal}</div>
                  <div className="sm">{c.total} mapeo(s){c.total > 0 && <> · <b style={{ color: malo ? 'var(--bad, #d93025)' : 'var(--ok)' }}>{c.huerfanas} huérfano(s)</b></>}</div>
                </div>
              )
            })}
          </div>
          {(diag.falabella?.huerfanas > 0 || diag.shopify?.huerfanas > 0 || diag.paris?.huerfanas > 0) && (
            <div className="alert alert-bad" style={{ marginTop: 8, display: 'block' }}>
              ⚠️ Hay mapeos de canal <b>huérfanos</b> (apuntan a productos que ya no existen, típicamente por reconectar una cuenta de ML). Esos canales <b>no reciben el stock</b> hasta volver a publicar/enlazar esos productos. Esta es casi seguro la causa de que no se propague el stock.
            </div>
          )}
        </div>
      )}
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
        <div style={{ flex: '0 1 150px' }}><label>Aplica a</label>
          <select className="input" value={cupForm.ciclo} onChange={setCup('ciclo')} title="En qué suscripción se puede usar">
            <option value="ambos">Mensual y anual</option>
            <option value="mensual">Solo mensual</option>
            <option value="anual">Solo anual</option>
          </select>
        </div>
        <div style={{ flex: '0 1 150px' }}><label>Vence (opcional)</label><input className="input" type="date" value={cupForm.vence} onChange={setCup('vence')} /></div>
        <div style={{ flex: '0 1 110px' }}><label>Usos máx.</label><input className="input" type="number" min="1" placeholder="∞" value={cupForm.usos_max} onChange={setCup('usos_max')} /></div>
        <button className="btn btn-accent">Crear cupón</button>
      </form>

      <div className="card" style={{ padding: 0, overflowX: 'auto' }}>
        {!cupones ? <div className="empty">Cargando…</div> : cupones.length === 0 ? (
          <div className="empty">Sin cupones. Crea uno arriba.</div>
        ) : (
          <table style={{ minWidth: 720 }}>
            <thead><tr><th>Código</th><th>Descuento</th><th>Aplica a</th><th>Vence</th><th>Usos</th><th>Estado</th><th></th></tr></thead>
            <tbody>
              {cupones.map((c) => (
                <tr key={c.id} style={!c.activo ? { opacity: 0.55 } : undefined}>
                  <td><b>{c.codigo}</b></td>
                  <td>{c.tipo === 'porcentaje' ? `${c.valor}%` : money(c.valor)}</td>
                  <td className="muted sm">{cicloLabel(c.ciclo)}</td>
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

      <h2 style={{ fontSize: 16, margin: '8px 0 0' }}>Correo de promoción</h2>
      <div className="card stack" style={{ gap: 10 }}>
        <p className="muted sm" style={{ margin: 0 }}>Envía un correo con la plantilla de Ventoki a todas las cuentas activas. Prueba primero contigo antes de enviar a todos.</p>
        {promoMsg && <div className="alert alert-ok">{promoMsg}</div>}
        <div><label>Asunto</label><input className="input" value={promo.asunto} onChange={setPromoK('asunto')} placeholder="Novedades de Ventoki este mes" /></div>
        <div><label>Título (dentro del correo)</label><input className="input" value={promo.titulo} onChange={setPromoK('titulo')} placeholder="Nuevas funciones para tu e-commerce 🚀" /></div>
        <div><label>Cuerpo</label><textarea className="input" rows={6} value={promo.cuerpo} onChange={setPromoK('cuerpo')} placeholder="Escribe tu mensaje. Deja una línea en blanco para separar párrafos." style={{ resize: 'vertical' }} /></div>
        <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
          <div style={{ flex: '1 1 160px' }}><label>Texto del botón (opcional)</label><input className="input" value={promo.cta_texto} onChange={setPromoK('cta_texto')} placeholder="Ver novedades" /></div>
          <div style={{ flex: '1 1 220px' }}><label>Enlace del botón (opcional)</label><input className="input" value={promo.cta_url} onChange={setPromoK('cta_url')} placeholder="https://ventoki.com" /></div>
        </div>
        <div>
          <label>Enviar a</label>
          <div className="row" style={{ gap: 16, marginTop: 4, flexWrap: 'wrap' }}>
            <label className="row sm" style={{ gap: 6, alignItems: 'center' }}><input type="radio" name="destino" checked={destino === 'usuarios'} onChange={() => setDestino('usuarios')} /> Usuarios existentes <span className="muted">(cuentas activas)</span></label>
            <label className="row sm" style={{ gap: 6, alignItems: 'center' }}><input type="radio" name="destino" checked={destino === 'lista'} onChange={() => setDestino('lista')} /> Posibles clientes <span className="muted">(lista de correos)</span></label>
          </div>
        </div>
        {destino === 'lista' && (
          <div>
            <label>Correos de posibles clientes</label>
            <textarea className="input" rows={4} value={listaEmails} onChange={(e) => setListaEmails(e.target.value)} placeholder="Pega los correos separados por coma, espacio o salto de línea" style={{ resize: 'vertical' }} />
            <span className="muted xs">{parseEmails(listaEmails).length} correo(s) válido(s) detectado(s).</span>
          </div>
        )}
        <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
          <div className="row" style={{ gap: 6, alignItems: 'flex-end' }}>
            <div style={{ flex: '1 1 200px' }}><label>Enviar prueba a</label><input className="input" type="email" value={pruebaEmail} onChange={(e) => setPruebaEmail(e.target.value)} placeholder="tu@correo.com" /></div>
            <button className="btn" disabled={promoBusy || !pruebaEmail.trim()} onClick={() => enviarPromo(pruebaEmail.trim())}>Enviar prueba</button>
          </div>
          <button className="btn btn-accent" disabled={promoBusy} onClick={() => enviarPromo(null)}>{promoBusy ? 'Enviando…' : (destino === 'lista' ? '📣 Enviar a la lista' : '📣 Enviar a todos')}</button>
        </div>
      </div>
    </div>
  )
}
