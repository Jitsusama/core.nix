"""Check dell-wmi ignores the XPS 14's hourly type 0x0012 event, code 0x0000.

    dell-wmi-ignores-the-hourly-event DRIVER

DRIVER is dell-wmi-base.c, as the machine's kernel builds it. Without the event in its
keymap for that type, as one to ignore, the driver logs it as an unknown key each hour.
"""

import re
import sys
from pathlib import Path


def main(argv: list[str]) -> None:
    """Fail, saying why, unless the type 0x0012 keymap ignores code 0x0000."""
    source = Path(argv[0]).read_text()
    keymap = re.search(r"dell_wmi_keymap_type_0012\[\] = \{(.*?)\n\};", source, re.DOTALL)
    if not keymap:
        sys.exit("dell-wmi has no keymap for type 0x0012 events.")
    if not re.search(r"\{ KE_IGNORE, 0x0000,", keymap.group(1)):
        sys.exit("dell-wmi logs the XPS 14's hourly type 0x0012 event as an unknown key.")


if __name__ == "__main__":
    main(sys.argv[1:])
