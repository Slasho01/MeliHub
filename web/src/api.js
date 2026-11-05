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
