# The Murmurs Page's Tests

The tests of how the murmurs page reads the mycelium node's murmur
records, run by Node's own test runner when the local sites build the
page.

- `model.test.ts` tries the records read as murmurs: newest first, each
  named by its quest's title; ties as short ids without the writer's own;
  how a murmur settled, and an empty settling read as none; what can't be
  read said while the rest still draws; the sentence and its turns; the
  quote cut on a word; and ages said short.

Each test builds the records it reads, so none needs a browser or a
node.
