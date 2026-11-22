// What Code draws of the record as markup: every word the daemon or the mycelium says
// is put in as words, never as markup, since a repository's, a branch's or a cell's
// name is whatever someone typed.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { drawer, groveOfTrees, rows } from "../draw.ts";
import { cellsOf, type Feed, type Grove, groveOf, type Season } from "../model.ts";

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
    using: kept("using"),
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

/** The grove drawn from the hostile records, or `these`, which must hold its one repository. */
function grove(these: Parameters<typeof told>[0] = records): Grove {
  const drawn = groveOf(told(these), cells, NOW);
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

it("holds a count to its own word only, never a part ending in a digit to the next", () => {
  const head = { branch: "v2", commit: "a1", upstream: { count: { behind: 3, ahead: 0 } } };
  const told2 = records.map((each) => (each.kind === "checkout" ? { ...each, head } : each));
  const whole = groveOf(told(told2), cells, NOW);
  const [opened] = whole?.repositories ?? [];
  assert.ok(whole !== undefined && opened !== undefined, "the records drew no repository");
  const context = { now: NOW, home: "/home/ada", homes: () => HOSTILE };
  const markup = drawer(opened, whole, context, new Set());
  assert.ok(markup.includes("v2 · 3\u00a0behind"), `a branch was held to the separator: ${markup}`);
});

it("names each cell's link to one home apart, so a redraw finds the cell's own again", () => {
  const two = cellsOf({
    cells: ["E2ZIAQ", "Q3NEWS"].map((name) => ({
      name,
      title: "at work",
      standing: "moving",
      cwd: `${tree}/src`,
    })),
  });
  const whole = groveOf(told(records), two, NOW);
  const [opened] = whole?.repositories ?? [];
  assert.ok(whole !== undefined && opened !== undefined, "the records drew no repository");
  const context = { now: NOW, home: "/home/ada", homes: () => "cells/#c=1" };
  const markup = drawer(opened, whole, context, new Set(["today"]));
  for (const name of ["E2ZIAQ", "Q3NEWS"]) {
    assert.ok(markup.includes(`data-key="cell:${name}"`), `${name}'s link wasn't named: ${markup}`);
  }
});

/**
 * The drawer of a repository with cell `cell` at work in a tree of it, across a repository
 * named `name` in the network `network`, as the mesh spells its name.
 */
function leadingDrawer(cell: string, name: string, network: string): string {
  const notes = "github:owl/notes";
  const both = [
    ...records,
    { key: `repository:${notes}`, kind: "repository", name, owner: "owl" },
    {
      key: "checkout:/home/ada/src/notes",
      kind: "checkout",
      repository: notes,
      path: "/home/ada/src/notes",
      bare: false,
      head: { branch: "main", commit: "c3" },
    },
  ];
  const working = { name: cell, title: "Join the notes", standing: "moving", networks: [network] };
  const whole = groveOf(told(both), cellsOf({ cells: [{ ...working, cwd: `${tree}/src` }] }), NOW);
  const opened = whole?.repositories.find((each) => each.key === repository);
  assert.ok(whole !== undefined && opened !== undefined, "the records drew no repository");
  const context = { now: NOW, home: "/home/ada", homes: () => undefined };
  return drawer(opened, whole, context, new Set(["today"]));
}

it("leads a tree's cell on to each other repository it works across, where Code shows it", () => {
  const markup = leadingDrawer("W9TIES", "notes", "repo.notes");
  const link = '<a href="#repo=owl%2Fnotes" data-key="across:W9TIES:github:owl/notes">notes</a>';
  const lead = `<span class="cell-across">also in ${link}</span>`;
  assert.ok(markup.includes(lead), `W9TIES didn't lead on to notes: ${markup}`);
});

it("puts a cell's name and the repositories it leads on to as words", () => {
  // The opened repository is named HOSTILE already, so the other is named apart from it, as a
  // name two repositories share joins neither.
  const markup = leadingDrawer(HOSTILE, "<i>notes</i>", "repo.i-notes-i");
  const lead = markup.slice(markup.indexOf('class="cell-across"'));
  assert.ok(markup.includes('class="cell-across"'), `the cell led nowhere: ${markup}`);
  assert.ok(asWords(lead), `the lead put the cell's name in as markup: ${lead}`);
  assert.ok(
    lead.includes("&lt;i&gt;notes&lt;/i&gt;") && !lead.includes("<i>"),
    `the lead put the repository's name in as markup: ${lead}`,
  );
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
function openDrawer(these: Parameters<typeof told>[0] = records): string {
  const whole = grove(these);
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

/** The hostile records with their repository keyed `key` instead. */
function keyedAs(key: string): Parameters<typeof told>[0] {
  return records.map((each) => {
    const entries = Object.entries(each).map(([name, value]) => [
      name,
      typeof value === "string" ? value.replace(repository, key) : value,
    ]);
    return Object.fromEntries(entries) as (typeof records)[number];
  });
}

it("leads to the launcher by a key mycelium-link takes whole, and by no other", () => {
  const link = (key: string): string =>
    `<a href="mycelium:repository/${key}" data-key="launcher:${key}">Open it in the launcher</a>`;
  for (const key of ["github:owl/notes", "local:~/src/notes", "gitlab:group/sub/notes"]) {
    const markup = openDrawer(keyedAs(key));
    assert.ok(markup.includes(link(key)), `${key} led nowhere: ${markup}`);
  }
  // The hostile key, one with a space, and one past 200 letters lead nowhere rather than
  // to a link the launcher would refuse.
  for (const key of [repository, "local:~/src/my notes", `github:owl/${"n".repeat(190)}`]) {
    const markup = openDrawer(keyedAs(key));
    assert.ok(!markup.includes("mycelium:repository/"), `${key} led on: ${markup}`);
  }
});

// What works in the hostile repository: two processes in its checkout, one in its tree.
const using = {
  key: `using:${repository}`,
  kind: "using",
  repository,
  processes: 3,
  memory: 1_300_000_000,
  cpu: 0.4,
  folders: [
    { path: checkout, processes: 2, memory: 300_000_000, cpu: 0.1 },
    { path: tree, processes: 1, memory: 1_000_000_000, cpu: 0.3 },
  ],
};

it("says in a row what works in its repository, its numbers held to their words", () => {
  const markup = rows(grove([...records, using]).repositories, NOW, "");
  const said = "40% of a core, 1.3\u00a0GB of memory";
  assert.ok(markup.includes(said), `the row didn't say what works in it: ${markup}`);
  assert.ok(
    !rows(grove().repositories, NOW, "").includes("of memory"),
    "an unmeasured row said a use",
  );
});

it("says in the drawer what works in each checkout and tree, and what isn't counted", () => {
  const markup = openDrawer([...records, using]);
  for (const said of [
    "Its processes take 40% of a core, 1.3\u00a0GB of memory, not counting builds the Nix daemon",
    "2\u00a0processes: 10% of a core, 300\u00a0MB of memory",
    "1\u00a0process: 30% of a core, 1.0\u00a0GB of memory",
  ]) {
    assert.ok(markup.includes(said), `the drawer didn't say "${said}": ${markup}`);
  }
  assert.ok(!openDrawer().includes("Its processes"), "an unmeasured drawer said a use");
});

it("says what works unread where the daemon's reading stopped, never as nothing working", () => {
  const unread = { ...using, processes: null, memory: null, cpu: null, folders: [] };
  const drawn = openDrawer([...records, unread]);
  // Said at a start as well as once stuck, so it claims no length of time.
  const lede = "What its processes take isn't known just now: the reading of them hasn't answered.";
  assert.ok(drawn.includes(lede), `the drawer didn't say it's unread: ${drawn}`);
  assert.ok(!drawn.includes("Its processes take"), `the drawer said a use it hasn't: ${drawn}`);
  const row = rows(grove([...records, unread]).repositories, NOW, "");
  assert.ok(row.includes("processes unread"), `the row didn't say it's unread: ${row}`);
});

const TREE_ROW = /<li class="tree /gu;
const MINUTE = 60_000;

/** A tree of the hostile repository named `name`, written `ago` before now. */
function treeNamed(name: string, ago: number, snapshot = false, named: string | null = null) {
  const path = `/home/ada/trees/${name}`;
  return {
    key: `tree:${path}`,
    kind: "tree",
    repository,
    checkout,
    path,
    snapshot,
    present: true,
    head: { branch: name, commit: "b2" },
    written_at: at(ago),
    change: named,
  };
}

// Fourteen trees written today, the oldest naming a change, and three snapshots among them.
const crowded = [
  ...records.filter((each) => each.kind !== "tree"),
  ...Array.from({ length: 13 }, (_, index) => treeNamed(`t${index}`, (index + 1) * MINUTE)),
  treeNamed("named", 50 * MINUTE, false, change),
  ...["s0", "s1", "s2"].map((name) => treeNamed(name, MINUTE / 2, true)),
];

/** How the crowded drawer is asked for: its seasons open, the cursor's trees, and the cells. */
interface Crowding {
  readonly open?: ReadonlySet<Season>;
  readonly keep?: readonly string[];
  readonly whole?: boolean;
  readonly workingIn?: readonly string[];
}

/** The crowded repository's drawer, asked for as `crowding` says. */
function crowdedDrawer(crowding: Crowding = {}): string {
  const working = (crowding.workingIn ?? []).map((name) => ({
    name: `wren-${name}`,
    title: name,
    standing: "moving",
    cwd: `/home/ada/trees/${name}`,
  }));
  const drawn = groveOf(told(crowded), cellsOf({ cells: working }), NOW);
  const opened = drawn?.repositories[0];
  assert.ok(drawn !== undefined && opened !== undefined, "the records drew no repository");
  const keep = new Set((crowding.keep ?? []).map((name) => `/home/ada/trees/${name}`));
  const whole = crowding.whole === true;
  const context = { now: NOW, home: "/home/ada", homes: () => undefined, keep, whole };
  return drawer(opened, drawn, context, crowding.open ?? new Set());
}

/** Whether `markup` lists the tree named `name`. */
const lists = (markup: string, name: string): boolean =>
  markup.includes(`data-path="/home/ada/trees/${name}"`);

it("lists twelve of a season's trees, those a change names first, and counts the rest", () => {
  const markup = crowdedDrawer();
  assert.equal(markup.match(TREE_ROW)?.length, 12, `the season didn't list twelve: ${markup}`);
  assert.ok(lists(markup, "named"), "the named tree wasn't listed");
  assert.ok(lists(markup, "t0") && !lists(markup, "t12"), "the newest weren't listed first");
  assert.ok(!lists(markup, "s0"), "a snapshot was listed");
  assert.ok(
    markup.includes(
      'And 5 more, 3\u00a0snapshots among them. <button type="button" class="unfold">',
    ),
    `the season didn't count what it left out: ${markup}`,
  );
});

it("lists a tree a cell works in, a snapshot among them, ahead of the newest", () => {
  const markup = crowdedDrawer({ workingIn: ["t12", "s2"] });
  assert.equal(markup.match(TREE_ROW)?.length, 12, `the season didn't list twelve: ${markup}`);
  assert.ok(lists(markup, "t12"), "the oldest tree, a cell working in it, wasn't listed");
  assert.ok(lists(markup, "s2"), "a snapshot a cell works in wasn't listed");
});

it("lists the trees the cursor stood on, even those the season leaves out", () => {
  const snapshot = crowdedDrawer({ keep: ["s1"] });
  assert.equal(snapshot.match(TREE_ROW)?.length, 13, `the cursor's tree wasn't added: ${snapshot}`);
  assert.ok(lists(snapshot, "s1"), "the snapshot the cursor is on wasn't listed");
  assert.ok(snapshot.includes("And 4 more"), `the count still held the cursor's: ${snapshot}`);
  const both = crowdedDrawer({ keep: ["s1", "t12"] });
  assert.equal(both.match(TREE_ROW)?.length, 14, `a tree the cursor left went: ${both}`);
  assert.ok(lists(both, "t12"), "the oldest tree the cursor stood on wasn't listed");
});

it("lists every tree once asked to, and counts none", () => {
  const markup = crowdedDrawer({ whole: true });
  assert.equal(markup.match(TREE_ROW)?.length, 17, `an opened season left trees out: ${markup}`);
  assert.ok(!markup.includes('class="unfold"'), `an opened season offered to list more: ${markup}`);
});

it("keeps the twelve in every season when an address opens only its tree's season", () => {
  const markup = crowdedDrawer({ open: new Set(["resting", "gone"]) });
  assert.equal(markup.match(TREE_ROW)?.length, 12, `opening a season lifted the cap: ${markup}`);
});
