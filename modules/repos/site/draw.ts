// The grove drawn: each repository as a tree whose crown holds its own trees
// as leaves, its changes as buds and the mycelium's threads at its roots;
// each as a row with its marks hung in a gutter; and the one open, beside
// the list, with its checkouts, changes and trees. Every mark carries a true
// count, and alike things are drawn alike, so one reads against the next.

import {
  type Cell,
  type Change,
  type Grove,
  type Repository,
  type Season,
  type Tree,
  plural,
  seasonOf,
  treeAddress,
} from "./model.ts";
import type { Homes } from "./homes.ts";
import { atWork, leads } from "./joins.ts";
import { usedBriefly, usedFully } from "./using.ts";
import { weighedBriefly, weighedFully, weightOfAll } from "./weight.ts";

/** What every part of the drawer reads besides its own: the time it's drawn at, the home
 * folder said as ~, and where the mycelium keeps each cell's home. */
interface Context {
  readonly now: number;
  readonly home: string | undefined;
  readonly homes: Homes;
}

/** What the open repository's drawer reads besides: the other repositories each cell works
 * across. */
interface Drawing extends Context {
  readonly across: (cell: Cell) => readonly Repository[];
}

const GITHUB_KEY = /^github:/u;

/** `words` made safe to put in markup. */
function escaped(words: string): string {
  return words
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

/** A name allowed to break after its dots and dashes, so a long one wraps whole rather than
 * being cut where two names alike would lose what tells them apart. */
function breakable(name: string): string {
  return escaped(name).replaceAll(/([.\-_])/gu, "$1<wbr>");
}

const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * An age, holding the moment it's told from, so the page can tell it again each minute without
 * drawing anything else.
 */
function when(at: number | undefined, now: number): string {
  const from = at === undefined ? "" : ` data-at="${at}"`;
  return `<span class="when"${from}>${escaped(ago(at, now))}</span>`;
}

/** How long ago `at` was, shortly, as a row's age says it. */
function ago(at: number | undefined, now: number): string {
  if (at === undefined) {
    return "";
  }
  const gap = Math.max(0, now - at);
  if (gap < HOUR) {
    return `${Math.max(1, Math.round(gap / MINUTE))}m`;
  }
  if (gap < DAY) {
    return `${Math.round(gap / HOUR)}h`;
  }
  return `${Math.round(gap / DAY)}d`;
}

/** A path with the home folder said as ~, as Joel types it. */
function tilde(path: string, home: string | undefined): string {
  return home !== undefined && (path === home || path.startsWith(`${home}/`))
    ? `~${path.slice(home.length)}`
    : path;
}

// A tree's season is told by its leaf's shape as well as its colour: filled
// while written today, outlined this week, a bare twig resting, a stump gone.
const LEAF: { readonly [season in Season]: string } = {
  today:
    '<path class="leaf today" d="M3 13C3 7 7 3 13 3c0 6-4 10-10 10z"/><path class="vein" d="M3 13 9 7"/>',
  week: '<path class="leaf week" d="M3 13C3 7 7 3 13 3c0 6-4 10-10 10z"/>',
  resting: '<path class="twig" d="M3 13 13 3M8 8l4 1M6 10l-1-4"/>',
  gone: '<ellipse class="stump" cx="8" cy="10" rx="5" ry="2.4"/><path class="stump" d="M3 10V7.5M13 10V7.5"/><ellipse class="rings" cx="8" cy="7.5" rx="5" ry="2.4"/>',
};

const SEASON_WORDS: { readonly [season in Season]: string } = {
  today: "written today",
  week: "written this week",
  resting: "resting",
  gone: "gone",
};

/** A tree's mark, its season said in words to a screen reader. */
function leaf(season: Season): string {
  return `<svg class="mk" viewBox="0 0 16 16" role="img" aria-label="${SEASON_WORDS[season]}">${LEAF[season]}</svg>`;
}

/** The season a repository stands in: its newest tree's. */
function standing(repository: Repository, now: number): Season {
  const { seasons } = repository;
  // A repository with no trees cut is as old as its checkouts were last written, never a
  // stump, which would say something was felled.
  if (repository.trees.length === 0) {
    return seasonOf(true, repository.writtenAt, now);
  }
  if (seasons.today > 0) {
    return "today";
  }
  if (seasons.week > 0) {
    return "week";
  }
  return seasons.resting > 0 ? "resting" : "gone";
}

// A change is a bud: dashed and closed while its checks run, open with a
// centre once they pass, split when one fails, ringed in the person's hue
// while it waits on a review. Shape tells as well as colour.
const BUD: { readonly [state in Change["checks"]]: string } = {
  passing:
    '<circle class="bud passing" cx="8" cy="8" r="5"/><circle class="heart" cx="8" cy="8" r="1.8"/>',
  pending: '<circle class="bud pending" cx="8" cy="8" r="5"/>',
  failing: '<path class="bud failing" d="M8 3a5 5 0 1 0 0 10M9.6 3.3a5 5 0 0 1 0 9.4"/>',
  none: '<circle class="bud none" cx="8" cy="8" r="5"/>',
};

const CHECKS_WORDS: { readonly [state in Change["checks"]]: string } = {
  passing: "its checks pass",
  pending: "its checks run",
  failing: "a check fails",
  none: "no checks run",
};

const REVIEW_WORDS: { readonly [state in Change["review"]]: string } = {
  approved: "approved",
  changes_asked: "changes asked for",
  waiting: "waiting on a review",
  none: "",
};

/** A change's bud, ringed while it waits on a review. */
function bud(change: Change): string {
  const ring = change.review === "waiting" ? '<circle class="asked" cx="8" cy="8" r="7.2"/>' : "";
  const words = [CHECKS_WORDS[change.checks], REVIEW_WORDS[change.review]]
    .filter(Boolean)
    .join(", ");
  return `<svg class="mk" viewBox="0 0 16 16" role="img" aria-label="${words}">${BUD[change.checks]}${ring}</svg>`;
}

/** The worst of a repository's changes, which its row's bud shows. */
function worstChange(changes: readonly Change[]): Change | undefined {
  const order: readonly Change["checks"][] = ["failing", "pending", "passing", "none"];
  return [...changes].sort(
    (one, other) => order.indexOf(one.checks) - order.indexOf(other.checks),
  )[0];
}

const GOLDEN = Math.PI * (3 - Math.sqrt(5));
const CROWN_LEAVES = 48;

/** A point `radius` from the crown's middle along `angle`, as a path's numbers. */
function point(radius: number, angle: number, cx: number, cy: number): readonly [number, number] {
  return [cx + radius * Math.cos(angle), cy + radius * Math.sin(angle)];
}

/**
 * One repository as a tree of the grove. Its crown's area grows with how
 * many trees it holds, against the largest in the grove; its leaves are the
 * trees written today (filled) and this week (outlined), set on a sunflower's
 * spiral so they keep their places; its buds are its changes; each thread
 * under the ground is a cell at work in it.
 */
function crown(repository: Repository, largest: number): string {
  const { today, week, resting, gone } = repository.seasons;
  const total = today + week + resting + gone;
  const cx = 40;
  const cy = 34;
  const radius = 9 + 21 * Math.sqrt(total / Math.max(1, largest));
  const leaves: string[] = [];
  const shown = Math.min(today + week, CROWN_LEAVES);
  for (let index = 0; index < shown; index += 1) {
    const out = (radius - 4) * Math.sqrt((index + 0.5) / Math.max(shown, 6));
    const [x, y] = point(out, index * GOLDEN, cx, cy);
    const turn = Math.round((index * GOLDEN * 180) / Math.PI) % 360;
    const kind = index < today ? "today" : "week";
    leaves.push(
      `<ellipse class="leaf ${kind}" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" rx="3.2" ry="1.7" transform="rotate(${turn} ${x.toFixed(1)} ${y.toFixed(1)})"/>`,
    );
  }
  const buds = repository.changes.slice(0, 7).map((change, index, all) => {
    const angle = -Math.PI / 2 + (index - (all.length - 1) / 2) * 0.42;
    const [x, y] = point(radius + 1.5, angle, cx, cy);
    return `<circle class="bud ${change.checks}" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="2.4"/>`;
  });
  const stumps = Array.from({ length: Math.min(gone, 4) }, (_, index) => {
    const x = index % 2 === 0 ? 28 - index * 3 : 52 + index * 3;
    return `<ellipse class="stump" cx="${x}" cy="76" rx="2.6" ry="1.2"/>`;
  });
  const threads = atWork(repository.cells)
    .slice(0, 6)
    .map((_, index, all) => {
      const spread = (index - (all.length - 1) / 2) * 9;
      return `<path class="thread" d="M40 78c0 6 ${(spread / 2).toFixed(1)} 8 ${spread.toFixed(1)} 12"/>`;
    });
  const top = cy + radius;
  return `<svg class="crown" viewBox="0 0 80 92" aria-hidden="true">
    <path class="trunk" d="M40 78V${Math.min(70, top).toFixed(1)}"/>
    <circle class="canopy" cx="${cx}" cy="${cy}" r="${radius.toFixed(1)}"/>
    ${leaves.join("")}${buds.join("")}
    <path class="ground" d="M8 78h64"/>${stumps.join("")}${threads.join("")}
  </svg>`;
}

/** What a repository's tree in the grove says to a screen reader. */
function groveWords(repository: Repository): string {
  const { today, week } = repository.seasons;
  const parts = [size(repository)];
  if (today + week > 0) {
    parts.push(`${today} written today, ${week} more this week`);
  }
  if (repository.changes.length > 0) {
    parts.push(plural(repository.changes.length, "change"));
  }
  const cells = atWork(repository.cells).length;
  if (cells > 0) {
    parts.push(`${plural(cells, "cell")} at work`);
  }
  return parts.join(", ");
}

/** The grove: every repository as a tree, the cursor's lit by the lantern. */
function groveOfTrees(grove: Grove): string {
  const largest = Math.max(1, ...grove.repositories.map((each) => each.trees.length));
  return grove.repositories
    .map(
      (repository) => `<a class="stand" role="option" id="stand-${escaped(repository.key)}"
      data-key="${escaped(repository.key)}" data-home="code/${escaped(repository.address)}"
      href="${escaped(repository.address)}" tabindex="-1" aria-label="${escaped(repository.name)}: ${escaped(groveWords(repository))}">
      ${crown(repository, largest)}<span class="nm">${breakable(repository.name)}</span></a>`,
    )
    .join("");
}

/** How big it is: its trees, or its checkouts when none was cut from them. */
function size(repository: Repository): string {
  return repository.trees.length > 0
    ? plural(repository.trees.length, "tree")
    : plural(repository.checkouts.length, "checkout");
}

/** How it grows: its trees written today, else this week, else nothing. */
function growth(repository: Repository): string {
  const { today, week } = repository.seasons;
  if (today > 0) {
    return `${today} today`;
  }
  return week > 0 ? `${week} this week` : "";
}

/** The words of a row after its name: its size, how it grows, its changes, its weight and what
 * works in it. */
function rowMeta(repository: Repository): string {
  const parts = [
    size(repository),
    growth(repository),
    repository.changes.length > 0 ? plural(repository.changes.length, "change") : "",
    weighedBriefly(weightOfAll(repository.checkouts.map((each) => each.weight))),
    usedBriefly(repository.using),
  ].filter(Boolean);
  return parts.map(counted).join(" · ");
}

/**
 * `words` with each number held to the word after it, so a row wraps "6.0 MB" whole. Each part
 * of a line goes alone, so a part ending in a digit isn't held to the separator after it.
 */
function counted(words: string): string {
  return words.replace(/(\d) /gu, "$1\u00a0");
}

/** One repository as a row: its marks in the gutter, then its name and facts. */
function row(repository: Repository, now: number): string {
  const change = worstChange(repository.changes);
  const cells = atWork(repository.cells).length;
  const season = standing(repository, now);
  return `<a class="row" role="option" id="row-${escaped(repository.key)}" data-key="${escaped(repository.key)}"
    data-home="code/${escaped(repository.address)}" href="${escaped(repository.address)}" tabindex="-1">
    <span class="g-season">${leaf(season)}</span>
    <span class="g-change">${change === undefined ? "" : bud(change)}</span>
    <span class="t-cell"><span class="t" translate="no">${escaped(repository.name)}</span>
      <span class="meta">${escaped(rowMeta(repository))}</span></span>
    <span class="cells">${cells > 0 ? `<svg class="mk" viewBox="0 0 16 16" aria-hidden="true"><path class="thread" d="M8 2v5c0 3-3 4-5 7M8 7c0 3 3 4 5 7"/></svg>${counted(plural(cells, "cell"))}` : ""}</span>
    ${when(repository.writtenAt, now)}
    <span class="row-keys" aria-hidden="true"><kbd>Return</kbd> step in · <kbd>o</kbd> open</span>
  </a>`;
}

/** The repositories as rows, or what the list holds when `query`, or the place, finds none. */
function rows(repositories: readonly Repository[], now: number, query: string): string {
  if (repositories.length === 0) {
    const none =
      query === ""
        ? "Nothing grew today; the tide of work is out."
        : `No repository is named like ${escaped(query)}.`;
    return `<p class="quiet">${none}</p>`;
  }
  return repositories.map((each) => row(each, now)).join("");
}

/** A cell's name, linked to its home when the mycelium's homes name it. */
function cellName(cell: Cell, homes: Homes): string {
  const home = homes(cell.name);
  const name = `<code class="cell-name">${escaped(cell.name)}</code>`;
  // Its key names the link apart from another cell's to one home, so a redraw keeps focus on it.
  const key = `data-key="cell:${escaped(cell.name)}"`;
  return home === undefined ? name : `<a href="../${escaped(home)}" ${key}>${name}</a>`;
}

/**
 * The other repositories a cell works across, each linked to where Code shows it, so the keys
 * follow its work on from the folder it's in. Each link's key names it apart, so a redraw keeps
 * focus on it.
 */
function cellAcross(cell: Cell, others: readonly Repository[]): string {
  if (others.length === 0) {
    return "";
  }
  const links = others.map(
    (each) =>
      `<a href="${escaped(each.address)}" data-key="across:${escaped(cell.name)}:${escaped(each.key)}">${escaped(each.name)}</a>`,
  );
  return ` <span class="cell-across">also in ${links.join(", ")}</span>`;
}

/** The cells at work in a folder, as a line under it. */
function cellsLine(cells: readonly Cell[], drawing: Drawing): string {
  const working = atWork(cells);
  if (working.length === 0) {
    return "";
  }
  const said = working.map(
    (cell) =>
      `${cellName(cell, drawing.homes)} <span class="cell-title">${escaped(cell.title)}</span>${cellAcross(cell, drawing.across(cell))}`,
  );
  return `<span class="at-work">${said.join("; ")}</span>`;
}

/** What works in a tree, as its own line, where anything does. */
function usedLine(use: Tree["using"]): string {
  const said = usedFully(use);
  return said === "" ? "" : `<span class="meta">${escaped(counted(said))}</span>`;
}

/** One tree in the drawer: its leaf, its branch, its folder, what works in it and its cells. */
function treeRow(tree: Tree, cutIn: string | undefined, drawing: Drawing): string {
  const { now, home } = drawing;
  // A tree in the folder the drawer has already named says only its own name, and none at all
  // when that name only repeats its branch, as the harness names the trees it cuts.
  const own =
    cutIn !== undefined && tree.path.startsWith(`${cutIn}/`)
      ? tree.path.slice(cutIn.length + 1)
      : undefined;
  // A snapshot's folder is named for its commit, which its row already says.
  const repeats =
    own !== undefined &&
    ((tree.branch !== undefined && own.endsWith(tree.branch.replace(/\//gu, "-"))) ||
      (tree.snapshot && tree.commit !== undefined && own.includes(tree.commit)));
  const where = repeats
    ? ""
    : `<span class="path" translate="no">${escaped(own ?? tilde(tree.path, home))}</span>`;
  const branch = tree.branch ?? (tree.commit === undefined ? "" : tree.commit.slice(0, 10));
  const change =
    tree.change === undefined
      ? ""
      : `<span class="tree-change">${bud(tree.change)}${tree.change.number === undefined ? "" : `#${tree.change.number}`}</span>`;
  return `<li class="tree ${tree.season}" data-path="${escaped(tree.path)}" data-home="code/${escaped(treeAddress(tree.path))}" tabindex="-1">
    <span class="g-season">${leaf(tree.season)}</span>
    <span class="tree-body"><span class="branch" translate="no">${escaped(branch)}${tree.snapshot ? ' <span class="snap">snapshot</span>' : ""}</span>
    ${where}${usedLine(tree.using)}${cellsLine(tree.cells, drawing)}</span>
    ${change}${when(tree.writtenAt, now)}
  </li>`;
}

const GROUPS: readonly [Season, string][] = [
  ["today", "Written Today"],
  ["week", "This Week"],
  ["resting", "Resting"],
  ["gone", "Gone"],
];

/** A repository's trees, grouped by season; resting and gone fold shut until opened. */
function treeGroups(repository: Repository, drawing: Drawing, open: ReadonlySet<Season>): string {
  return GROUPS.map(([season, title]) => {
    const trees = repository.trees.filter((each) => each.season === season);
    if (trees.length === 0) {
      return "";
    }
    const folds = season === "resting" || season === "gone";
    const shown = !folds || open.has(season);
    const head = `<h3 class="group"><span class="group-title">${title}</span> <span class="n">${trees.length.toLocaleString("en-CA")}</span></h3>`;
    const list = shown
      ? `<ul class="trees">${trees.map((each) => treeRow(each, repository.cutIn, drawing)).join("")}</ul>`
      : `<p class="folded">Folded; press <kbd>z</kbd> to open ${plural(trees.length, "tree")}.</p>`;
    return `<section class="season ${season}" data-season="${season}">${head}${list}</section>`;
  }).join("");
}

/** A change in the drawer: its bud, number, title and where it stands. */
function changeRow(change: Change): string {
  const words = [
    CHECKS_WORDS[change.checks],
    REVIEW_WORDS[change.review],
    change.draft ? "a draft" : "",
  ]
    .filter(Boolean)
    .join(", ");
  const number = change.number === undefined ? "" : `<span class="num">#${change.number}</span> `;
  const title =
    change.url === undefined
      ? escaped(change.title)
      : `<a href="${escaped(change.url)}">${escaped(change.title)}</a>`;
  const failed =
    change.failed.length > 0
      ? `<span class="failed">${escaped(change.failed.join(", "))}</span>`
      : "";
  return `<li class="change ${change.checks}"><span class="g-change">${bud(change)}</span>
    <span class="change-body">${number}${title}<span class="stands">${escaped(words)}</span>${failed}</span></li>`;
}

/** What works in a repository, as a sentence following its lede's, saying what it leaves out;
 * nothing where nothing works. */
function workingIn(repository: Repository): string {
  const said = usedBriefly(repository.using);
  return said === ""
    ? ""
    : ` ${counted(`Its processes take ${said}, not counting builds the Nix daemon runs for it.`)}`;
}

/** The repository open beside the list. */
function drawer(
  repository: Repository,
  grove: Grove,
  context: Context,
  open: ReadonlySet<Season>,
): string {
  const { home } = context;
  const found = (key: string): Repository | undefined =>
    grove.repositories.find((each) => each.key === key);
  const leading = leads(repository, grove.repositories);
  const drawing: Drawing = {
    ...context,
    across: (cell) =>
      (leading.get(cell.name) ?? [])
        .map(found)
        .filter((each): each is Repository => each !== undefined),
  };
  // Where its remote keeps it, as the record keys it: GitHub's owner and name, or the host
  // and folder of any other remote, so no place is made up for one GitHub doesn't hold.
  const where = repository.key.replace(GITHUB_KEY, "");
  const checkouts = repository.checkouts
    .map((each) => {
      const at = each.bare ? "bare" : (each.branch ?? "");
      const drift = [
        each.behind ? `${each.behind} behind` : "",
        each.ahead ? `${each.ahead} ahead` : "",
      ].filter(Boolean);
      const weight = weighedFully(each.weight);
      const used = usedFully(each.using);
      return `<li class="checkout"><span class="path" translate="no">${escaped(tilde(each.path, home))}</span>
        <span class="meta">${escaped([at, ...drift, weight, used].filter(Boolean).map(counted).join(" · "))}</span>${cellsLine(each.cells, drawing)}</li>`;
    })
    .join("");
  const joined = repository.joined
    .map(found)
    .filter((each): each is Repository => each !== undefined);
  const threads =
    joined.length === 0
      ? ""
      : `<p class="joined">Joined through the mycelium to ${joined
          .map((each) => `<a href="${escaped(each.address)}">${escaped(each.name)}</a>`)
          .join(", ")}, by the cells at work across them.</p>`;
  const changes =
    repository.changes.length === 0
      ? ""
      : `<section class="changes"><h3 class="group"><span class="group-title">Changes</span> <span class="n">${repository.changes.length}</span></h3>
        <ul>${repository.changes.map(changeRow).join("")}</ul></section>`;
  return `<article class="open" data-home="code/${escaped(repository.address)}">
    <p class="eyebrow">repository · <span translate="no">${escaped(where)}</span></p>
    <h2 class="open-title" id="open-title" tabindex="-1">${escaped(repository.name)}</h2>
    <p class="open-lede">${escaped(repository.trunk === undefined ? "No trunk named" : `Its trunk is ${repository.trunk}`)}; ${escaped(plural(repository.trees.length, "tree"))} cut from ${escaped(plural(repository.checkouts.length, "checkout"))}${repository.cutIn === undefined ? "" : `, most in <span class="path" translate="no">${escaped(tilde(repository.cutIn, home))}</span>`}.${escaped(workingIn(repository))}</p>
    ${threads}
    <ul class="checkouts">${checkouts}</ul>
    ${changes}
    ${treeGroups(repository, drawing, open)}
  </article>`;
}

export { ago, type Context, drawer, escaped, groveOfTrees, rows, tilde };
