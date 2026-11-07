import { Link } from 'react-router-dom'

const Code = ({ children }) => <pre className="doc-pre"><code>{children}</code></pre>

function Endpoint({ metodo, ruta, children }) {
  const cls = metodo === 'GET' ? 'get' : 'post'
  return (
    <div className="doc-ep">
      <div className="doc-ep-head"><span className={`doc-m ${cls}`}>{metodo}</span><code>{ruta}</code></div>
      {children}
    </div>
  )
}

export default function Docs() {
  const base = typeof window !== 'undefined' ? window.location.origin : 'https://tu-melihub.com'
  return (
    <div className="lp">
      <header className="lp-nav">
        <Link to="/" className="brand" style={{ textDecoration: 'none' }}>Meli<b>Hub</b></Link>
        <nav className="lp-nav-links">
          <Link to="/" className="muted">Inicio</Link>
          <Link to="/login" className="btn btn-sm btn-accent">Entrar</Link>
        </nav>
      </header>

      <div className="doc">
        <h1 className="lp-h2" style={{ textAlign: 'left' }}>Documentación de la API</h1>
        <p className="muted" style={{ marginTop: 8 }}>
          MeliHub expone una API simple para que tu tienda o cualquier plataforma lea tu catálogo
          de Mercado Libre y reporte ventas y devoluciones, manteniendo el stock sincronizado en
          ambos sentidos. Si usas WordPress, el <a href="#wp">plugin oficial</a> hace todo esto por ti.
        </p>

        <section className="card doc-card">
          <h2>1. Autenticación</h2>
          <p className="muted">
            Cada petición se autentica con una <b>API key</b> en la cabecera <code>X-API-Key</code>.
            Creas tus keys en el panel, en <b>API keys</b> (menú del dueño). Cada key pertenece a tu
            cuenta y puede limitarse a una cuenta de Mercado Libre y a un <b>canal</b> (p. ej. una
            lista de precios distinta por plataforma).
          </p>
          <Code>{`X-API-Key: mh_tu_clave_secreta`}</Code>
        </section>

        <section className="card doc-card">
          <h2>2. URL base</h2>
          <p className="muted">Todas las rutas cuelgan de <code>/api/pub</code> en tu dominio de MeliHub:</p>
          <Code>{`${base}/api/pub`}</Code>
        </section>

        <section className="card doc-card">
          <h2>3. Endpoints</h2>

          <Endpoint metodo="GET" ruta="/api/pub/productos">
            <p className="muted">Devuelve tu catálogo (todas las publicaciones activas de las cuentas que la key puede ver).</p>
            <Code>{`curl ${base}/api/pub/productos \\
  -H "X-API-Key: mh_tu_clave_secreta"`}</Code>
            <p className="muted sm">Respuesta (array de productos):</p>
            <Code>{`[
  {
    "id": "7b4537eb-fca6-475a-92f5-bd75f61fb75f",
    "ml_item_id": "MLC1234567890",
    "titulo": "Adaptador USB-C",
    "precio": 9990,
    "moneda": "CLP",
    "stock": 25,
    "estado": "active",
    "thumbnail": "https://http2.mlstatic.com/...jpg",
    "canal": "mitienda",
    "descripcion": "…",
    "especificaciones": [{ "clave": "Marca", "valor": "Genérico" }],
    "categoria": "Accesorios"
  }
]`}</Code>
          </Endpoint>

          <Endpoint metodo="POST" ruta="/api/pub/ventas">
            <p className="muted">Reporta una venta: <b>descuenta</b> el stock en MeliHub y lo empuja a Mercado Libre.</p>
            <Code>{`curl -X POST ${base}/api/pub/ventas \\
  -H "X-API-Key: mh_tu_clave_secreta" \\
  -H "Content-Type: application/json" \\
  -d '{ "producto_id": "7b4537eb-…", "cantidad": 2 }'`}</Code>
            <p className="muted sm">Respuesta:</p>
            <Code>{`{ "producto_id": "7b4537eb-…", "nuevo_stock": 23, "ml_actualizado": true }`}</Code>
          </Endpoint>

          <Endpoint metodo="POST" ruta="/api/pub/devoluciones">
            <p className="muted">Devuelve stock (cancelación o reembolso): <b>suma</b> el stock y lo empuja de vuelta a Mercado Libre.</p>
            <Code>{`curl -X POST ${base}/api/pub/devoluciones \\
  -H "X-API-Key: mh_tu_clave_secreta" \\
  -H "Content-Type: application/json" \\
  -d '{ "producto_id": "7b4537eb-…", "cantidad": 2 }'`}</Code>
            <p className="muted sm">Respuesta:</p>
            <Code>{`{ "producto_id": "7b4537eb-…", "nuevo_stock": 25, "ml_actualizado": true }`}</Code>
          </Endpoint>

          <p className="muted sm" style={{ marginTop: 14 }}>
            <b>Notas.</b> <code>producto_id</code> es el <code>id</code> que entrega <code>GET /productos</code>
            (no el <code>ml_item_id</code>). <code>cantidad</code> debe ser mayor que 0. Si la publicación ya
            no está conectada a una cuenta de ML activa, el stock se ajusta igual en MeliHub y
            <code>ml_actualizado</code> vuelve <code>false</code>.
          </p>
        </section>

        <section className="card doc-card">
          <h2>4. Errores</h2>
          <p className="muted">Respuestas con código HTTP estándar y un cuerpo JSON <code>{'{ "error": "…" }'}</code>:</p>
          <ul className="doc-list">
            <li><code>401</code> — falta o no coincide la <code>X-API-Key</code>.</li>
            <li><code>403</code> — el producto no pertenece a esta key.</li>
            <li><code>404</code> — producto no encontrado.</li>
            <li><code>400</code> — datos inválidos (p. ej. stock insuficiente en una venta).</li>
          </ul>
        </section>

        <section id="wp" className="card doc-card">
          <h2>5. Plugin de WordPress / WooCommerce</h2>
          <p className="muted">
            Si tu tienda es WooCommerce, no necesitas programar: instala el plugin <b>MeliHub Sync</b>,
            pega la URL de tu MeliHub y una API key, y sincroniza el catálogo. Las ventas de WooCommerce
            reportan la baja de stock automáticamente.
          </p>
          <div className="row" style={{ gap: 10, flexWrap: 'wrap', marginTop: 4 }}>
            <a className="btn btn-accent" href="/melihub-sync.zip" download>⬇ Descargar plugin (.zip)</a>
          </div>
          <ol className="doc-list" style={{ marginTop: 14 }}>
            <li>En WordPress: <b>Plugins → Añadir nuevo → Subir plugin</b>, elige el .zip y actívalo (requiere WooCommerce).</li>
            <li>Ve a <b>MeliHub Sync</b> en el menú y pega la <b>URL</b> de tu MeliHub y tu <b>API key</b>.</li>
            <li>Pulsa <b>Sincronizar</b>. Tus productos de Mercado Libre aparecerán en WooCommerce con stock y precio.</li>
          </ol>
        </section>

        <p className="muted sm" style={{ textAlign: 'center', margin: '32px 0' }}>
          ¿Dudas? Escríbenos desde tu panel. · <Link to="/" className="muted">Volver al inicio</Link>
        </p>
      </div>
    </div>
  )
}
