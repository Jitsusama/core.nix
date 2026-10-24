"""Make an age recipient for the security key with age-plugin-fido2-hmac, as a person makes one.

    make-recipient PIN

It answers the plugin's questions as its README says: the PIN, a PIN for decrypting too,
and no separate identity, so the key alone opens what's sealed to it. It prints the
recipient, and what the plugin said goes to standard error.
"""

import os
import pty
import re
import select
import sys

PLUGIN = "@plugin@"
# How long the plugin may say nothing before it counts as stuck, in seconds.
QUIET = 60
# age's terminal takes a choice as 1 or 2 (filippo.io/age/plugin/tui.go).
CHOICES = ("1", "2")


def answers(pin: str) -> list[tuple[str, str]]:
    """Give each of the plugin's questions, by words it asks with, and its answer."""
    return [
        ("enter your PIN", pin),
        ("require a PIN", "1"),
        ("separate identity", "2"),
    ]


def talk(fd: int, pin: str) -> str:
    """Answer the plugin on its terminal until it's done, and give all it said."""
    said = ""
    answered: set[str] = set()
    while True:
        if not select.select([fd], [], [], QUIET)[0]:
            sys.exit("the plugin went quiet, having said:\n" + said)
        try:
            chunk = os.read(fd, 1024).decode(errors="replace")
        except OSError:
            return said
        if not chunk:
            return said
        said += chunk
        for prompt, answer in answers(pin):
            if prompt in said and prompt not in answered:
                answered.add(prompt)
                # A choice is one key; a value ends with Return.
                os.write(fd, (answer if answer in CHOICES else answer + "\n").encode())


def main(argv: list[str]) -> None:
    """Run the plugin on a terminal of its own, and print the recipient it made."""
    pid, fd = pty.fork()
    if pid == 0:
        # The plugin's path is the store's, fixed when the test is built.
        os.execv(PLUGIN, ["age-plugin-fido2-hmac", "-g"])  # noqa: S606
    said = talk(fd, argv[0])
    os.waitpid(pid, 0)
    sys.stderr.write(said)
    print(re.findall(r"age1fido2-hmac1[0-9a-z]+", said)[-1])


if __name__ == "__main__":
    main(sys.argv[1:])
