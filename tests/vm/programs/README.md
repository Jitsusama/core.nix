# The Programs VM Tests

The programs Joel runs, started on a machine booted in QEMU, saying what
they have open and doing what the launcher asks of them: what only the
program itself, really running, can show. What the launcher makes of
what they say is proven in the shell's own cases; these prove that a
program says it and acts on it. Each test's opening comment says what it
proves.

| File                                  | Holds                                                   |
| ------------------------------------- | ------------------------------------------------------- |
| `checks.nix`                          | the area's checks, by name                              |
| `chrome-extension.nix`, `-vm-test.py` | Chrome's open tabs, from its extension and session file |
