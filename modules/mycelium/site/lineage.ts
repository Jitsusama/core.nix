// Who started each cell, from its forage (the node's `forage` kind): the
// cell that asked this machine's node to start it, else the orchestrator
// its presence names. Nothing of a forage crosses a link, so a node says
// it only of its own machine's cells, and the map grows one network out
// of another only from those until the cell itself carries it. From that,
// which network grows out of which, as a hypha branches from another.

import { type Held, object, text } from "../../local-sites/kit/records.ts";

/** A cell and the cell that started it, each by its address. */
interface Started {
  readonly cell: string;
  readonly by: string;
}

/** A network as the map draws it: where the node sets it, and the cells drawn in it. */
interface Network {
  readonly key: string;
  readonly name: string;
  readonly within: string | null;
  readonly cells: readonly string[];
}

/** What a network grew out of, the cells there that started its own, and how many they started. */
interface Grown {
  readonly from: string;
  readonly by: readonly string[];
  readonly cells: number;
}

const collate = new Intl.Collator("en-CA").compare;

/**
 * Who started each cell a forage is held for; none at all when the node
 * serves no forages, so a node too old to serve them reads as one that
 * says nothing of who started whom. A cell said to have started itself
 * names nobody.
 */
function startedOf(held: Held | undefined): Started[] | null {
  if (held === undefined) {
    return null;
  }
  const started: Started[] = [];
  for (const record of held.all()) {
    const cell = text(record, "cell");
    const by = text(object(record, "lineage") ?? {}, "started_by");
    if (cell && by && by !== cell) {
      started.push({ cell, by });
    }
  }
  return started;
}

/**
 * Which networks grow out of which, from who started their cells, each
 * drawn network's cells known by `hubOf` (a cell's address to the key of
 * the network it is drawn in, none for a receptor or a cell working
 * alone). A network the node sets within none grows out of the network
 * whose cells started more of its cells than any other, ties going to the
 * first by name. Where the node says a network sits always stands, a start
 * from inside the network or what sits in it counts for nothing, and the
 * strongest starts are settled first, so of two networks that started
 * each other's cells the one that started more is the trunk, and no
 * branch is grown that would loop back round to itself.
 */
function grownOf(
  networks: readonly Network[],
  hubOf: ReadonlyMap<string, string>,
  started: readonly Started[] | null,
): Map<string, Grown> {
  const named = new Map(networks.map((network) => [network.key, network]));
  const startedBy = new Map((started ?? []).map((start) => [start.cell, start.by]));
  const grown = new Map<string, Grown>();
  const within = (key: string): string | null =>
    named.get(key)?.within ?? grown.get(key)?.from ?? null;
  const sitsIn = (key: string, of: string): boolean => {
    const seen = new Set<string>();
    for (let at: string | null = key; at !== null && !seen.has(at); at = within(at)) {
      if (at === of) {
        return true;
      }
      seen.add(at);
    }
    return false;
  };
  const offered: { readonly key: string; readonly grown: Grown }[] = [];
  for (const network of networks) {
    if (network.within !== null) {
      continue;
    }
    const from = new Map<string, { readonly starters: Set<string>; readonly cells: Set<string> }>();
    for (const cell of network.cells) {
      const by = startedBy.get(cell);
      const hub = by === undefined ? undefined : hubOf.get(by);
      if (by === undefined || hub === undefined || !named.has(hub) || sitsIn(hub, network.key)) {
        continue;
      }
      const had = from.get(hub) ?? { starters: new Set<string>(), cells: new Set<string>() };
      had.starters.add(by);
      had.cells.add(cell);
      from.set(hub, had);
    }
    const nameOf = (key: string): string => named.get(key)?.name ?? key;
    const [best] = [...from].sort(
      ([one, ones], [other, others]) =>
        others.cells.size - ones.cells.size || collate(nameOf(one), nameOf(other)),
    );
    if (best) {
      const [key, { starters, cells }] = best;
      const by = [...starters].sort(collate);
      offered.push({ key: network.key, grown: { from: key, by, cells: cells.size } });
    }
  }
  offered.sort((one, other) => other.grown.cells - one.grown.cells || collate(one.key, other.key));
  for (const { key, grown: offer } of offered) {
    if (!sitsIn(offer.from, key)) {
      grown.set(key, offer);
    }
  }
  return grown;
}

export type { Grown, Network, Started };
export { grownOf, startedOf };
