import { useState } from 'react'
import { Link } from 'react-router-dom'

const clp = (n) => '$' + n.toLocaleString('es-CL')

const FEATURES = [
  { ic: '🔗', t: 'Todas tus cuentas juntas', d: 'Conecta cuantas cuentas de Mercado Libre quieras con OAuth y gestiónalas desde un solo panel.' },
  { ic: '📦', t: 'Inventario unificado', d: 'Un catálogo central con el stock, precio y estado de cada publicación de todas tus cuentas.' },
  { ic: '🔄', t: 'Stock en dos vías', d: 'Vendes en tu tienda o en otro canal y el stock baja solo en Mercado Libre. Sin descuadres.' },
  { ic: '💰', t: 'Ventas y ganancias', d: 'Mira las ventas del día de todas las cuentas, con comisiones y ganancia neta ya calculada.' },
  { ic: '🏷️', t: 'Etiquetas de envío', d: 'Descarga juntas las etiquetas de Flex y Centro de envío (colecta) de todas tus cuentas, por día.' },
  { ic: '🧩', t: 'Distribuye tu catálogo', d: 'Lleva tus productos a tu tienda web, WooCommerce y marketplaces con Mercado Libre como fuente única.' },
  { ic: '🏬', t: 'Publica en Falabella', d: 'Conecta Falabella Seller y lleva tu catálogo de ML con precios, stock, imágenes y órdenes sincronizados. (Pro y Business)' },
]

const EXTRAS = [
  { ic: '👥', t: 'Equipo con roles', d: 'Suma operadores con permisos de operación o solo lectura. Revoca el acceso al instante.' },
  { ic: '🔐', t: 'Seguridad', d: 'Verificación en dos pasos (2FA), tokens cifrados y registro de actividad de tu equipo.' },
  { ic: '⚡', t: 'Siempre al día', d: 'Sincroniza publicaciones, precios y ventas con un clic. El panel refleja el estado real en ML.' },
]

const STEPS = [
  { n: 1, t: 'Conecta', d: 'Vincula tus cuentas de Mercado Libre en segundos con OAuth seguro.' },
  { n: 2, t: 'Centraliza', d: 'Catálogo, stock, ventas y envíos de todas las cuentas en un solo lugar.' },
  { n: 3, t: 'Distribuye', d: 'Vende en más canales y mantén el stock sincronizado automáticamente.' },
]

function Precio({ p, destacado, bullets }) {
  return (
    <div className={`lp-price ${destacado ? 'featured' : ''}`}>
      {destacado && <div className="lp-price-tag">Más elegido</div>}
      <h3>{p.nombre}</h3>
      <div className="lp-price-num">
        {p.precio_mensual === 0 ? 'Gratis' : <>{clp(p.precio_mensual)}<span>/mes</span></>}
      </div>
      <div className="muted sm" style={{ minHeight: 18 }}>
        {p.precio_anual > 0 ? `${clp(p.precio_anual)} al año (2 meses gratis)` : 'Para siempre'}
      </div>
      <ul className="lp-price-list">{bullets.map((b, i) => <li key={i}>{b}</li>)}</ul>
      <Link to="/login?r=1" className={`btn ${destacado ? 'btn-accent' : ''}`} style={{ width: '100%', textAlign: 'center' }}>
        {p.precio_mensual === 0 ? 'Empezar gratis' : 'Probar 3 días'}
      </Link>
    </div>
  )
}

