"""Stand in for the summarizer's model, which a VM can't run.

It answers as `call-transcripts ask` does, a JSON prompt a line in and a JSON answer a line
out, until its input ends. It picks every line it's shown, copies each marked line whole as
its quote, and keeps every quote. Into the folder named first it writes its process's id,
notes "started" in its log each time it starts, and holds its first answer until something
is written to the pipe named gate there, so a test can start a call while it's summing up.
"""

import json
import os
import re
import sys
from pathlib import Path

folder = Path(sys.argv[1])
(folder / "summarizer-pid").write_text(f"{os.getpid()}\n")
with (folder / "summarizer-log").open("a") as log:
    log.write("started\n")
# Opening a pipe to read waits until the test opens it to write.
with (folder / "gate").open() as gate:
    gate.read()


def answer(prompt: str) -> str:
    """Answer as a model asked the summary's three questions would."""
    if prompt.startswith("These are numbered lines"):
        return ", ".join(re.findall(r"^(L\d+) ", prompt, re.MULTILINE))
    if "Copy, word for word" in prompt:
        marked = re.search(r"^>>> L\d+ \[[^\]]*\] [^:]+: (.*)$", prompt, re.MULTILINE)
        return marked[1] if marked else "NONE"
    return "YES"


for line in sys.stdin:
    asked = json.loads(line)
    print(json.dumps({"answer": answer(asked["prompt"])}), flush=True)
