"""Check the IPU bridge knows the XPS 14's colour sensor, OVTI08F4, is upside down.

    ipu-bridge-knows-the-upside-down-sensor BRIDGE

BRIDGE is ipu-bridge.c, as the machine's kernel builds it. The sensor's firmware says it
isn't upside down, so only an entry in the bridge's upside-down table that matches the
XPS 14 makes it report 180 degrees.
"""

import re
import sys
from pathlib import Path


def main(argv: list[str]) -> None:
    """Fail, saying why, unless the table holds OVTI08F4 for Dell's XPS 14 DA14260."""
    source = Path(argv[0]).read_text()
    table = re.search(r"upside_down_sensor_dmi_ids\[\] = \{(.*?)\n\};", source, re.DOTALL)
    if not table:
        sys.exit("The bridge has no upside-down sensor table.")
    ours = [
        entry
        for entry in re.split(r"\n\t\{\n", table.group(1))
        if '"Dell Inc."' in entry and '"XPS 14 DA14260"' in entry
    ]
    if not any('.driver_data = "OVTI08F4"' in entry for entry in ours):
        sys.exit("The bridge doesn't know the XPS 14's colour sensor is upside down.")


if __name__ == "__main__":
    main(sys.argv[1:])
