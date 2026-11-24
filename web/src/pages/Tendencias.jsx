import { useEffect, useState } from 'react'
import { get, money } from '../api'

function Badge({ tengo }) {
  if (tengo === undefined) return null
  return tengo
    ? <span className="tag tag-ok" style={{ whiteSpace: 'nowrap' }} title="Ya tienes productos de esta tendencia">✓ lo tienes</span>
    : <span className="tag" style={{ whiteSpace: 'nowrap', color: 'var(--accent)' }} title="No tienes nada de esto — oportunidad para publicar">⚡ oportunidad</span>
}

export default function Tendencias() {
  const [cat, setCat] = useState('')
  const [data, setData] = useState(null)
  const [cats, setCats] = useState([])
  const [cargando, setCargando] = useState(false)
  const [error, setError] = useState('')

  const cargar = (categoria) => {
    setCargando(true); setError('')
    get('/tendencias' + (categoria ? `?categoria=${categoria}` : ''))
      .then((d) => { setData(d); if (Array.isArray(d.categorias)) setCats(d.categorias) })
      .catch((e) => setError(e.message)).finally(() => setCargando(false))
  }
  useEffect(() => { cargar('') }, [])
  const onCat = (c) => { setCat(c); cargar(c) }

  const buscado = Array.isArray(data?.buscado) ? data.buscado : []
  const crece = buscado.slice(0, 10)
  const masBuscado = buscado.slice(10)
  const vendido = Array.isArray(data?.vendido) ? data.vendido : []

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Tendencias</h1>
          <p className="muted sm" style={{ margin: 0 }}>Lo más buscado y lo más vendido en Mercado Libre Chile (semanal). <span className="tag tag-ok">✓ lo tienes</span> = ya cubres esa búsqueda · <span className="tag" style={{ color: 'var(--accent)' }}>⚡ oportunidad</span> = no tienes nada, conviene publicar.</p>
        </div>
        <select className="input" value={cat} onChange={(e) => onCat(e.target.value)} style={{ maxWidth: 280 }}>
          <option value="">Todas las categorías</option>
          {cats.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}
      {cargando && <div className="muted">Cargando…</div>}

      <div className="row" style={{ gap: 14, flexWrap: 'wrap', alignItems: 'flex-start' }}>
        <div className="card stack" style={{ flex: '1 1 300px', gap: 6 }}>
          <b>🔥 Lo que más crece</b>
          <span className="muted xs" style={{ marginTop: -2 }}>Búsquedas al alza esta semana</span>
          {crece.length === 0 ? <span className="muted sm">—</span> : crece.map((t, i) => (
            <a key={i} href={t.url} target="_blank" rel="noreferrer" className="row between" style={{ gap: 8, textDecoration: 'none', padding: '2px 0', alignItems: 'center' }}>
              <span className="sm"><b style={{ color: 'var(--accent)' }}>{i + 1}.</b> {t.keyword}</span>
              <Badge tengo={t.tengo} />
            </a>
          ))}
        </div>
        <div className="card stack" style={{ flex: '1 1 300px', gap: 6 }}>
          <b>🔎 Más buscado</b>
          <span className="muted xs" style={{ marginTop: -2 }}>Los términos más populares</span>
          {masBuscado.length === 0 ? <span className="muted sm">—</span> : masBuscado.map((t, i) => (
            <a key={i} href={t.url} target="_blank" rel="noreferrer" className="row between sm" style={{ gap: 8, textDecoration: 'none', padding: '1px 0', alignItems: 'center' }}>
              <span>{t.keyword}</span>
              <Badge tengo={t.tengo} />
            </a>
          ))}
        </div>
      </div>

      <div className="card stack" style={{ gap: 10 }}>
        <b>🏆 {data?.fuente_vendido === 'destacados' ? 'Destacados de la categoría' : 'Más vendido'}</b>
        {cat && data?.fuente_vendido === 'destacados' && <span className="muted xs" style={{ marginTop: -4 }}>Mercado Libre no habilita el ranking de "más vendidos" para apps externas, así que mostramos las publicaciones destacadas de la categoría.</span>}
        {!cat ? (
          <span className="muted sm">Elige una categoría arriba para ver sus productos más vendidos.</span>
        ) : vendido.length === 0 ? (
          <span className="muted sm">Sin datos de más vendidos para esta categoría.</span>
        ) : (
          <div className="row" style={{ gap: 12, flexWrap: 'wrap' }}>
            {vendido.map((p, i) => (
              <a key={p.id} href={p.permalink} target="_blank" rel="noreferrer" className="card" style={{ flex: '1 1 240px', textDecoration: 'none', display: 'flex', gap: 10, alignItems: 'center' }}>
                <b style={{ color: 'var(--accent)', minWidth: 22, textAlign: 'center' }}>{i + 1}</b>
                {p.thumbnail
                  ? <img src={p.thumbnail} alt="" width="46" height="46" style={{ borderRadius: 6, objectFit: 'cover', flexShrink: 0, background: 'var(--bg)' }} />
                  : <span style={{ width: 46, height: 46, flexShrink: 0 }} />}
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div className="sm" style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.titulo}</div>
                  <b className="sm">{money(p.precio)}</b>
                </div>
              </a>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
