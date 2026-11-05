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
import Admin from './pages/Admin'
import Reset from './pages/Reset'
import Legal from './pages/Legal'
import Equipo from './pages/Equipo'
import Aceptar from './pages/Aceptar'

export default function App() {
  const [auth, setAuth] = useState(() => !!getToken())
  const [me, setMe] = useState(null)

  const logout = () => { setToken(null); setAuth(false); setMe(null) }

  useEffect(() => {
    if (auth) get('/auth/me').then(setMe).catch(logout)
  }, [auth])

  if (!auth) return (
    <Routes>
      <Route path="/reset" element={<Reset />} />
      <Route path="/aceptar" element={<Aceptar />} />
      <Route path="/legal" element={<Legal />} />
      <Route path="*" element={<Login onAuth={() => setAuth(true)} />} />
    </Routes>
  )

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
          {me?.es_owner && <NavLink to="/api-keys" className={cls}>API keys</NavLink>}
          {me?.es_owner && <NavLink to="/equipo" className={cls}>Equipo</NavLink>}
          <NavLink to="/notificaciones" className={cls}>Notificaciones</NavLink>
          {me?.es_owner && <NavLink to="/plan" className={cls}>Plan</NavLink>}
          <NavLink to="/perfil" className={cls}>Mi cuenta</NavLink>
          {me?.es_admin && <NavLink to="/admin" className={cls}>Admin</NavLink>}
        </nav>
        <div className="spacer" />
        {me?.es_owner && (me.plan !== 'business'
          ? <NavLink to="/plan" className="btn btn-sm btn-accent" style={{ marginRight: 10 }}>⭐ Mejorar plan</NavLink>
          : <NavLink to="/plan" className="muted sm" style={{ marginRight: 10, textDecoration: 'none' }}>Plan Business ✓</NavLink>)}
        {me && <span className="muted sm">{me.nombre}{me.es_admin ? ' · admin' : me.es_miembro ? ` · ${me.rol_equipo}` : ''}</span>}
        <button className="btn btn-sm" onClick={logout}>Salir</button>
      </header>
      <main className="container">
        <Routes>
          <Route path="/" element={<Dashboard me={me} />} />
          <Route path="/inventario" element={<Inventario me={me} />} />
          <Route path="/ganancias" element={<Ganancias me={me} />} />
          <Route path="/ventas" element={<Ventas />} />
          <Route path="/api-keys" element={me && !me.es_owner ? <Navigate to="/" replace /> : <ApiKeys />} />
          <Route path="/equipo" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Equipo />} />
          <Route path="/notificaciones" element={<Notificaciones />} />
          <Route path="/plan" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Plan />} />
          <Route path="/perfil" element={<Perfil />} />
          <Route path="/admin" element={me && !me.es_admin ? <Navigate to="/" replace /> : <Admin />} />
          <Route path="/legal" element={<Legal />} />
          <Route path="/aceptar" element={<Aceptar />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </>
  )
}
