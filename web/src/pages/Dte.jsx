import { useEffect, useState } from 'react'
import { get, post, del } from '../api'

export default function Dte() {
  const [cuentas, setCuentas] = useState(null)
  const [form, setForm] = useState({ api_token: '', rut_emisor: '', razon_social: '', certificacion: true })
  const [error, setError] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)

  const cargar = () => { setCuentas(null); get('/dte/cuentas').then(setCuentas).catch((e) => setError(e.message)) }
  useEffect(cargar, [])

  const conectar = async (e) => {
    e.preventDefault()
    if (!form.api_token.trim() || !form.rut_emisor.trim()) { setError('Ingresa el token de LibreDTE y el RUT emisor.'); return }
    setBusy(true); setError(''); setMsg('')
    try {
      await post('/dte/cuentas', form)
      setMsg('Cuenta de facturación conectada.')
      setForm({ api_token: '', rut_emisor: '', razon_social: '', certificacion: true })
      cargar()
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }
  const desconectar = async (c) => {
    if (!window.confirm('¿Desconectar esta cuenta de facturación?')) return
    setError('')
    try { await del(`/dte/cuentas/${c.id}`); cargar() } catch (e) { setError(e.message) }
  }

  return (
    <div className="stack">
      <div>
        <h1 style={{ margin: 0 }}>Facturación electrónica (SII)</h1>
        <p className="muted sm" style={{ margin: 0 }}>Conecta tu cuenta de LibreDTE para emitir boletas y facturas de tus ventas (ideal para WooCommerce y cotizaciones; Mercado Libre ya factura por su cuenta).</p>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {msg && <div className="alert alert-ok">{msg}</div>}

      <div className="row" style={{ gap: 12, alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <form className="card stack" style={{ gap: 10, flex: '1 1 340px' }} onSubmit={conectar}>
          <b className="sm">Conectar LibreDTE</b>
          <label>Token (hash) de LibreDTE
            <input className="input" type="password" autoComplete="off" placeholder="Tu API token de LibreDTE" value={form.api_token} onChange={(e) => setForm({ ...form, api_token: e.target.value })} />
          </label>
          <label>RUT emisor
            <input className="input" placeholder="76192083-9" value={form.rut_emisor} onChange={(e) => setForm({ ...form, rut_emisor: e.target.value })} />
          </label>
          <label>Razón social (opcional)
            <input className="input" placeholder="Mi Empresa SpA" value={form.razon_social} onChange={(e) => setForm({ ...form, razon_social: e.target.value })} />
          </label>
          <label className="row" style={{ gap: 8, alignItems: 'center', fontWeight: 400 }}>
            <input type="checkbox" checked={form.certificacion} onChange={(e) => setForm({ ...form, certificacion: e.target.checked })} />
            <span className="sm">Ambiente de <b>certificación</b> (pruebas del SII). Desmárcalo solo cuando emitas en producción.</span>
          </label>
          <button className="btn btn-dark" disabled={busy}>{busy ? 'Conectando…' : 'Conectar'}</button>
          <p className="muted xs" style={{ margin: 0 }}>Tu token se guarda cifrado. Necesitas tu <b>certificado digital</b> y <b>folios (CAF)</b> cargados en LibreDTE para emitir.</p>
        </form>

        <div className="card stack" style={{ gap: 8, flex: '1 1 300px' }}>
          <b className="sm">Cuentas conectadas</b>
          {cuentas == null ? <div className="muted sm">Cargando…</div> : cuentas.length === 0 ? (
            <div className="muted sm">Aún no conectas ninguna cuenta de facturación.</div>
          ) : cuentas.map((c) => (
            <div key={c.id} className="row between" style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
              <div>
                <b>{c.razon_social || c.rut_emisor}</b>
                <div className="muted xs">{c.rut_emisor} · {c.proveedor}{c.certificacion ? ' · certificación' : ' · producción'}</div>
              </div>
              <button className="btn btn-sm" onClick={() => desconectar(c)}>Desconectar</button>
            </div>
          ))}
        </div>
      </div>

      <div className="card" style={{ borderLeft: '3px solid var(--warn)' }}>
        <b className="sm">Próximo paso</b>
        <p className="muted sm" style={{ margin: '4px 0 0' }}>Una vez conectada tu cuenta y validada la emisión, podrás emitir <b>boleta/factura</b> desde el <b>Cotizador</b> y desde tus ventas. La emisión real se habilita cuando validemos tu token.</p>
      </div>
    </div>
  )
}
