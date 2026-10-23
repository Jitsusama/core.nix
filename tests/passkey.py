"""Play a browser to the one security key there is, through Yubico's library.

It makes a credential for a site and signs in with it, the way a passkey
(CTAP2) or an older security key (U2F) is used, and checks each signature
against the key it made.

    passkey make NAME SITE passkey|security|verified
    passkey sign NAME SITE passkey|security

A passkey is kept on the key, so signing in names only the site; a security
key's handle is kept here, in NAME.cred. "verified" asks the key to know
who's there, as some sites do.
"""

import os
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

from fido2 import cbor
from fido2.cose import CoseKey
from fido2.ctap import CtapError
from fido2.ctap1 import ApduError, Ctap1
from fido2.ctap2.base import Ctap2
from fido2.hid import CtapHidDevice
from fido2.utils import sha256

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping

ES256 = {"type": "public-key", "alg": -7}

# What a U2F key answers until it's been touched: the conditions of its use
# aren't met yet. A browser asks again until told yes, or this long passes.
NOT_TOUCHED_YET = 0x6985
TOUCH_WAIT_SECONDS = 40


def key() -> CtapHidDevice:
    """Find the security key there is."""
    return next(CtapHidDevice.list_devices())


def touched[T](ask: Callable[[], T]) -> T:
    """Ask a U2F key until it's touched, and give its answer."""
    deadline = time.monotonic() + TOUCH_WAIT_SECONDS
    while True:
        try:
            return ask()
        except ApduError as error:
            if error.code != NOT_TOUCHED_YET or time.monotonic() > deadline:
                raise
            time.sleep(0.25)


def kept_path(name: str) -> Path:
    """Give where a credential's record is kept."""
    return Path(f"{name}.cred")


def kept_field[T](kept: Mapping[str, object], field: str, kind: type[T]) -> T:
    """Give a field of a kept record, refusing one that isn't what it should be."""
    value = kept.get(field)
    if not isinstance(value, kind):
        sys.exit(f"the record keeps no {field}")
    return value


def make(name: str, site: str, kind: str) -> None:
    """Make a credential for the site, and keep what signing in needs."""
    asked = os.urandom(32)
    kept: dict[str, object]
    if kind == "security":
        app = sha256(site.encode())
        registered = touched(lambda: Ctap1(key()).register(asked, app))
        registered.verify(app, asked)
        kept = {"handle": registered.key_handle, "key": registered.public_key}
    else:
        made = Ctap2(key()).make_credential(
            asked,
            {"id": site, "name": site},
            {"id": os.urandom(16), "name": "joel"},
            [ES256],
            options={"rk": True, "uv": kind == "verified"},
        )
        if not made.auth_data.is_user_present():
            sys.exit("made without anyone there")
        credential = made.auth_data.credential_data
        if credential is None:
            sys.exit("made with no credential")
        kept = {"id": credential.credential_id, "key": dict(credential.public_key)}
    kept_path(name).write_bytes(cbor.encode(kept))


def sign(name: str, site: str, kind: str) -> None:
    """Sign in to the site with the credential made, and check the signature."""
    kept = cbor.decode(kept_path(name).read_bytes())
    if not isinstance(kept, dict):
        sys.exit("the record isn't one this kept")
    asked = os.urandom(32)
    if kind == "security":
        app = sha256(site.encode())
        handle = kept_field(kept, "handle", bytes)
        signed = touched(lambda: Ctap1(key()).authenticate(asked, app, handle))
        if not signed.user_presence & 1:
            sys.exit("signed without anyone there")
        signed.verify(app, asked, kept_field(kept, "key", bytes))
    else:
        asserted = Ctap2(key()).get_assertion(site, asked)
        if asserted.credential["id"] != kept_field(kept, "id", bytes):
            sys.exit("signed with another passkey")
        if not asserted.auth_data.is_user_present():
            sys.exit("signed without anyone there")
        asserted.verify(asked, CoseKey.parse(kept_field(kept, "key", dict)))


VERBS: dict[str, Callable[[str, str, str], None]] = {"make": make, "sign": sign}


def main(argv: list[str]) -> None:
    """Make or sign, as the command line says."""
    verb, name, site, kind = argv
    try:
        VERBS[verb](name, site, kind)
    except (ApduError, CtapError) as error:
        sys.exit(f"the key said no: {error}")


if __name__ == "__main__":
    main(sys.argv[1:])
