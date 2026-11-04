#!/usr/bin/env python3
"""Pruebas end-to-end de MeliHub contra un servidor + mock de Mercado Libre.

Uso: python3 smoke_test.py [BASE] [ML_MOCK]
  BASE     base http de la API (por defecto http://127.0.0.1:8090)
  ML_MOCK  base http del mock de ML (por defecto http://127.0.0.1:9933)
El servidor debe correr con ML_API_BASE/ML_AUTH_BASE apuntando al mock.
"""
import json, re, sys, urllib.request, urllib.error

BASE = (sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8090").rstrip("/") + "/api"
ML_MOCK = (sys.argv[2] if len(sys.argv) > 2 else "http://127.0.0.1:9933").rstrip("/")
passed = failed = 0


def call(method, path, body=None, token=None, apikey=None, raw=False):
    h = {"Content-Type": "application/json"}
    if token: h["Authorization"] = "Bearer " + token
    if apikey: h["X-API-Key"] = apikey
    req = urllib.request.Request(BASE + path, method=method, headers=h)
    data = json.dumps(body).encode() if body is not None else None
    try:
        with urllib.request.urlopen(req, data, timeout=20) as r:
            raw_b = r.read()
            return r.status, (raw_b.decode() if raw else (json.loads(raw_b) if raw_b else None))
    except urllib.error.HTTPError as e:
        b = e.read()
        try: return e.code, json.loads(b)
        except Exception: return e.code, b.decode()


def check(name, cond, info=None):
    global passed, failed
    if cond:
        passed += 1; print(f"  ok   {name}")
    else:
        failed += 1; print(f"  FAIL {name} -> {info}")


def expect(name, res, status):
    check(name, res[0] == status, res); return res[1]


print("== Salud / auth")
expect("health", call("GET", "/health"), 200)
reg = expect("registro (primer = admin)", call("POST", "/auth/registro", {"email": "a@t.cl", "password": "secret123", "nombre": "Ana"}), 201)
check("primer vendedor es admin", reg["vendedor"]["rol"] == "admin", reg)
admin = reg["token"]
expect("email duplicado -> 409", call("POST", "/auth/registro", {"email": "a@t.cl", "password": "secret123", "nombre": "x"}), 409)
expect("password corta -> 400", call("POST", "/auth/registro", {"email": "b@t.cl", "password": "123", "nombre": "x"}), 400)
expect("login ok", call("POST", "/auth/login", {"email": "a@t.cl", "password": "secret123"}), 200)
expect("login malo -> 401", call("POST", "/auth/login", {"email": "a@t.cl", "password": "no"}), 401)
expect("me sin token -> 401", call("GET", "/auth/me"), 401)
check("me con token", call("GET", "/auth/me", token=admin)[1]["email"] == "a@t.cl")

print("== Mercado Libre: conectar + sincronizar")
expect("conectar sin token -> 401", call("POST", "/ml/conectar"), 401)
url = expect("conectar", call("POST", "/ml/conectar", {}, token=admin), 200)["authorize_url"]
check("authorize_url válida", "/authorization?" in url and "state=" in url, url)
state = re.search(r"state=([0-9a-f]+)", url).group(1)
expect("callback estado inválido -> 400", call("GET", "/ml/callback?code=X&state=malo", raw=True), 400)
expect("callback ok (HTML)", call("GET", f"/ml/callback?code=X&state={state}", raw=True), 200)
cuentas = expect("listar cuentas", call("GET", "/ml/cuentas", token=admin), 200)
check("1 cuenta conectada", len(cuentas) == 1 and cuentas[0]["sitio"] == "MLC", cuentas)
cid = cuentas[0]["id"]
sync = expect("sincronizar", call("POST", f"/ml/cuentas/{cid}/sincronizar", {}, token=admin), 200)
check("se importan 3 publicaciones", sync["sincronizados"] == 3, sync)
prods = expect("catálogo con ganancias", call("GET", "/productos", token=admin), 200)
check("3 productos", len(prods) == 3, len(prods))
check("ganancia ML calculada", prods[0]["ml"] is not None, prods[0])

print("== Gestor de inventario")
p = next(x for x in prods if x["ml_item_id"] == "MLC001")
expect("fijar costo y precio JarStore", call("PUT", f"/productos/{p['id']}/inventario", {"costo": 1000, "precio_jarstore": 4000}, token=admin), 200)
p = next(x for x in call("GET", "/productos", token=admin)[1] if x["ml_item_id"] == "MLC001")
check("precio_jarstore guardado", p["precio_jarstore"] == 4000 and p["costo"] == 1000, p)
check("ganancia JarStore calculada (margen>0)", p["jarstore"] and p["jarstore"]["ganancia"] > 0, p.get("jarstore"))
call("POST", f"/ml/cuentas/{cid}/sincronizar", {}, token=admin)
p = next(x for x in call("GET", "/productos", token=admin)[1] if x["ml_item_id"] == "MLC001")
check("la re-sincronización NO borra costo/precio", p["costo"] == 1000 and p["precio_jarstore"] == 4000, p)
cfg = expect("config ganancias (default)", call("GET", "/config/ganancias", token=admin), 200)
check("defaults razonables", cfg["iva_pct"] == 19.0 and cfg["comision_ml_pct"] == 13.0, cfg)
g13 = next(x for x in call("GET", "/productos", token=admin)[1] if x["ml_item_id"] == "MLC001")["ml"]["ganancia"]
expect("actualizar comisión ML", call("PUT", "/config/ganancias", {"comision_ml_pct": 10.0}, token=admin), 200)
g10 = next(x for x in call("GET", "/productos", token=admin)[1] if x["ml_item_id"] == "MLC001")["ml"]["ganancia"]
check("menor comisión -> más ganancia", g10 > g13, (g13, g10))

print("== API keys + distribución")
k = expect("crear API key", call("POST", "/api-keys", {"nombre": "WordPress", "canal": "jarstore"}, token=admin), 201)
key = k["api_key"]
check("la key se devuelve una vez", key.startswith("mhk_"), key)
check("keys listadas", len(call("GET", "/api-keys", token=admin)[1]) == 1)
expect("pub sin api-key -> 401", call("GET", "/pub/productos"), 401)
expect("pub api-key inválida -> 401", call("GET", "/pub/productos", apikey="mhk_mala"), 401)
pub = expect("pub/productos", call("GET", "/pub/productos", apikey=key), 200)
mlc1 = next(x for x in pub if x["ml_item_id"] == "MLC001")
check("sirve el precio del canal JarStore", mlc1["precio"] == 4000, mlc1)
stock0 = mlc1["stock"]

print("== Venta: stock en dos vías")
v = expect("reportar venta (2 uds)", call("POST", "/pub/ventas", {"producto_id": mlc1["id"], "cantidad": 2}, apikey=key), 200)
check("stock baja en MeliHub", v["nuevo_stock"] == stock0 - 2, v)
check("stock empujado a Mercado Libre", v["ml_actualizado"] is True, v)
# verificar en el mock de ML
ml_item = json.loads(urllib.request.urlopen(ML_MOCK + "/items?ids=MLC001&attributes=id").read())[0]["body"]
check("ML (mock) refleja el nuevo stock", ml_item["available_quantity"] == stock0 - 2, ml_item)
expect("stock insuficiente -> 400", call("POST", "/pub/ventas", {"producto_id": mlc1["id"], "cantidad": 9999}, apikey=key), 400)

print("== Webhook")
expect("webhook (200 y async)", call("POST", "/ml/webhook", {"topic": "items", "resource": "/items/MLC002", "user_id": 1702973815}, raw=True), 200)
expect("webhook de usuario desconocido -> 200 ignorado", call("POST", "/ml/webhook", {"topic": "items", "resource": "/items/MLC002", "user_id": 999}, raw=True), 200)

print("== Aislamiento multi-tenant")
call("POST", "/auth/registro", {"email": "otro@t.cl", "password": "secret123", "nombre": "Otro"})
otro = call("POST", "/auth/login", {"email": "otro@t.cl", "password": "secret123"})[1]["token"]
check("otro vendedor no ve cuentas ajenas", len(call("GET", "/ml/cuentas", token=otro)[1]) == 0)
check("otro vendedor no ve productos ajenos", len(call("GET", "/productos", token=otro)[1]) == 0)
expect("otro no sincroniza cuenta ajena -> 403", call("POST", f"/ml/cuentas/{cid}/sincronizar", {}, token=otro), 403)

print("== Revocar API key")
ks = call("GET", "/api-keys", token=admin)[1]
expect("revocar key", call("DELETE", f"/api-keys/{ks[0]['id']}", token=admin), 204)
expect("key revocada ya no sirve -> 401", call("GET", "/pub/productos", apikey=key), 401)

print(f"\n{passed} ok, {failed} fallidas")
sys.exit(1 if failed else 0)
