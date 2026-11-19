// How the cells at work join one repository to another. A cell is in a network for each
// repository it works across, which the mesh names `repo.` and the repository's name as the
// mesh spells a group's part: lower-cased, each run of anything but letters and digits one
// hyphen, none at either end, as part() in the harness's lib/mesh/scope.ts spells it. So
// core.nix is repo.core-nix. A network names a repository by its name alone, so where two
// repositories are spelled alike it names neither. Nothing here touches the page, so each is
// tried in Node: the joins in tests/model.test.ts, and the ties in tests/map.test.ts.

import type { Cell, Repository } from "./model.ts";

const REPOSITORY_NETWORK = "repo.";
const NOT_SPELLED = /[^a-z0-9]+/gu;
const EDGES = /^-|-$/gu;
// What keeps a tie's two keys apart in its name: a local repository's key is its folder, which
// may hold a line break but never a null.
const APART = "\u0000";

/** The cells at work, rather than quiet or resting, of `cells`. */
function atWork(cells: readonly Cell[]): readonly Cell[] {
  return cells.filter((cell) => cell.standing !== "quiet" && cell.standing !== "dormant");
}

/** A repository's name as the mesh spells it in a network's name. */
function spelled(name: string): string {
  return name.toLowerCase().replace(NOT_SPELLED, "-").replace(EDGES, "");
}

/** The repositories named by a cell's networks, as the mesh spells them. */
function networked(cell: Cell): readonly string[] {
  return cell.networks
    .filter((each) => each.startsWith(REPOSITORY_NETWORK))
    .map((each) => each.slice(REPOSITORY_NETWORK.length));
}

/** The repositories by how the mesh spells each, each spelling with every repository it names. */
function bySpelling(repositories: readonly Repository[]): Map<string, string[]> {
  const spellings = new Map<string, string[]>();
  for (const each of repositories) {
    const spelling = spelled(each.name);
    spellings.set(spelling, [...(spellings.get(spelling) ?? []), each.key]);
  }
  return spellings;
}

/** Two repositories a cell's work ties together, by key in order, and every such cell by name. */
interface Tie {
  readonly ends: readonly [string, string];
  readonly cells: readonly string[];
}

const inOrder = (one: string, other: string): number => one.localeCompare(other);
// Which of a tie's ends comes first goes by the keys' own characters, since a comparison of
// words calls a folder written composed and the same written decomposed equal.
const byUnits = (one: string, other: string): number => Number(one > other) - Number(one < other);

/**
 * Each other repository a cell at work in `repository` works across, by key, with the cell; a
 * cell quiet or resting gets nothing done between them, as the page counts it nowhere else.
 */
function* across(
  repository: Repository,
  spellings: ReadonlyMap<string, readonly string[]>,
): Generator<readonly [string, string]> {
  for (const cell of atWork(repository.cells)) {
    for (const name of networked(cell)) {
      const [key, ...alike] = name === "" ? [] : (spellings.get(name) ?? []);
      if (key !== undefined && alike.length === 0 && key !== repository.key) {
        yield [key, cell.name];
      }
    }
  }
}

/** Each repository with the others its cells' networks name too. */
function joined(repositories: readonly Repository[]): Repository[] {
  const spellings = bySpelling(repositories);
  return repositories.map((repository) => {
    const others = new Set([...across(repository, spellings)].map(([key]) => key));
    return { ...repository, joined: [...others].sort(inOrder) };
  });
}

/** Each cell at work in `repository` by name, with the other repositories it works across. */
function leads(
  repository: Repository,
  repositories: readonly Repository[],
): Map<string, readonly string[]> {
  const found = new Map<string, readonly string[]>();
  for (const [key, cell] of across(repository, bySpelling(repositories))) {
    const known = found.get(cell) ?? [];
    found.set(cell, known.includes(key) ? known : [...known, key].sort(inOrder));
  }
  return found;
}

/**
 * Every pair of repositories a cell's work ties, once whichever side said it, with the cells
 * that tie them: a tie stands when either side's cells name the other by a spelling only it has.
 */
function ties(repositories: readonly Repository[]): Tie[] {
  const spellings = bySpelling(repositories);
  const tied = new Map<string, { ends: readonly [string, string]; cells: Set<string> }>();
  for (const repository of repositories) {
    for (const [other, cell] of across(repository, spellings)) {
      const [one = "", two = ""] = [repository.key, other].sort(byUnits);
      const id = `${one}${APART}${two}`;
      const tie = tied.get(id) ?? { ends: [one, two], cells: new Set<string>() };
      tie.cells.add(cell);
      tied.set(id, tie);
    }
  }
  return (
    [...tied.values()]
      .map(({ ends, cells }) => ({ ends, cells: [...cells].sort(inOrder) }))
      // By each end in turn, since a comparison of the two joined skips the null between them.
      .sort(
        (one, other) => inOrder(one.ends[0], other.ends[0]) || inOrder(one.ends[1], other.ends[1]),
      )
  );
}

export { atWork, joined, leads, type Tie, ties };
