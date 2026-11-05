import { useEffect, useState } from 'react'
import { get, put } from '../api'

export default function Ganancias() {
  const [cfg, setCfg] = useState(null)
  const [error, setError] = useState(null)
  const [msg, setMsg] = useState(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => { get('/config/ganancias').then(setCfg).catch((e) => setError(e.message)) }, [])
  const set = (k, v) => setCfg({ ...cfg, [k]: v })

  const guardar = async () => {
    setBusy(true); setError(null); setMsg(null)
    try {
      const c = await put('/config/ganancias', {
        precios_con_iva: cfg.precios_con_iva,
        iva_pct: Number(cfg.iva_pct),
        comision_ml_pct: Number(cfg.comision_ml_pct),
        comision_ml_fija: Number(cfg.comision_ml_fija),
        comision_mp_pct: Number(cfg.comision_mp_pct),
        comision_mp_fija: Number(cfg.comision_mp_fija),
        margen_objetivo: Number(cfg.margen_objetivo),
        redondeo: Number(cfg.redondeo),
      })
      setCfg(c); setMsg('Configuración guardada. Las ganancias se recalculan con estas tasas.')
    } catch (e) { setError(e.message) } finally { setBusy(false) }
  }

  if (!cfg) return <div className="empty">Cargando…</div>
  const num = (k, label, step = '0.1') => (
    <div><label>{label}</label><input className="input" type="number" step={step} min="0" value={cfg[k]} onChange={(e) => set(k, e.target.value)} /></div>
  )

  return (
    <div className="stack" style={{ maxWidth: 560 }}>
      <div><h1>Ganancias e impuestos</h1><p className="muted sm" style={{ margin: 0 }}>Tasas para calcular la ganancia por canal.</p></div>
      {msg && <div className="alert alert-ok">{msg}</div>}
      {error && <div className="alert alert-bad">{error}</div>}
      <div className="card stack">
        <label className="row" style={{ gap: 8, cursor: 'pointer' }}>
          <input type="checkbox" checked={cfg.precios_con_iva} onChange={(e) => set('precios_con_iva', e.target.checked)} />
          <span>Los precios que ingreso <b>incluyen IVA</b> (el neto = precio ÷ (1 + IVA))</span>
        </label>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
          {num('iva_pct', 'IVA (%)')}
          <div />
          {num('comision_ml_pct', 'Comisión Mercado Libre (%)')}
          {num('comision_ml_fija', 'Cargo fijo ML (CLP)', '1')}
          {num('comision_mp_pct', 'Comisión Mercado Pago / JarStore (%)')}
          {num('comision_mp_fija', 'Cargo fijo MP (CLP)', '1')}
        </div>
        <div style={{ borderTop: '1px solid var(--line)', paddingTop: 12 }}>
          <b>Reglas de precio (sugerencia)</b>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14, marginTop: 8 }}>
            {num('margen_objetivo', 'Margen objetivo (%)')}
            {num('redondeo', 'Precio termina en (ej. 990; 0 = sin redondeo)', '1')}
          </div>
          <p className="muted sm" style={{ margin: '6px 0 0' }}>En Inventario, los productos con costo mostrarán un <b>precio JarStore sugerido</b> que alcanza este margen.</p>
        </div>
        <div><button className="btn btn-accent" onClick={guardar} disabled={busy}>{busy ? 'Guardando…' : 'Guardar'}</button></div>
      </div>
      <p className="muted sm">Fórmula por canal: <b>Ganancia = Precio neto − Costo − Comisión</b>. El costo se trata como neto (sin IVA). Son estimaciones; ajusta las tasas a tu caso real.</p>
    </div>
  )
}
