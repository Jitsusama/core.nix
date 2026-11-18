// What Code draws of the record as markup: every word the daemon or the mycelium says
// is put in as words, never as markup, since a repository's, a branch's or a cell's
// name is whatever someone typed.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { drawer, groveOfTrees, rows } from "../draw.ts";
import { cellsOf, type Feed, type Grove, groveOf } from "../model.ts";

process.env.TZ = "UTC";
const NOW = Date.parse("2026-11-16T15:00:00Z");
const at = (ago: number): string => new Date(NOW - ago).toISOString();

// Markup a name could carry, with a quote to close an attribute it's put in.
const HOSTILE = '<img src=x onerror="alert(1)">';

/** A watch per kind, each told its kind's records whole. */
function told(these: readonly { readonly key: string; readonly kind: string }[]): Feed {
  const kept = (kind: string): Held => {
    const held = new Held();
    held.took({ records: these.filter((record) => record.kind === kind) });
    return held;
  };
  return {
    reading: kept("reading"),
    repository: kept("repository"),
    checkout: kept("checkout"),
    tree: kept("tree"),
    change: kept("change"),
  };
}

const repository = `github:owl/${HOSTILE}`;
const checkout = `/home/ada/src/${HOSTILE}`;
const tree = `/home/ada/trees/${HOSTILE}`;
const change = `change:${repository}#7`;
const records = [
  { key: "reading", kind: "reading", machine: HOSTILE, roots: ["/home/ada/src"], sources: [] },
  {
    key: `repository:${repository}`,
    kind: "repository",
    name: HOSTILE,
    owner: "owl",
    trunk: HOSTILE,
    written_at: at(60_000),
  },
  {
    key: `checkout:${checkout}`,
    kind: "checkout",
    repository,
    path: checkout,
    bare: false,
    head: { branch: HOSTILE, commit: "a1" },
  },
  {
    key: `tree:${tree}`,
    kind: "tree",
    repository,
    checkout,
    path: tree,
    snapshot: false,
    present: true,
    head: { branch: HOSTILE, commit: "b2" },
    written_at: at(60_000),
    change,
  },
  {
    key: change,
    kind: "change",
    repository,
    title: HOSTILE,
    branch: HOSTILE,
    draft: false,
    checks: { state: "failing", passed: 0, failed: [HOSTILE, HOSTILE], pending: [] },
    review: "waiting",
    github: { number: 7, url: `https://github.com/owl/${HOSTILE}/pull/7` },
  },
];
const cells = cellsOf({
  cells: [{ name: HOSTILE, title: HOSTILE, standing: "moving", cwd: `${tree}/src` }],
});

/** The grove drawn from the hostile records, which must hold its one repository. */
function grove(): Grove {
  const drawn = groveOf(told(records), cells, NOW);
  assert.ok(drawn?.repositories[0] !== undefined, "the records drew no repository");
  return drawn;
}

/** Whether `markup` holds `HOSTILE` as words, never as markup. */
function asWords(markup: string): boolean {
  return (
    !markup.includes("<img") && markup.includes("&lt;img src=x onerror=&quot;alert(1)&quot;&gt;")
  );
}

it("puts a repository's name in its row as words", () => {
  const markup = rows(grove().repositories, NOW, "");
  assert.ok(asWords(markup), `a row put a name in as markup: ${markup}`);
});

it("holds each number in a row to the word it counts, so a row never wraps between them", () => {
  const markup = rows(grove().repositories, NOW, "");
  for (const count of ["1\u00a0tree", "1\u00a0cell"]) {
    assert.ok(markup.includes(count), `a row could wrap ${count} from its word: ${markup}`);
  }
});

it("puts what was searched for as words when nothing is named like it", () => {
  const markup = rows([], NOW, HOSTILE);
  assert.ok(asWords(markup), `the list put what was searched for in as markup: ${markup}`);
});

it("puts a repository's name in the grove as words", () => {
  const markup = groveOfTrees(grove());
  assert.ok(asWords(markup), `the grove put a name in as markup: ${markup}`);
});

/** The drawer of the hostile grove's one repository, with every season open. */
function openDrawer(): string {
  const whole = grove();
  const [opened] = whole.repositories;
  assert.ok(opened !== undefined, "the records drew no repository");
  const context = { now: NOW, home: "/home/ada", homes: () => HOSTILE };
  const open = new Set(["today", "week", "resting", "gone"] as const);
  return drawer(opened, whole, context, open);
}

it("puts every name in the drawer as words: checkouts, trees, branches and cells", () => {
  const markup = openDrawer();
  assert.ok(asWords(markup), `the drawer put a name in as markup: ${markup}`);
});

it("puts a change's title, address and failed checks in the drawer as words", () => {
  const markup = openDrawer();
  // The change is drawn, by its number, so the words above were put somewhere.
  assert.ok(markup.includes('class="change failing"'), `the drawer drew no change: ${markup}`);
  assert.ok(
    markup.includes("&lt;img src=x onerror=&quot;alert(1)&quot;&gt;, &lt;img"),
    `the drawer didn't put the failed checks in as words: ${markup}`,
  );
  assert.ok(
    markup.includes('href="https://github.com/owl/&lt;img src=x onerror=&quot;'),
    `the drawer put a change's address in as markup: ${markup}`,
  );
  assert.ok(asWords(markup), `the drawer put a change in as markup: ${markup}`);
});
