"""Stand in for dictation's service, which would load Whisper, on the socket systemd hands it.

It speaks the service's protocol (modules/dictation/src/service.rs): the first press listens,
the second types for a few seconds, a press meanwhile says it's still writing, and then it
says it heard nothing, in the silence the VM's microphone gives. A client that sends "watch"
is told the state, then each change, until it goes.
"""

import socket
import threading
import time

# The socket systemd hands a socket-activated service.
LISTENING = 3
# How long the stand-in types for, in seconds.
TYPING = 5


class Dictation:
    """Dictation's state, and the clients watching it."""

    def __init__(self) -> None:
        """Start idle, with nobody watching."""
        self.state = "idle"
        self.watchers: list[socket.socket] = []
        self.lock = threading.Lock()

    def tell(self, word: str) -> None:
        """Tell every watcher a word, forgetting any that has gone."""
        for watcher in list(self.watchers):
            try:
                watcher.sendall(f"{word}\n".encode())
            except OSError:
                self.watchers.remove(watcher)

    def become(self, state: str) -> None:
        """Change state, and tell the watchers."""
        self.state = state
        self.tell(state)

    def typed(self) -> None:
        """Type for a while, then say nothing was heard and go back to idle."""
        time.sleep(TYPING)
        with self.lock:
            self.tell("heard-nothing silent")
            self.become("idle")

    def serve(self, client: socket.socket) -> None:
        """Answer one client: a watcher stays, a press moves dictation on."""
        line = client.makefile().readline().strip()
        with self.lock:
            if line == "watch":
                client.sendall(f"{self.state}\n".encode())
                self.watchers.append(client)
                return
            if self.state == "idle":
                self.become("listening")
            elif self.state == "listening":
                self.become("transcribing")
                threading.Thread(target=self.typed, daemon=True).start()
            else:
                self.tell("still-writing")
            client.sendall(f"{self.state}\n".encode())
        client.close()


def main() -> None:
    """Serve every client systemd's socket brings, each in a thread of its own."""
    listener = socket.socket(fileno=LISTENING)
    dictation = Dictation()
    while True:
        client, _ = listener.accept()
        threading.Thread(target=dictation.serve, args=(client,), daemon=True).start()


if __name__ == "__main__":
    main()