export default function Landing() {
  const [menu, setMenu] = useState(false)
  const planes = [
    { p: { nombre: 'Free', precio_mensual: 0, precio_anual: 0 }, bullets: ['1 cuenta de Mercado Libre', 'Hasta 50 productos', '1 canal de distribución', 'Sin marketplaces (Falabella en Pro)'] },
    { p: { nombre: 'Pro', precio_mensual: 9990, precio_anual: 99900 }, destacado: true, bullets: ['3 cuentas de Mercado Libre', 'Hasta 1.000 productos', '3 canales de distribución', 'Falabella y marketplaces', 'Equipo y roles'] },
    { p: { nombre: 'Business', precio_mensual: 29990, precio_anual: 299900 }, bullets: ['Cuentas ilimitadas', 'Productos ilimitados', 'Canales ilimitados', 'Falabella y marketplaces', 'Soporte prioritario'] },
  ]

  return (
    <div className="lp">
      <header className="lp-nav">
        <div className="brand">Meli<b>Hub</b></div>
        <button className="nav-toggle" aria-label="Menú" aria-expanded={menu} onClick={() => setMenu((v) => !v)}>☰</button>
        <nav className={`lp-nav-links ${menu ? 'open' : ''}`} onClick={() => setMenu(false)}>
          <a href="#features">Características</a>
          <a href="#como">Cómo funciona</a>
          <a href="#precios">Precios</a>
          <Link to="/docs">API</Link>
          <Link to="/login" className="btn btn-sm">Entrar</Link>
          <Link to="/login?r=1" className="btn btn-sm btn-accent">Crear cuenta</Link>
        </nav>
      </header>

      <section className="lp-hero">
        <div className="lp-badge">Hub multi-cuenta de Mercado Libre</div>
        <h1 className="lp-h1">Todas tus cuentas de<br /><span className="hl">Mercado Libre</span>, en un solo panel.</h1>
        <p className="lp-sub">
          MeliHub centraliza el inventario, las ventas y los envíos de todas tus cuentas de Mercado Libre,
          y te deja distribuir tu catálogo a tu tienda, WooCommerce y marketplaces como Falabella. Sincroniza
          el stock en ambos sentidos, descarga etiquetas y gestiona todo desde un mismo lugar.
        </p>
        <div className="lp-cta">
          <Link to="/login?r=1" className="btn btn-accent btn-lg">Empezar gratis</Link>
          <a href="#precios" className="btn btn-lg">Ver planes</a>
        </div>
        <p className="muted sm" style={{ marginTop: 10 }}>3 días de prueba · Sin tarjeta para empezar</p>

        <div className="lp-mock">
          <div className="lp-mock-bar"><span /><span /><span /><em className="muted sm">Panel MeliHub</em></div>
          <div className="lp-mock-body">
            <div className="lp-mock-row"><b>Inventario</b><span className="tag tag-ok">Sincronizado</span><i className="muted sm">1.342 productos</i></div>
            <div className="lp-mock-row"><b>Ventas de hoy</b><span className="tag">18 pedidos</span><i className="muted sm">Ganancia $284.900</i></div>
            <div className="lp-mock-row"><b>Envíos</b><span className="tag tag-ok">7 por imprimir</span><i className="muted sm">Flex + colecta</i></div>
            <div className="lp-mock-row"><b>Canales</b><span className="tag">Tienda + WooCommerce + Falabella</span><i className="muted sm">Stock en 2 vías</i></div>
          </div>
        </div>
      </section>

      <section id="features" className="lp-section">
        <h2 className="lp-h2">Todo tu Mercado Libre, ordenado</h2>
        <p className="lp-lead">Deja de saltar entre cuentas y pestañas. MeliHub reúne lo que importa en una sola vista.</p>
        <div className="lp-grid">
          {FEATURES.map((f, i) => (
            <div key={i} className="lp-card">
              <div className="lp-ic">{f.ic}</div>
              <h3>{f.t}</h3>
              <p className="muted">{f.d}</p>
            </div>
          ))}
        </div>
        <div className="lp-grid lp-grid-3" style={{ marginTop: 16 }}>
          {EXTRAS.map((f, i) => (
            <div key={i} className="lp-card lp-card-soft">
              <div className="lp-ic sm">{f.ic}</div>
              <h3>{f.t}</h3>
              <p className="muted sm">{f.d}</p>
            </div>
          ))}
        </div>
      </section>

      <section id="como" className="lp-section lp-section-alt">
        <h2 className="lp-h2">Cómo funciona</h2>
        <div className="lp-steps">
          {STEPS.map((s) => (
            <div key={s.n} className="lp-step">
              <div className="lp-step-n">{s.n}</div>
              <h3>{s.t}</h3>
              <p className="muted">{s.d}</p>
            </div>
          ))}
        </div>
      </section>

      <section id="precios" className="lp-section">
        <h2 className="lp-h2">Planes simples</h2>
        <p className="lp-lead">Empieza gratis y crece cuando lo necesites. Todos los planes pagados incluyen 3 días de prueba.</p>
        <div className="lp-pricing">
          {planes.map((x, i) => <Precio key={i} {...x} />)}
        </div>
      </section>

      <section className="lp-final">
        <h2 className="lp-h2">Centraliza tu Mercado Libre hoy</h2>
        <p className="lp-lead">Conecta tu primera cuenta en menos de un minuto.</p>
        <Link to="/login?r=1" className="btn btn-accent btn-lg">Crear mi cuenta gratis</Link>
      </section>

      <footer className="lp-footer">
        <div className="brand">Meli<b>Hub</b></div>
        <div className="row" style={{ gap: 16, flexWrap: 'wrap' }}>
          <Link to="/docs" className="muted">Documentación API</Link>
          <Link to="/legal" className="muted">Términos y privacidad</Link>
          <Link to="/login" className="muted">Entrar</Link>
        </div>
        <span className="muted sm">© {new Date().getFullYear()} MeliHub · No afiliado a Mercado Libre.</span>
      </footer>
    </div>
  )
}
