#!/usr/bin/env python3
# Mock de la API de Mercado Libre para probar MeliHub.
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

USER_ID = 1702973815
ITEMS = {
    "MLC001": {"id":"MLC001","title":"Balanza Digital 40kg","price":19940,"available_quantity":7,"status":"active","permalink":"https://ml.cl/MLC001","thumbnail":"https://img/1.jpg","category_id":"MLC1234","currency_id":"CLP","variations":[]},
    "MLC002": {"id":"MLC002","title":"Atril Para Partituras","price":11990,"available_quantity":3,"status":"active","permalink":"https://ml.cl/MLC002","thumbnail":"https://img/2.jpg","category_id":"MLC5678","currency_id":"CLP","variations":[]},
    "MLC003": {"id":"MLC003","title":"Power Bank USB-C","price":79990,"available_quantity":0,"status":"paused","permalink":"https://ml.cl/MLC003","thumbnail":"https://img/3.jpg","category_id":"MLC9999","currency_id":"CLP","variations":[]},
}
class H(BaseHTTPRequestHandler):
    def log_message(self,*a): pass
    def _send(self, obj, code=200):
        b=json.dumps(obj).encode(); self.send_response(code)
        self.send_header("Content-Type","application/json"); self.send_header("Content-Length",str(len(b))); self.end_headers(); self.wfile.write(b)
    def do_POST(self):
        p=urlparse(self.path)
        if p.path=="/oauth/token":
            self._send({"access_token":"ACCESS-123","refresh_token":"REFRESH-456","expires_in":21600,"user_id":USER_ID,"token_type":"bearer"})
        else: self._send({"error":"not_found"},404)
    def do_PUT(self):
        n=int(self.headers.get('Content-Length',0)); body=json.loads(self.rfile.read(n) or b'{}')
        iid=self.path.rstrip('/').split('/')[-1]
        if iid in ITEMS and 'available_quantity' in body: ITEMS[iid]['available_quantity']=body['available_quantity']
        self._send({"id":iid,"available_quantity":body.get('available_quantity')})
    def do_GET(self):
        p=urlparse(self.path); q=parse_qs(p.query)
        if p.path.startswith("/users/") and p.path.endswith("/items/search"):
            self._send({"results":list(ITEMS.keys()),"paging":{"total":len(ITEMS),"offset":0,"limit":50}})
        elif p.path.startswith("/users/"):
            self._send({"id":USER_ID,"nickname":"JARSTORE_CL","site_id":"MLC"})
        elif p.path=="/items":
            ids=(q.get("ids",[""])[0]).split(",")
            self._send([{"code":200,"body":ITEMS[i]} for i in ids if i in ITEMS])
        else: self._send({"error":"not_found"},404)
ThreadingHTTPServer(("127.0.0.1",9933),H).serve_forever()
