const BASE = (import.meta.env.VITE_API_URL || '') + '/api'

let token = null
try { token = localStorage.getItem('melihub_token') } catch { /* sin storage */ }

export function setToken(t) {
  token = t
  try { t ? localStorage.setItem('melihub_token', t) : localStorage.removeItem('melihub_token') } catch { /* */ }
}
export function getToken() { return token }

async function api(path, { method = 'GET', body } = {}) {
  const headers = { 'ngrok-skip-browser-warning': '1' }
  if (body !== undefined) headers['Content-Type'] = 'application/json'
  if (token) headers.Authorization = `Bearer ${token}`
  let res
  try {
    res = await fetch(BASE + path, { method, headers, body: body !== undefined ? JSON.stringify(body) : undefined })
  } catch {
    throw new Error('No se pudo conectar con el servidor.')
  }
  const text = await res.text()
  const data = text ? JSON.parse(text) : null
  if (!res.ok) {
    if (res.status === 401) setToken(null)
    throw new Error(data?.error || `Error ${res.status}`)
  }
  return data
}

export const get = (p) => api(p)
export const post = (p, b) => api(p, { method: 'POST', body: b ?? {} })
export const put = (p, b) => api(p, { method: 'PUT', body: b ?? {} })
export const del = (p) => api(p, { method: 'DELETE' })

export const money = (v, m = 'CLP') => new Intl.NumberFormat('es-CL', { style: 'currency', currency: m, maximumFractionDigits: 0 }).format(v || 0)

function saveBlob(blob, filename) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url; a.download = filename; a.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

/** Descarga un archivo del backend (con auth), p. ej. un CSV. */
export async function descargarCsv(path, filename) {
  const headers = { 'ngrok-skip-browser-warning': '1' }
  if (token) headers.Authorization = `Bearer ${token}`
  const res = await fetch(BASE + path, { headers })
  if (!res.ok) throw new Error(`Error ${res.status}`)
  saveBlob(await res.blob(), filename)
}

/** Descarga un archivo del backend usando el nombre que indica el servidor (PDF/ZIP/etc). */
export async function descargarArchivo(path, fallback = 'archivo') {
  const headers = { 'ngrok-skip-browser-warning': '1' }
  if (token) headers.Authorization = `Bearer ${token}`
  const res = await fetch(BASE + path, { headers })
  if (!res.ok) {
    const t = await res.text()
    let msg
    try { msg = JSON.parse(t)?.error } catch { /* no json */ }
    throw new Error(msg || `Error ${res.status}`)
  }
  const cd = res.headers.get('Content-Disposition') || ''
  const m = /filename="?([^"]+)"?/.exec(cd)
  saveBlob(await res.blob(), m ? m[1] : fallback)
}

/** Genera y descarga un CSV desde datos en el navegador. `rows` = array de arrays. */
export function csvLocal(filename, rows) {
  const esc = (v) => {
    const s = String(v ?? '')
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
  }
  const csv = rows.map((r) => r.map(esc).join(',')).join('\n')
  saveBlob(new Blob(['﻿' + csv], { type: 'text/csv;charset=utf-8' }), filename)
}
