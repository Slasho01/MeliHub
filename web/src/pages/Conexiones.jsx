import { useEffect, useState } from 'react'
import { get } from '../api'

const estadoInfo = (s) => ({
  ok: ['Operativa', 'var(--ok)'],
  atrasada: ['Sincronización atrasada', 'var(--warn)'],
  pendiente: ['Sin sincronizar', 'var(--warn)'],
  desconectada: ['Desconectada', 'var(--bad, #d93025)'],
}[s] || [s || '—', 'var(--muted)'])

const tipoColor = (t) => ({ 'Mercado Libre': '#ffe600', Falabella: '#78be20', Paris: '#2a6f97', Shopify: '#96bf48' }[t] || 'var(--muted)')

export default function Conexiones() {
  const [rows, setRows] = useState(null)
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)

  const cargar = () => { setCargando(true); setError(''); get('/salud/conexiones').then(setRows).catch((e) => setError(e.message)).finally(() => setCargando(false)) }
  useEffect(cargar, [])

  const lista = rows || []
  const ok = lista.filter((r) => r.estado === 'ok').length
  const warn = lista.filter((r) => r.estado === 'atrasada' || r.estado === 'pendiente').length
  const bad = lista.filter((r) => r.estado === 'desconectada').length

  return (
    <div className="stack">
      <div className="row between" style={{ flexWrap: 'wrap', gap: 10, alignItems: 'flex-end' }}>
        <div>
          <h1 style={{ margin: 0 }}>Conexiones</h1>
          <p className="muted sm" style={{ margin: 0 }}>Estado de todas tus integraciones. Detecta a tiempo si algo dejó de sincronizar o se desconectó.</p>
        </div>
        <button className="btn btn-sm" onClick={cargar} disabled={cargando}>↻ Actualizar</button>
      </div>

      {error && <div className="alert alert-bad">{error}</div>}

      {rows == null ? (
        <div className="muted">Cargando…</div>
      ) : lista.length === 0 ? (
        <div className="empty">Aún no tienes integraciones conectadas. Conecta Mercado Libre, Falabella o Paris para verlas aquí.</div>
      ) : (
        <>
          <div className="row" style={{ gap: 10, flexWrap: 'wrap' }}>
            <div className="card" style={{ flex: '1 1 140px', borderLeft: '4px solid var(--ok)' }}><div className="muted sm">Operativas</div><div style={{ fontSize: 26, fontWeight: 800, color: 'var(--ok)' }}>{ok}</div></div>
            <div className="card" style={{ flex: '1 1 140px', borderLeft: '4px solid var(--warn)' }}><div className="muted sm">Con atención</div><div style={{ fontSize: 26, fontWeight: 800, color: 'var(--warn)' }}>{warn}</div></div>
            <div className="card" style={{ flex: '1 1 140px', borderLeft: '4px solid var(--bad, #d93025)' }}><div className="muted sm">Desconectadas</div><div style={{ fontSize: 26, fontWeight: 800, color: 'var(--bad, #d93025)' }}>{bad}</div></div>
          </div>

          {bad + warn > 0 && (
            <div className="alert" style={{ borderLeft: '3px solid var(--warn)' }}>
              {bad > 0 && <>Tienes <b>{bad}</b> conexión(es) desconectada(s) que debes reconectar. </>}
              {warn > 0 && <>Hay <b>{warn}</b> con sincronización pendiente o atrasada — revisa que la cuenta siga activa.</>}
            </div>
          )}

          <div className="row" style={{ gap: 10, flexWrap: 'wrap', alignItems: 'stretch' }}>
            {lista.map((r, i) => {
              const [txt, color] = estadoInfo(r.estado)
              return (
                <div key={i} className="card stack" style={{ flex: '1 1 260px', gap: 8, borderLeft: `4px solid ${color}` }}>
                  <div className="row between" style={{ alignItems: 'center' }}>
                    <span className="badge" style={{ background: 'var(--surface-2)', borderLeft: `3px solid ${tipoColor(r.tipo)}`, paddingLeft: 8 }}>{r.tipo}</span>
                    <span className="row" style={{ gap: 6, alignItems: 'center' }}>
                      <span style={{ width: 9, height: 9, borderRadius: '50%', background: color, display: 'inline-block' }} />
                      <span className="sm" style={{ color, fontWeight: 600 }}>{txt}</span>
                    </span>
                  </div>
                  <div className="strong" style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.nombre}</div>
                  <div className="muted sm">{r.detalle}</div>
                  {r.productos != null && <div className="muted xs">{r.productos} producto(s) sincronizados</div>}
                </div>
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}
