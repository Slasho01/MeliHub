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
import Facturacion from './pages/Facturacion'
import Suscripcion from './pages/Suscripcion'
import Auditoria from './pages/Auditoria'
import Envios from './pages/Envios'
import Preguntas from './pages/Preguntas'
import Reclamos from './pages/Reclamos'
import Catalogo from './pages/Catalogo'
import Cotizador from './pages/Cotizador'
import Conexiones from './pages/Conexiones'
import Dte from './pages/Dte'
import Reportes from './pages/Reportes'
import Resumen from './pages/Resumen'
import Correo from './pages/Correo'
import Despachos from './pages/Despachos'
import Devoluciones from './pages/Devoluciones'
import Falabella from './pages/Falabella'
import Paris from './pages/Paris'
import Landing from './pages/Landing'
import Docs from './pages/Docs'

export default function App() {
  const [auth, setAuth] = useState(() => !!getToken())
  const [me, setMe] = useState(null)
  const [menu, setMenu] = useState(() => typeof window !== 'undefined' && window.innerWidth > 900)

  const logout = () => { setToken(null); setAuth(false); setMe(null) }

  useEffect(() => {
    if (auth) get('/auth/me').then(setMe).catch(logout)
  }, [auth])

  if (!auth) return (
    <Routes>
      <Route path="/login" element={<Login onAuth={() => setAuth(true)} />} />
      <Route path="/reset" element={<Reset />} />
      <Route path="/aceptar" element={<Aceptar />} />
      <Route path="/legal" element={<Legal />} />
      <Route path="/docs" element={<Docs />} />
      <Route path="/" element={<Landing />} />
      <Route path="*" element={<Login onAuth={() => setAuth(true)} />} />
    </Routes>
  )

  const cls = ({ isActive }) => (isActive ? 'on' : '')
  const closeNavMobile = () => { if (typeof window !== 'undefined' && window.innerWidth <= 900) setMenu(false) }
  return (
    <div className={`app-shell ${menu ? 'nav-open' : ''}`}>
      <aside className="sidebar">
        <div className="sidebar-head">
          <div className="brand">Meli<b>Hub</b></div>
          <button className="side-toggle" aria-label="Ocultar menú" title="Ocultar menú" onClick={() => setMenu(false)}>«</button>
        </div>
        <nav className="sidebar-nav" onClick={closeNavMobile}>
          <div className="sidebar-section">Mercado Libre</div>
          <NavLink to="/" end className={cls}>Resumen</NavLink>
          <NavLink to="/cuentas" className={cls}>Cuentas</NavLink>
          <NavLink to="/conexiones" className={cls}>Conexiones</NavLink>
          <NavLink to="/ventas" className={cls}>Ventas</NavLink>
          <NavLink to="/reportes" className={cls}>Reportes</NavLink>
          <NavLink to="/envios" className={cls}>Envíos</NavLink>
          <NavLink to="/despachos" className={cls}>Despachos</NavLink>
          <NavLink to="/preguntas" className={cls}>Preguntas</NavLink>
          <NavLink to="/reclamos" className={cls}>Reclamos</NavLink>
          <NavLink to="/devoluciones" className={cls}>Devoluciones</NavLink>
          <NavLink to="/catalogo" className={cls}>Catálogo (Buy Box)</NavLink>
          <div className="sidebar-section">Falabella</div>
          <NavLink to="/falabella" className={cls}>Cuentas</NavLink>
          {me?.es_admin && <div className="sidebar-section">Paris</div>}
          {me?.es_admin && <NavLink to="/paris" className={cls}>Cuentas</NavLink>}
          <div className="sidebar-section">Catálogo</div>
          <NavLink to="/inventario" className={cls}>Inventario</NavLink>
          <NavLink to="/cotizador" className={cls}>Cotizador</NavLink>
          <NavLink to="/ganancias" className={cls}>Ganancias</NavLink>
          <div className="sidebar-section">Cuenta</div>
          {me?.es_owner && <NavLink to="/api-keys" className={cls}>API keys</NavLink>}
          {me?.es_owner && <NavLink to="/equipo" className={cls}>Equipo</NavLink>}
          {me?.es_owner && <NavLink to="/auditoria" className={cls}>Actividad</NavLink>}
          <NavLink to="/notificaciones" className={cls}>Notificaciones</NavLink>
          {me?.es_owner && <NavLink to="/plan" className={cls}>Plan</NavLink>}
          {me?.es_owner && <NavLink to="/suscripcion" className={cls}>Suscripción</NavLink>}
          {me?.es_owner && <NavLink to="/facturacion" className={cls}>Facturación</NavLink>}
          {me?.es_admin && <NavLink to="/dte" className={cls}>Facturación SII</NavLink>}
          <NavLink to="/perfil" className={cls}>Mi cuenta</NavLink>
          {me?.es_admin && <NavLink to="/correo" className={cls}>Correo</NavLink>}
          {me?.es_admin && <NavLink to="/admin" className={cls}>Admin</NavLink>}
        </nav>
        <div className="sidebar-foot">
          {me?.es_owner && (me.plan !== 'business'
            ? <NavLink to="/plan" className="btn btn-sm btn-accent" style={{ width: '100%' }}>⭐ Mejorar plan</NavLink>
            : <div className="muted sm" style={{ textAlign: 'center' }}>Plan Business ✓</div>)}
          {me && <div className="muted xs" style={{ textAlign: 'center' }}>{me.nombre}{me.es_admin ? ' · admin' : me.es_miembro ? ` · ${me.rol_equipo}` : ''}</div>}
          <button className="btn btn-sm" style={{ width: '100%' }} onClick={logout}>Salir</button>
        </div>
      </aside>
      <div className="sidebar-backdrop" onClick={() => setMenu(false)} />
      <div className="main">
        <header className="main-top">
          <button className="menu-btn" aria-label="Menú" aria-expanded={menu} onClick={() => setMenu((v) => !v)}>☰</button>
          <div className="brand">Meli<b>Hub</b></div>
        </header>
        <main className="container">
        <Routes>
          <Route path="/" element={<Resumen />} />
          <Route path="/cuentas" element={<Dashboard me={me} />} />
          <Route path="/conexiones" element={<Conexiones />} />
          <Route path="/inventario" element={<Inventario me={me} />} />
          <Route path="/cotizador" element={<Cotizador />} />
          <Route path="/ganancias" element={<Ganancias me={me} />} />
          <Route path="/ventas" element={<Ventas />} />
          <Route path="/reportes" element={<Reportes />} />
          <Route path="/envios" element={<Envios />} />
          <Route path="/despachos" element={<Despachos />} />
          <Route path="/preguntas" element={<Preguntas />} />
          <Route path="/reclamos" element={<Reclamos />} />
          <Route path="/devoluciones" element={<Devoluciones />} />
          <Route path="/catalogo" element={<Catalogo />} />
          <Route path="/falabella" element={<Falabella me={me} />} />
          <Route path="/paris" element={me && !me.es_admin ? <Navigate to="/" replace /> : <Paris me={me} />} />
          <Route path="/api-keys" element={me && !me.es_owner ? <Navigate to="/" replace /> : <ApiKeys />} />
          <Route path="/equipo" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Equipo />} />
          <Route path="/auditoria" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Auditoria />} />
          <Route path="/notificaciones" element={<Notificaciones />} />
          <Route path="/plan" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Plan />} />
          <Route path="/suscripcion" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Suscripcion />} />
          <Route path="/facturacion" element={me && !me.es_owner ? <Navigate to="/" replace /> : <Facturacion />} />
          <Route path="/dte" element={me && !me.es_admin ? <Navigate to="/" replace /> : <Dte />} />
          <Route path="/correo" element={me && !me.es_admin ? <Navigate to="/" replace /> : <Correo />} />
          <Route path="/perfil" element={<Perfil />} />
          <Route path="/admin" element={me && !me.es_admin ? <Navigate to="/" replace /> : <Admin />} />
          <Route path="/legal" element={<Legal />} />
          <Route path="/docs" element={<Docs />} />
          <Route path="/aceptar" element={<Aceptar />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
        </main>
      </div>
    </div>
  )
}
