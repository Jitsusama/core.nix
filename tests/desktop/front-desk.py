"""Stand in for Lantern's front desk, as a hotel's: every page goes there until someone signs in.

Its one page has one button, which signs in. It answers each connection on a thread of its
own, as a real one does: WebKit opens a connection ahead of asking anything, and a server
answering one at a time waits on it while the page and wifid's next check wait behind.
"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# Where it listens: the address the stand-in network gives the desk.
ADDRESS = ("10.42.0.1", 80)
SIGN_IN = "http://front-desk.lantern/sign-in"
PAGE = b"""<!doctype html><title>Lantern</title>
<form method="post" action="/sign-in"><button autofocus>Sign in</button></form>"""
WELCOME = b"<p>Welcome to Lantern.</p>"


class Desk(ThreadingHTTPServer):
    """The desk, which remembers whether anyone has signed in."""

    signed_in = False


class Answers(BaseHTTPRequestHandler):
    """What the desk answers each request with."""

    server: Desk

    def answer(self, code: int, headers: dict[str, str] | None = None, body: bytes = b"") -> None:
        """Answer with a status, the headers given and a body."""
        self.send_response(code)
        for name, value in (headers or {}).items():
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        """Give the sign-in page, nothing once signed in, or send the asker to sign in."""
        if self.path == "/sign-in":
            self.answer(200, {"Content-Type": "text/html"}, PAGE)
        elif self.server.signed_in:
            self.answer(204)
        else:
            self.answer(302, {"Location": SIGN_IN})

    def do_POST(self) -> None:
        """Sign in, whatever was sent, and say welcome."""
        self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.server.signed_in = True
        self.answer(200, {"Content-Type": "text/html"}, WELCOME)


if __name__ == "__main__":
    Desk(ADDRESS, Answers).serve_forever()
