import { useEffect, useState } from 'react'
import { NavLink, Navigate, Route, Routes } from 'react-router-dom'
import { get, getToken, setToken } from './api'
import Login from './pages/Login'
import Dashboard from './pages/Dashboard'
import Catalogo from './pages/Catalogo'

export default function App() {
  const [auth, setAuth] = useState(() => !!getToken())
  const [me, setMe] = useState(null)

  const logout = () => { setToken(null); setAuth(false); setMe(null) }

  useEffect(() => {
    if (auth) get('/auth/me').then(setMe).catch(logout)
  }, [auth])

  if (!auth) return <Login onAuth={() => setAuth(true)} />

  const cls = ({ isActive }) => (isActive ? 'on' : '')
  return (
    <>
      <header className="topbar">
        <div className="brand">Meli<b>Hub</b></div>
        <nav className="nav">
          <NavLink to="/" end className={cls}>Cuentas</NavLink>
          <NavLink to="/catalogo" className={cls}>Catálogo</NavLink>
        </nav>
        <div className="spacer" />
        {me && <span className="muted sm">{me.nombre}{me.rol === 'admin' ? ' · admin' : ''}</span>}
        <button className="btn btn-sm" onClick={logout}>Salir</button>
      </header>
      <main className="container">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/catalogo" element={<Catalogo />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </>
  )
}
