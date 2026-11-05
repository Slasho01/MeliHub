import { useState } from 'react'

const ACTUALIZADO = 'noviembre 2026'

function Terminos() {
  return (
    <div className="stack" style={{ lineHeight: 1.7 }}>
      <h2>Términos y Condiciones</h2>
      <p className="muted sm">Última actualización: {ACTUALIZADO}</p>
      <p><b>1. El servicio.</b> MeliHub es una herramienta que centraliza las publicaciones de Mercado Libre de una o más cuentas, administra inventario, precios y ganancias, y las distribuye a plataformas de terceros (JarStore, WordPress) mediante API-keys. El uso del servicio implica la aceptación de estos términos.</p>
      <p><b>2. Cuenta.</b> Eres responsable de la veracidad de tus datos, de la seguridad de tu contraseña y de toda actividad en tu cuenta. Debes tener autorización para conectar las cuentas de Mercado Libre que vincules.</p>
      <p><b>3. Uso aceptable.</b> No puedes usar MeliHub para fines ilícitos, para vulnerar los términos de Mercado Libre, Mercado Pago u otras plataformas, ni para acceder a datos que no te pertenecen.</p>
      <p><b>4. Integraciones de terceros.</b> MeliHub se conecta a Mercado Libre, Mercado Pago y otros servicios. El funcionamiento de esas integraciones depende de dichas plataformas; no garantizamos su disponibilidad ni nos hacemos responsables de cambios en sus APIs o políticas.</p>
      <p><b>5. Suscripción y pagos.</b> Los planes de pago se cobran de forma mensual mediante Mercado Pago. Puedes cancelar cuando quieras; el acceso al plan se mantiene hasta el fin del período pagado. No hay reembolsos por períodos ya iniciados, salvo obligación legal.</p>
      <p><b>6. Disponibilidad.</b> Hacemos esfuerzos razonables por mantener el servicio operativo, pero se ofrece "tal cual", sin garantías de disponibilidad ininterrumpida.</p>
      <p><b>7. Limitación de responsabilidad.</b> En la medida permitida por la ley, MeliHub no responde por daños indirectos, pérdida de ventas, de datos o de stock derivados del uso del servicio o de fallas de integraciones de terceros.</p>
      <p><b>8. Terminación.</b> Podemos suspender cuentas que incumplan estos términos. Puedes cerrar tu cuenta cuando quieras.</p>
      <p><b>9. Ley aplicable.</b> Estos términos se rigen por las leyes de la República de Chile.</p>
      <p className="muted sm"><i>Este es un borrador de referencia; revísalo con un abogado antes de operar comercialmente.</i></p>
    </div>
  )
}

function Privacidad() {
  return (
    <div className="stack" style={{ lineHeight: 1.7 }}>
      <h2>Política de Privacidad</h2>
      <p className="muted sm">Última actualización: {ACTUALIZADO}</p>
      <p><b>1. Datos que tratamos.</b> (a) Datos de tu cuenta: nombre, email, contraseña (cifrada). (b) Credenciales de acceso a Mercado Libre (tokens OAuth) que autorizas, almacenadas de forma cifrada. (c) Datos de tus publicaciones, inventario y ventas traídos de Mercado Libre. (d) Datos de pago procesados por Mercado Pago (no almacenamos tu tarjeta).</p>
      <p><b>2. Para qué los usamos.</b> Para operar el servicio: sincronizar publicaciones, calcular ganancias, distribuir a tus plataformas, procesar la suscripción y enviarte notificaciones.</p>
      <p><b>3. Con quién los compartimos.</b> Con los proveedores necesarios para prestar el servicio: Mercado Libre y Mercado Pago (integraciones), el proveedor de infraestructura/base de datos y el de envío de correos. No vendemos tus datos.</p>
      <p><b>4. Seguridad.</b> Usamos HTTPS, contraseñas con hashing (bcrypt) y cifrado en reposo de los tokens de Mercado Libre. Ningún sistema es 100% infalible, pero aplicamos medidas razonables.</p>
      <p><b>5. Tus derechos.</b> Conforme a la Ley N° 19.628 (Chile), puedes acceder, rectificar o eliminar tus datos. Al cerrar tu cuenta, eliminamos tus datos asociados.</p>
      <p><b>6. Retención.</b> Conservamos tus datos mientras tengas una cuenta activa y el tiempo necesario para cumplir obligaciones legales.</p>
      <p><b>7. Contacto.</b> Para ejercer tus derechos o consultas de privacidad, escríbenos al correo de soporte.</p>
      <p className="muted sm"><i>Este es un borrador de referencia; revísalo con un abogado antes de operar comercialmente.</i></p>
    </div>
  )
}

export default function Legal() {
  const [tab, setTab] = useState(() => (window.location.hash === '#privacidad' ? 'privacidad' : 'terminos'))
  return (
    <div className="stack" style={{ maxWidth: 760 }}>
      <div className="row" style={{ gap: 8 }}>
        <button className={`btn btn-sm ${tab === 'terminos' ? 'btn-accent' : ''}`} onClick={() => setTab('terminos')}>Términos</button>
        <button className={`btn btn-sm ${tab === 'privacidad' ? 'btn-accent' : ''}`} onClick={() => setTab('privacidad')}>Privacidad</button>
      </div>
      <div className="card">{tab === 'terminos' ? <Terminos /> : <Privacidad />}</div>
    </div>
  )
}
