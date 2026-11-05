import { useEffect, useState } from 'react'
import { NavLink, Navigate, Route, Routes } from 'react-router-dom'
import { get, getToken, setToken } from './api'
import Login from './pages/Login'
import Dashboard from './pages/Dashboard'
import Inventario from './pages/Inventario'
import Ganancias from './pages/Ganancias'
import ApiKeys from './pages/ApiKeys'
import Ventas from './pages/Ventas'
import Notificaciones from './pages/Notificaciones'
import Plan from './pages/Plan'
import Perfil from './pages/Perfil'

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
          <NavLink to="/inventario" className={cls}>Inventario</NavLink>
          <NavLink to="/ganancias" className={cls}>Ganancias</NavLink>
          <NavLink to="/ventas" className={cls}>Ventas</NavLink>
          <NavLink to="/api-keys" className={cls}>API keys</NavLink>
          <NavLink to="/notificaciones" className={cls}>Notificaciones</NavLink>
          <NavLink to="/plan" className={cls}>Plan</NavLink>
          <NavLink to="/perfil" className={cls}>Mi cuenta</NavLink>
        </nav>
        <div className="spacer" />
        {me && (me.plan !== 'business'
          ? <NavLink to="/plan" className="btn btn-sm btn-accent" style={{ marginRight: 10 }}>⭐ Mejorar plan</NavLink>
          : <NavLink to="/plan" className="muted sm" style={{ marginRight: 10, textDecoration: 'none' }}>Plan Business ✓</NavLink>)}
        {me && <span className="muted sm">{me.nombre}{me.rol === 'admin' ? ' · admin' : ''}</span>}
        <button className="btn btn-sm" onClick={logout}>Salir</button>
      </header>
      <main className="container">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/inventario" element={<Inventario />} />
          <Route path="/ganancias" element={<Ganancias />} />
          <Route path="/ventas" element={<Ventas />} />
          <Route path="/api-keys" element={<ApiKeys />} />
          <Route path="/notificaciones" element={<Notificaciones />} />
          <Route path="/plan" element={<Plan />} />
          <Route path="/perfil" element={<Perfil />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </>
  )
}
