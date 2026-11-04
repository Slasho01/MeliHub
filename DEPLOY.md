# Desplegar MeliHub en Render

Dos servicios: **melihub-api** (Rust/Docker) + base PostgreSQL, y **melihub-panel**
(React static site). Cuando tengas la app de Mercado Libre aprobada, enchufas sus
credenciales por variables de entorno.

## 1. API + base de datos
1. Render → **New → Blueprint** → repo de MeliHub → **Apply** (crea db, API y panel).
2. En **melihub-api → Environment** completa:
   - `JWT_SECRET`: cadena larga aleatoria (`openssl rand -hex 32`).
   - `ML_CLIENT_ID`, `ML_CLIENT_SECRET`: de tu app de Mercado Libre.
   - `ML_REDIRECT_URI`: `https://<tu-api>.onrender.com/api/ml/callback`
     (debe coincidir EXACTO con el Redirect URI configurado en la app de ML).
   - `CORS_ORIGINS`: la URL del panel (paso 2).
3. `DATABASE_URL` y `HOST=0.0.0.0` ya quedan puestos. Migraciones automáticas al arrancar.

## 2. Panel
1. En **melihub-panel → Environment**: `VITE_API_URL` = la URL de la API (sin `/api`).
2. Redeploy. (Si cambias `VITE_API_URL`, usa "Clear build cache & deploy".)

## 3. Primer vendedor
Regístrate desde el panel (el primero queda **admin**). Luego conecta tus cuentas
de Mercado Libre y crea API keys para tus plataformas (JarStore, WordPress).

## Notas
- El webhook de ML (cuando lo uses) apunta a `https://<api>/api/ml/webhook`.
- Sin `ML_CLIENT_ID/SECRET` el hub funciona pero no conecta cuentas (modo sin ML).
