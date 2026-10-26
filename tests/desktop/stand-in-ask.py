"""Stand in for pi-ask, which would ask a model, speaking pi's RPC on stdin and stdout.

It speaks what the launcher's Ask row expects (modules/quickshell/launcher/Ask.qml): a
prompt is answered in two pieces a moment apart, then settled; an abort settles what's
being answered as stopped. It starts with the harmless extension error the work machine's
pi prints, which the row has to pass over. A question holding "fail" is answered with the
error a model's provider gives instead, and one holding "open" is done through a tool,
the model saying it will before the call and what it did after. As the work machine's pi
does, a prompt after a new session is started and then never answered, since its proxy
extension keeps the session it began with; and a second question to one pi is answered as
carrying the first, so the row is seen giving each question a pi of its own.
"""

import json
import sys
import threading

# How long each piece of an answer takes, in seconds, so the row is seen being answered.
PIECE = 0.5
ANSWER = ["A fjord is a long, narrow inlet of the sea. ", "Glaciers carved its steep sides."]
# What a second question to one pi is answered with.
CARRIED = "It carries the last question."
# What a request is answered with, either side of its tool call.
BEFORE_THE_CALL = "Let me open it. "
DONE = "It's open."


class Pi:
    """One session's answering, which an abort stops."""

    def __init__(self) -> None:
        """Start answering nothing."""
        self.lock = threading.Lock()
        self.stopped = threading.Event()
        self.answering: threading.Thread | None = None
        self.renewed = False
        self.asked = 0

    def say(self, event: dict[str, object]) -> None:
        """Write one event as a line."""
        with self.lock:
            sys.stdout.write(json.dumps(event) + "\n")
            sys.stdout.flush()

    def answer(self, question: str) -> None:
        """Answer a question piece by piece, unless it's stopped."""
        if "fail" in question:
            self.say(
                {
                    "type": "message_end",
                    "message": {
                        "role": "assistant",
                        "stopReason": "error",
                        "errorMessage": "the model's provider refused",
                    },
                },
            )
            self.say({"type": "agent_settled", "aborted": False})
            return
        self.asked += 1
        # None is where the tool is called.
        pieces: list[str | None] = [*ANSWER] if self.asked == 1 else [CARRIED]
        if "open" in question:
            pieces = [BEFORE_THE_CALL, None, DONE]
        for piece in pieces:
            if piece is None:
                self.say({"type": "tool_execution_start", "toolName": "launcher"})
                continue
            if self.stopped.wait(PIECE):
                break
            self.say(
                {
                    "type": "message_update",
                    "assistantMessageEvent": {"type": "text_delta", "delta": piece},
                },
            )
        aborted = self.stopped.is_set()
        reason = "aborted" if aborted else "stop"
        self.say({"type": "message_end", "message": {"role": "assistant", "stopReason": reason}})
        self.say({"type": "agent_settled", "aborted": aborted})

    def heard(self, command: dict[str, object]) -> None:
        """Answer one command."""
        kind = command.get("type")
        ident = command.get("id")
        if kind == "prompt":
            self.say({"type": "response", "id": ident, "success": True})
            if self.renewed:
                return
            self.stopped.clear()
            self.answering = threading.Thread(
                target=self.answer,
                args=(str(command.get("message", "")),),
                daemon=True,
            )
            self.answering.start()
        elif kind == "abort":
            self.stopped.set()
            if self.answering is not None:
                self.answering.join()
            self.say({"type": "response", "id": ident, "success": True})
        else:
            self.renewed = self.renewed or kind == "new_session"
            self.say({"type": "response", "id": ident, "success": True})


def main() -> None:
    """Read commands until stdin closes."""
    pi = Pi()
    pi.say({"type": "extension_error", "error": 'MCP server "tool-gateway" is registered'})
    for line in sys.stdin:
        if line.strip() == "":
            continue
        pi.heard(json.loads(line))


main()
