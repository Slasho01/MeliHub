import { useEffect, useState } from 'react'
import { get, post } from '../api'

export default function ParisPublicar({ cuenta }) {
  const [familias, setFamilias] = useState(null)
  const [error, setError] = useState(null)
  const [result, setResult] = useState(null)
  const [busy, setBusy] = useState(false)
  const [atributos, setAtributos] = useState([{ code: '', value: '' }])
  const [form, setForm] = useState({ name: '', ref_product: '', family_code: '', precio: '', oferta: '' })

  useEffect(() => {
    setFamilias(null); setError(null)
    get(`/paris/cuentas/${cuenta.id}/familias`).then(setFamilias).catch((e) => setError(e.message))
  }, [cuenta.id])

  // La API puede devolver un array directo o { data: [...] }.
  const famList = Array.isArray(familias) ? familias : (familias?.data || [])

  const setAttr = (i, k, v) => setAtributos((a) => a.map((x, j) => (j === i ? { ...x, [k]: v } : x)))
  const addAttr = () => setAtributos((a) => [...a, { code: '', value: '' }])
  const rmAttr = (i) => setAtributos((a) => a.filter((_, j) => j !== i))

  const publicar = async (e) => {
    e.preventDefault()
    setBusy(true); setError(null); setResult(null)
    try {
      const body = {
        name: form.name,
        ref_product: form.ref_product,
        family_code: form.family_code,
        precio: Number(form.precio) || 0,
        oferta: form.oferta ? Number(form.oferta) : null,
        atributos: atributos.filter((a) => a.code && a.value),
      }
      const r = await post(`/paris/cuentas/${cuenta.id}/producto`, body)
      setResult(r)
    } catch (err) { setError(err.message) } finally { setBusy(false) }
  }

  return (
    <form className="stack" style={{ gap: 10 }} onSubmit={publicar}>
      {error && <div className="alert alert-bad">{error}</div>}
      {result && <div className="alert alert-ok">Producto enviado a Paris. <span className="muted sm">Revisa su aprobación en tu Seller Center.</span></div>}

      <input className="input" placeholder="Nombre del producto" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} required />
      <input className="input" placeholder="SKU / referencia (ej. el id de Mercado Libre)" value={form.ref_product} onChange={(e) => setForm({ ...form, ref_product: e.target.value })} required />

      <label className="sm muted">Familia</label>
      <select className="input" value={form.family_code} onChange={(e) => setForm({ ...form, family_code: e.target.value })} required>
        <option value="">{familias == null ? 'Cargando familias…' : '— Elige una familia —'}</option>
        {famList.map((f) => <option key={f.code || f.id} value={f.code || f.id}>{f.name || f.nombre || f.code}</option>)}
      </select>

      <div className="row" style={{ gap: 8 }}>
        <input className="input" type="number" placeholder="Precio (CLP)" value={form.precio} onChange={(e) => setForm({ ...form, precio: e.target.value })} required />
        <input className="input" type="number" placeholder="Precio oferta (opcional)" value={form.oferta} onChange={(e) => setForm({ ...form, oferta: e.target.value })} />
      </div>

      <div className="stack" style={{ gap: 6 }}>
        <div className="row between"><span className="sm muted">Atributos (según la familia)</span><button type="button" className="btn btn-sm" onClick={addAttr}>+ Atributo</button></div>
        {atributos.map((a, i) => (
          <div key={i} className="row" style={{ gap: 6 }}>
            <input className="input" placeholder="código (ej. color)" value={a.code} onChange={(e) => setAttr(i, 'code', e.target.value)} />
            <input className="input" placeholder="valor (ej. Rojo)" value={a.value} onChange={(e) => setAttr(i, 'value', e.target.value)} />
            <button type="button" className="btn btn-sm" onClick={() => rmAttr(i)}>✕</button>
          </div>
        ))}
        <p className="muted sm" style={{ margin: 0 }}>Los códigos válidos dependen de la familia elegida (los entrega la API de atributos de esa familia).</p>
      </div>

      <button className="btn btn-dark" disabled={busy}>{busy ? 'Publicando…' : 'Publicar en Paris'}</button>
    </form>
  )
}
