"""What a shell case said it drew, for the checks that read its pictures back.

A case run offscreen keeps pictures of what it drew and says, on a line of
its log starting MEANT, what each picture is, as JSON. turned-mark.py and
emoji-marks.py read that line here, so it's read one way.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path


def meant(log: Path) -> dict[str, Any]:
    """Read what the shell said it drew."""
    for line in log.read_text(encoding="utf-8").splitlines():
        if "MEANT " in line:
            said = json.loads(line.split("MEANT ", 1)[1])
            if isinstance(said, dict):
                return said
    silent = f"the shell never said what it drew, in {log}"
    raise SystemExit(silent)
