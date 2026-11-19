# The System Page's Tests

The tests of how the system page reads the system daemon's records and
draws them.

- `model.test.ts` tries what the page makes of the records, the summary, a
  record per part and one per thing, without a browser: the order it draws
  them in, what it says of what isn't told or read, and that a record's
  words reach the page only as text. It also tries what is drawn: the
  legend's marks and words, each machine's section and its ties, the rows
  the keys step through, what a screen reader hears, and that the page's
  own files wear no weather words.
- `machine.test.ts` tries the words a machine is said in: the headline
  over the machines, the lede, its kind, each of its ties, and when it
  was updated, today, yesterday or on a date.
- `bridge.test.ts` tries the bridge drawing: a pier for each machine, wide
  and narrow, its name and health over it, a dot for each cell and one
  for several past what a pier holds, on one scale, what lies past its
  piers, and every name escaped.
- `address.test.ts` tries the page's address for a machine: the name it
  asks for, and where the cursor lands for one drawn, one past those
  drawn, and one the page doesn't have, and when the records that may
  name it are still coming, overdue or all in.
- `page.test.ts` tries that the page preloads every module of its own
  that it reaches only through another, so its first draw comes whole.
- Node's own test runner runs all five as the local sites build the
  page, so `system-site-holds` fails when one does.
- `reads-live-test.py` reads the page Nix builds in a headless browser, beside
  a stand-in daemon holding `system.json`: the page as the records change,
  stop and come back, the cursor landing on a machine's address, even
  one whose record comes after the page has redrawn, and the
  web-design skill's page checks, its links to
  each thing's home among them. It is `system-site-reads-live`.
- `system.json` is that stand-in's records, under test names.

That the page and the daemon find each other on a real account is the VM
test `vm-the-system-page-reads-its-daemon`'s, in `tests/vm/sites/`.
