export function passwordOk(p) {
  return p.length >= 8 && /[A-Za-z]/.test(p) && /\d/.test(p)
}

export function scorePassword(p) {
  if (!p) return 0
  let s = 0
  if (p.length >= 8) s++
  if (p.length >= 12) s++
  if (/[a-z]/.test(p) && /[A-Z]/.test(p)) s++
  if (/\d/.test(p)) s++
  if (/[^A-Za-z0-9]/.test(p)) s++
  return Math.min(4, s)
}

const LABELS = ['Muy débil', 'Débil', 'Media', 'Fuerte', 'Muy fuerte']
const COLORS = ['var(--bad)', 'var(--bad)', 'var(--warn)', 'var(--ok)', 'var(--ok)']

export default function PasswordMeter({ value }) {
  if (!value) return null
  const s = scorePassword(value)
  const ok = passwordOk(value)
  return (
    <div style={{ marginTop: 4 }}>
      <div style={{ height: 5, background: 'var(--line)', borderRadius: 4 }}>
        <div style={{ height: '100%', width: `${Math.max(10, (s / 4) * 100)}%`, background: COLORS[s], borderRadius: 4, transition: 'width .2s' }} />
      </div>
      <div className="sm" style={{ color: ok ? 'var(--muted)' : 'var(--bad)', marginTop: 3 }}>
        {LABELS[s]}{!ok && ' · mínimo 8 caracteres, con letras y números'}
      </div>
    </div>
  )
}
