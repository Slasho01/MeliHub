import { useState } from 'react'

// Lista desplegable con búsqueda: escribes y filtra las opciones. options: [{value, label}].
export default function Combobox({ options, value, onChange, placeholder }) {
  const [q, setQ] = useState('')
  const [open, setOpen] = useState(false)
  const sel = options.find((o) => o.value === value)
  const text = open ? q : (sel ? sel.label : '')
  const filtered = options.filter((o) => (o.label || '').toLowerCase().includes(q.toLowerCase())).slice(0, 80)
  return (
    <div style={{ position: 'relative' }}>
      <input
        className="input"
        placeholder={placeholder}
        value={text}
        onChange={(e) => { setQ(e.target.value); setOpen(true) }}
        onFocus={() => { setQ(''); setOpen(true) }}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && filtered.length > 0 && (
        <div style={{ position: 'absolute', zIndex: 20, left: 0, right: 0, top: '100%', marginTop: 2, maxHeight: 240, overflowY: 'auto', background: 'var(--surface)', color: 'var(--ink)', border: '1px solid var(--line)', borderRadius: 8, boxShadow: '0 10px 24px rgba(0,0,0,.45)' }}>
          {filtered.map((o) => (
            <div
              key={o.value}
              style={{ padding: '8px 10px', cursor: 'pointer', fontSize: 14, color: 'var(--ink)', borderBottom: '1px solid var(--line)', background: o.value === value ? 'var(--line)' : 'transparent' }}
              onMouseDown={(e) => { e.preventDefault(); onChange(o.value); setOpen(false); setQ('') }}
            >
              {o.label}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
