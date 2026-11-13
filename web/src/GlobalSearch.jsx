import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { get, money } from './api'

export default function GlobalSearch() {
  const [q, setQ] = useState('')
  const [res, setRes] = useState([])
  const [open, setOpen] = useState(false)
  const nav = useNavigate()
  const ref = useRef(null)

  useEffect(() => {
    const t = setTimeout(() => {
      const term = q.trim()
      if (term.length < 2) { setRes([]); return }
      get(`/productos?q=${encodeURIComponent(term)}`).then((d) => setRes((Array.isArray(d) ? d : (d?.data || [])).slice(0, 7))).catch(() => {})
    }, 250)
    return () => clearTimeout(t)
  }, [q])

  const ir = (term) => { const t = String(term || '').trim(); if (t) { nav(`/inventario?q=${encodeURIComponent(t)}`); setOpen(false); setQ('') } }

  return (
    <div ref={ref} style={{ position: 'relative', flex: 1, maxWidth: 420, minWidth: 0 }}>
      <input className="input input-sm" placeholder="Buscar productos…" value={q}
        onChange={(e) => { setQ(e.target.value); setOpen(true) }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 160)}
        onKeyDown={(e) => { if (e.key === 'Enter') ir(q) }} />
      {open && res.length > 0 && (
        <div style={{ position: 'absolute', zIndex: 30, left: 0, right: 0, top: '100%', marginTop: 4, background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 10, boxShadow: '0 12px 28px rgba(0,0,0,.4)', overflow: 'hidden' }}>
          {res.map((p) => (
            <div key={p.id} onMouseDown={(e) => { e.preventDefault(); ir(p.titulo || p.ml_item_id) }}
              style={{ padding: '8px 12px', display: 'flex', gap: 8, alignItems: 'center', cursor: 'pointer', borderBottom: '1px solid var(--line)' }}>
              {p.thumbnail && <img src={p.thumbnail} alt="" width="28" height="28" style={{ borderRadius: 5, objectFit: 'cover' }} />}
              <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 13 }}>{p.titulo}</span>
              <span className="muted" style={{ fontSize: 12, whiteSpace: 'nowrap' }}>{p.stock ?? '—'} u · {money(p.precio)}</span>
            </div>
          ))}
          <div onMouseDown={(e) => { e.preventDefault(); ir(q) }} style={{ padding: '7px 12px', fontSize: 12, cursor: 'pointer', color: 'var(--accent)' }}>Ver todo en Inventario →</div>
        </div>
      )}
    </div>
  )
}
