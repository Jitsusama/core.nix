// How the cells at work join one repository to another. A cell is in a network for each
// repository it works across, which the mesh names `repo.` and the repository's name as the
// mesh spells a group's part: lower-cased, each run of anything but letters and digits one
// hyphen, none at either end. So core.nix is repo.core-nix. Nothing here touches the page, so
// each is tried in Node (tests/model.test.ts).

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

/** Each repository with the others its cells' networks name too. */
function joined(repositories: readonly Repository[]): Repository[] {
  const bySpelling = new Map(repositories.map((each) => [spelled(each.name), each.key]));
  return repositories.map((repository) => {
    const others = new Set<string>();
    for (const cell of repository.cells) {
      for (const name of networked(cell)) {
        const key = bySpelling.get(name);
        if (key !== undefined && key !== repository.key) {
          others.add(key);
        }
      }
    }
    return { ...repository, joined: [...others].sort((one, other) => one.localeCompare(other)) };
  });
}

export { joined };
