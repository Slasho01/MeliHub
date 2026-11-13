import { useState } from 'react'
import { Link } from 'react-router-dom'
import ThemeToggle from '../ThemeToggle'

const clp = (n) => '$' + n.toLocaleString('es-CL')

const CANALES = ['Mercado Libre', 'Falabella', 'Shopify', 'WooCommerce', 'Tu tienda']

const FEATURES = [
  { ic: '🔗', t: 'Todas tus cuentas juntas', d: 'Conecta cuantas cuentas de Mercado Libre quieras con OAuth y gestiónalas desde un solo panel.' },
  { ic: '📦', t: 'Inventario unificado', d: 'Un catálogo central con el stock, precio y estado de cada publicación de todas tus cuentas.' },
  { ic: '🔄', t: 'Stock en dos vías', d: 'Vendes en cualquier canal y el stock baja solo en Mercado Libre y el resto. Sin sobreventa ni descuadres.' },
  { ic: '💰', t: 'Ganancia real', d: 'Ventas del día con la ganancia neta ya calculada: precio − comisión de ML − tu costo.' },
  { ic: '⚡', t: 'Repricing automático', d: 'Mantén tus publicaciones de catálogo ganando la Buy Box, respetando siempre tu margen mínimo.' },
  { ic: '🚚', t: 'Despachos unificados', d: 'Todos los pedidos por despachar de tus canales en una vista, con descarga de etiquetas.' },
  { ic: '🏬', t: 'Multicanal de verdad', d: 'Lleva tu catálogo de ML a Falabella, Shopify y tu tienda (WooCommerce) con precio y stock sincronizados.' },
  { ic: '📊', t: 'Reportes y dashboard', d: 'Rentabilidad por cuenta, productos más vendidos y evolución de ventas. Exporta el reporte en PDF.' },
]

const EXTRAS = [
  { ic: '💬', t: 'Post-venta completo', d: 'Preguntas, reclamos y devoluciones de todas tus cuentas, centralizados y con respuesta rápida.' },
  { ic: '🏷️', t: 'Catálogo y promociones', d: 'Compite en la Buy Box y suma tus productos a las ofertas y campañas de Mercado Libre.' },
  { ic: '🧾', t: 'Cotizador con PDF', d: 'Arma cotizaciones con productos de tu inventario y descárgalas en PDF con el logo de tu empresa.' },
  { ic: '👥', t: 'Equipo con roles', d: 'Suma operadores con permisos de operación o solo lectura. Revoca el acceso al instante.' },
  { ic: '🔐', t: 'Seguridad', d: 'Verificación en dos pasos (2FA), tokens cifrados y registro de actividad de tu equipo.' },
  { ic: '🩺', t: 'Salud y alertas', d: 'Panel de conexiones que avisa si algo dejó de sincronizar, y alertas de stock bajo configurables.' },
]

const STEPS = [
  { n: 1, t: 'Conecta', d: 'Vincula tus cuentas de Mercado Libre y tus canales en segundos, de forma segura.' },
  { n: 2, t: 'Centraliza', d: 'Catálogo, stock, ventas, post-venta y despachos de todo, en un solo lugar.' },
  { n: 3, t: 'Vende más', d: 'Distribuye a más canales, gana la Buy Box y mantén el stock sincronizado solo.' },
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
    { p: { nombre: 'Free', precio_mensual: 0, precio_anual: 0 }, bullets: ['1 cuenta de Mercado Libre', 'Hasta 50 productos', '1 canal de distribución', 'Sin otros marketplaces'] },
    { p: { nombre: 'Pro', precio_mensual: 9990, precio_anual: 99900 }, destacado: true, bullets: ['3 cuentas de Mercado Libre', 'Hasta 1.000 productos', 'Falabella, Shopify y tu tienda', 'Repricing y promociones', 'Equipo y roles'] },
    { p: { nombre: 'Business', precio_mensual: 29990, precio_anual: 299900 }, bullets: ['Cuentas ilimitadas', 'Productos ilimitados', 'Todos los canales', 'Reportes y post-venta', 'Soporte prioritario'] },
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
          <ThemeToggle className="btn btn-sm" />
          <Link to="/login" className="btn btn-sm">Entrar</Link>
          <Link to="/login?r=1" className="btn btn-sm btn-accent">Crear cuenta</Link>
        </nav>
      </header>

      <section className="lp-hero">
        <div className="lp-badge">Hub multicanal para Mercado Libre y tu e-commerce</div>
        <h1 className="lp-h1">Vende en todos lados,<br />controla todo desde <span className="hl">un panel</span>.</h1>
        <p className="lp-sub">
          MeliHub centraliza el inventario, las ventas, el post-venta y los despachos de todas tus cuentas de
          Mercado Libre, y distribuye tu catálogo a <b>Falabella, Shopify y tu tienda</b> (WooCommerce) con el
          stock sincronizado en ambos sentidos. Repricing de Buy Box, reportes y cotizador, todo en un mismo lugar.
        </p>
        <div className="lp-cta">
          <Link to="/login?r=1" className="btn btn-accent btn-lg">Empezar gratis</Link>
          <a href="#precios" className="btn btn-lg">Ver planes</a>
        </div>
        <p className="muted sm" style={{ marginTop: 10 }}>3 días de prueba · Sin tarjeta para empezar</p>

        <div className="row" style={{ justifyContent: 'center', gap: 8, flexWrap: 'wrap', marginTop: 22 }}>
          <span className="muted sm">Vende en:</span>
          {CANALES.map((c) => (
            <span key={c} style={{ border: '1px solid var(--line)', background: 'var(--surface)', borderRadius: 999, padding: '4px 12px', fontSize: 13, fontWeight: 600 }}>{c}</span>
          ))}
        </div>

        <div className="lp-mock">
          <div className="lp-mock-bar"><span /><span /><span /><em className="muted sm">Panel MeliHub</em></div>
          <div className="lp-mock-body">
            <div className="lp-mock-row"><b>Inventario</b><span className="tag tag-ok">Sincronizado</span><i className="muted sm">1.342 productos</i></div>
            <div className="lp-mock-row"><b>Ventas de hoy</b><span className="tag">18 pedidos</span><i className="muted sm">Ganancia $284.900</i></div>
            <div className="lp-mock-row"><b>Buy Box</b><span className="tag tag-ok">Ganando</span><i className="muted sm">Repricing activo</i></div>
            <div className="lp-mock-row"><b>Canales</b><span className="tag">ML · Falabella · Shopify · Tu tienda</span><i className="muted sm">Stock en 2 vías</i></div>
          </div>
        </div>
      </section>

      <section id="features" className="lp-section">
        <h2 className="lp-h2">Todo tu e-commerce, ordenado</h2>
        <p className="lp-lead">Deja de saltar entre cuentas, canales y pestañas. MeliHub reúne lo que importa en una sola vista.</p>
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
        <h2 className="lp-h2">Centraliza tu e-commerce hoy</h2>
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
