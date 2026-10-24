"""Give the one security key there is a PIN, through Yubico's library, as a person would.

set-pin PIN
"""

import sys

from fido2.ctap2.base import Ctap2
from fido2.ctap2.pin import ClientPin
from fido2.hid import CtapHidDevice


def main(argv: list[str]) -> None:
    """Set the first key found to the PIN given."""
    device = next(iter(CtapHidDevice.list_devices()))
    ClientPin(Ctap2(device)).set_pin(argv[0])


if __name__ == "__main__":
    main(sys.argv[1:])
