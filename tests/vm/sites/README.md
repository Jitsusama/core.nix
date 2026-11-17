# The Sites VM Tests

Joel's local sites as a machine runs them, on machines booted in QEMU: the
sites' server and the daemons its live pages read, each a unit of the
account's own manager as its home module renders it. What a page draws is
the page's own test, run in the build beside a stand-in daemon
(`modules/local-sites/tests`); these prove only that the units find each
other. Each test's opening comment says what it proves.

| File                             | Holds                                                   |
| -------------------------------- | ------------------------------------------------------- |
| `checks.nix`                     | the area's checks, by name                              |
| `code-page.nix`, `-vm-test.py`   | the code page served, its feed live as git writes       |
| `system-page.nix`, `-vm-test.py` | the system page served, its feed live from its daemon   |
