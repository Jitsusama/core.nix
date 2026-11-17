# The System Page's Tests

The tests of how the system page reads the system daemon's records and
draws them.

- `model.test.ts` tries the records read as the page's model: nothing drawn
  before the summary, the parts in its order with their things, a part not
  told yet said, unknown health read as not read, a part's side and what it
  holds read apart, the headline's counts, the lede, and a record's words
  reaching the page as text. Node's own test runner runs it as the local
  sites build the page, so `system-site-holds` fails when it does.
- `reads-live-test.py` reads the page Nix builds in a headless browser, beside
  a stand-in daemon holding `system.json`: live, then a part that turns, a
  machine that goes, a daemon stopped and back, and a server gone. It is
  `system-site-reads-live`.
- `system.json` is that stand-in's records: a summary, four parts and two
  machines, under test names.

That the page and the daemon find each other on a real account is the VM
test `vm-the-system-page-reads-its-daemon`'s, in `tests/vm/sites/`.
