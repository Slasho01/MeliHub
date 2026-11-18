import { useEffect, useRef } from 'react'

// Widget de Cloudflare Turnstile (CAPTCHA). Llama onToken(token) cuando se resuelve.
// Remonta (key) para resetear el token tras un intento fallido.
export default function Turnstile({ siteKey, onToken }) {
  const ref = useRef(null)
  const widgetId = useRef(null)

  useEffect(() => {
    if (!siteKey) return
    let cancelled = false
    const render = () => {
      if (cancelled || !window.turnstile || !ref.current) return
      try {
        widgetId.current = window.turnstile.render(ref.current, {
          sitekey: siteKey,
          callback: (t) => onToken(t),
          'error-callback': () => onToken(''),
          'expired-callback': () => onToken(''),
        })
      } catch { /* ya renderizado */ }
    }
    if (window.turnstile) {
      render()
    } else {
      const existing = document.getElementById('cf-turnstile-script')
      if (existing) {
        existing.addEventListener('load', render)
      } else {
        const s = document.createElement('script')
        s.id = 'cf-turnstile-script'
        s.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js'
        s.async = true
        s.defer = true
        s.onload = render
        document.head.appendChild(s)
      }
    }
    return () => {
      cancelled = true
      try { if (widgetId.current && window.turnstile) window.turnstile.remove(widgetId.current) } catch { /* ignore */ }
    }
  }, [siteKey])

  return <div ref={ref} style={{ display: 'flex', justifyContent: 'center', minHeight: 1 }} />
}
