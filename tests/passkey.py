# Plays a browser to the one security key there is, through Yubico's FIDO
# library: makes a credential for a site and signs in with it, the way a
# passkey (CTAP2) or an older security key (U2F) is used, and checks each
# signature against the key it made.
#
#   passkey make NAME SITE passkey|security|verified
#   passkey sign NAME SITE passkey|security
#
# A passkey is kept on the key, so signing in names only the site; a security
# key's handle is kept here, in NAME.cred. "verified" asks the key to know
# who's there, as some sites do.
import os
import sys
import time

from fido2 import cbor
from fido2.cose import CoseKey
from fido2.ctap import CtapError
from fido2.ctap1 import ApduError, Ctap1
from fido2.ctap2 import Ctap2
from fido2.hid import CtapHidDevice
from fido2.utils import sha256

ES256 = {"type": "public-key", "alg": -7}


def key():
    return next(CtapHidDevice.list_devices())


# A U2F key answers "not yet" until it's been touched, so a browser asks
# again until it's told yes or gives up.
def touched(ask):
    deadline = time.monotonic() + 40
    while True:
        try:
            return ask()
        except ApduError as error:
            waiting = error.code == 0x6985
            if not waiting or time.monotonic() > deadline:
                raise
            time.sleep(0.25)


def make(name, site, kind):
    asked = os.urandom(32)
    if kind == "security":
        app = sha256(site.encode())
        made = touched(lambda: Ctap1(key()).register(asked, app))
        made.verify(app, asked)
        kept = {"handle": made.key_handle, "key": made.public_key}
    else:
        options = {"rk": True, "uv": kind == "verified"}
        made = Ctap2(key()).make_credential(
            asked,
            {"id": site, "name": site},
            {"id": os.urandom(16), "name": "joel"},
            [ES256],
            options=options,
        )
        if not made.auth_data.is_user_present():
            sys.exit("made without anyone there")
        credential = made.auth_data.credential_data
        kept = {
            "id": credential.credential_id,
            "key": dict(credential.public_key),
        }
    with open(f"{name}.cred", "wb") as file:
        file.write(cbor.encode(kept))


def sign(name, site, kind):
    with open(f"{name}.cred", "rb") as file:
        kept = cbor.decode(file.read())
    asked = os.urandom(32)
    if kind == "security":
        app = sha256(site.encode())
        signed = touched(
            lambda: Ctap1(key()).authenticate(asked, app, kept["handle"])
        )
        if not signed.user_presence & 1:
            sys.exit("signed without anyone there")
        signed.verify(app, asked, kept["key"])
    else:
        signed = Ctap2(key()).get_assertion(site, asked)
        if signed.credential["id"] != kept["id"]:
            sys.exit("signed with another passkey")
        if not signed.auth_data.is_user_present():
            sys.exit("signed without anyone there")
        signed.verify(asked, CoseKey.parse(kept["key"]))


def main():
    verb, name, site, kind = sys.argv[1:]
    try:
        {"make": make, "sign": sign}[verb](name, site, kind)
    except (ApduError, CtapError) as error:
        sys.exit(f"the key said no: {error}")


main()
