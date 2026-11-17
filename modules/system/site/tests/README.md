# The System Page's Tests

The tests of how the system page reads the system daemon's records and
draws them.

- `model.test.ts` tries what the page makes of the records, the summary, a
  record per part and one per thing, without a browser: the order it draws
  them in, what it says of what isn't told or read, the headline and the
  lede, and that a record's words reach the page only as text. It also
  tries what is drawn in the system's words: the legend's marks and words,
  what each part says on the outlook and what a screen reader hears, and
  that the page's own files wear no weather words. Node's own
  test runner runs it as the local sites build the page, so
  `system-site-holds` fails when it does.
- `reads-live-test.py` reads the page Nix builds in a headless browser, beside
  a stand-in daemon holding `system.json`: the page as the records change,
  stop and come back, and the web-design skill's page checks, its links to
  each thing's home among them. It is `system-site-reads-live`.
- `system.json` is that stand-in's records, under test names.

That the page and the daemon find each other on a real account is the VM
test `vm-the-system-page-reads-its-daemon`'s, in `tests/vm/sites/`.
