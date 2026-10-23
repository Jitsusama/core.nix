"""Stand in for the pages the Wi-Fi test's networks are checked and signed in at.

tests/wifi.nix runs this inside the access points' namespace, given the folder of its
stand-in certificates. Lantern reaches beyond itself, so its check pages answer as Google's
do, over HTTP and HTTPS, and its API gives a session a few seconds long. The Inn sends every
guest to its front desk until they've signed in there, and answers nothing over HTTPS, as a
network letting only the plain check through would.
"""

import json
import ssl
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LANTERN = "10.42.0.1"
# How long the session Lantern's API gives lasts, in seconds, from when it's first asked.
SESSION = 15
signed_in: set[str] = set()
session: list[float] = []


class Page(BaseHTTPRequestHandler):
    """Answer a network's check, sign-in or question about its session."""

    def answer(self, code: int, headers: dict[str, str] | None = None, body: bytes = b"") -> None:
        """Answer with a status, its headers and a body."""
        self.send_response(code)
        for name, value in (headers or {}).items():
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        """Answer as the network the request reached does."""
        network: str = self.connection.getsockname()[0]
        if self.path == "/sign-in":
            signed_in.add(network)
            self.answer(200, body=b"Welcome")
        elif self.path == "/captive":
            session[:] = session or [time.monotonic()]
            left = SESSION - (time.monotonic() - session[0])
            said: dict[str, object] = {"captive": False, "seconds-remaining": int(left)}
            if left <= 0:
                said = {"captive": True, "user-portal-url": "https://router.lantern/renew"}
            kind = {"Content-Type": "application/captive+json"}
            self.answer(200, kind, json.dumps(said).encode())
        elif network == LANTERN or network in signed_in:
            self.answer(204)
        else:
            self.answer(302, {"Location": "http://front-desk.inn/sign-in"})


certificates = Path(sys.argv[1])
secure = ThreadingHTTPServer((LANTERN, 443), Page)
tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
tls.load_cert_chain(certificates / "page.pem", certificates / "page.key")
secure.socket = tls.wrap_socket(secure.socket, server_side=True)
threading.Thread(target=secure.serve_forever, daemon=True).start()
ThreadingHTTPServer(("", 80), Page).serve_forever()
