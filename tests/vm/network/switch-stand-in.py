"""A TP-Link Easy Smart switch's web page, as far as net's switch reader meets it.

net-switch.nix, beside this, runs it on the machine standing in for the switch. Its status pages
are the real switch's, kept in modules/net/contract/switch/, served only to an address that has
signed in with the password it was started with; any other gets the sign-in page, with a 401,
as the switch answers. It counts each sign-in, right or refused, in a file beside it, so the test
can say how often the reader signed in.

Usage: switch-stand-in.py CONTRACT PASSWORD COUNTED
"""

from __future__ import annotations

import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs

SIGN_IN = b"<html><form action=logon.cgi></form></html>"


def main() -> None:
    """Serve the switch's pages on port 80 until stopped."""
    contract, password, counted = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    # The switch's own address on the test's network, as a real switch has
    # one, rather than every address the machine holds.
    address = sys.argv[4]
    signed_in: set[str] = set()
    counted.write_text("0\n")

    class Switch(BaseHTTPRequestHandler):
        """One request to the switch: a status page, or a sign-in."""

        def do_GET(self) -> None:
            """Give a status page to a signed-in address, else the sign-in page."""
            page = contract / self.path.lstrip("/")
            if self.client_address[0] in signed_in and page.suffix == ".htm" and page.is_file():
                self.answer(200, page.read_bytes())
            else:
                self.answer(401, SIGN_IN)

        def do_POST(self) -> None:
            """Sign an address in when the form holds the password, and count it."""
            if self.path != "/logon.cgi":
                self.answer(404, b"")
                return
            length = int(self.headers.get("Content-Length", "0"))
            form = parse_qs(self.rfile.read(length).decode())
            counted.write_text(f"{int(counted.read_text()) + 1}\n")
            if form.get("username") == ["admin"] and form.get("password") == [password]:
                signed_in.add(self.client_address[0])
            self.answer(200, SIGN_IN)

        def answer(self, status: int, body: bytes) -> None:
            """Send a page whole."""
            self.send_response(status)
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    ThreadingHTTPServer((address, 80), Switch).serve_forever()


if __name__ == "__main__":
    main()
