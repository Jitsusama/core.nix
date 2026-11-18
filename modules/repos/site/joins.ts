// How the cells at work join one repository to another. A cell is in a network for each
// repository it works across, which the mesh names `repo.` and the repository's name as the
// mesh spells a group's part: lower-cased, each run of anything but letters and digits one
// hyphen, none at either end, as part() in the harness's lib/mesh/scope.ts spells it. So
// core.nix is repo.core-nix. A network names a repository by its name alone, so where two
// repositories are spelled alike it names neither. Nothing here touches the page, so each is
// tried in Node (tests/model.test.ts).

import type { Cell, Repository } from "./model.ts";

const REPOSITORY_NETWORK = "repo.";
const NOT_SPELLED = /[^a-z0-9]+/gu;
const EDGES = /^-|-$/gu;

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

/** Each other repository a cell in `repository` works across, by key, with the cell. */
function* across(
  repository: Repository,
  spellings: ReadonlyMap<string, readonly string[]>,
): Generator<readonly [string, string]> {
  for (const cell of repository.cells) {
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

/**
 * Every pair of repositories a cell's work ties, once whichever side said it, with the cells
 * that tie them: a tie stands when either side's cells name the other by a spelling only it has.
 */
function ties(repositories: readonly Repository[]): Tie[] {
  const spellings = bySpelling(repositories);
  const tied = new Map<string, { ends: readonly [string, string]; cells: Set<string> }>();
  for (const repository of repositories) {
    for (const [other, cell] of across(repository, spellings)) {
      const [one = "", two = ""] = [repository.key, other].sort(inOrder);
      const id = `${one}\n${two}`;
      const tie = tied.get(id) ?? { ends: [one, two], cells: new Set<string>() };
      tie.cells.add(cell);
      tied.set(id, tie);
    }
  }
  return [...tied.values()]
    .map(({ ends, cells }) => ({ ends, cells: [...cells].sort(inOrder) }))
    .sort((one, other) => inOrder(one.ends.join("\n"), other.ends.join("\n")));
}

export { joined, type Tie, ties };
