# The Murmurs Page's Tests

The tests of how the murmurs page reads the mycelium node's murmur
records: its logic run by Node's own test runner when the local sites
build the page, and the built page read live in a browser.

- `model.test.ts` tries the records read as murmurs: newest first, each
  named by its quest's title; ties as short ids without the writer's own;
  how a murmur settled, and an empty settling read as none; what can't be
  read said while the rest still draws; the sentence and its turns; the
  quote cut on a word; and ages said short.

Each of those builds the records it reads, so none needs a browser or
a node.

- `murmurs-reads-live-test.py` proves the built page reads live through
  the sites' server in Chromium, beside a stand-in node holding three
  murmurs said minutes before it runs: arriving, settling, going and
  unreadable murmurs, the node and the server going, words kept as
  words, the keys and a tap on those the head shows, and the web-design
  skill's page checks (`murmurs-site-reads-live`).
