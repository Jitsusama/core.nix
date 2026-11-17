// Whether the map reaches every cell the node holds, judged from what the
// drawing made of a reading (view.js's model) beside the reading itself.
// Joel found cells the map drew as working alone after their audit was
// reported done, so this says each such miss by name: a cell in an
// orchestration, or a receptor, drawn as working alone; a network drawn
// with no name but its address, judged by the name the drawing gives it;
// and a cell the map holds in no network it draws, or under one whose
// parents it doesn't, which no step-in, fold or find can then reach. Every
// cell the map holds in a drawn network can be found by its address and
// opened, so holding it there is reaching it.
//
// Opened with `?reach` in its address, the page writes its answer on the
// root element as `data-reach` once it has drawn each reading, for the map's
// build test and `mycelium-map-check` to read. Otherwise it does nothing.

/** Why a cell fails: drawn alone, in an unnamed network, or held nowhere. */
type Why = "alone" | "unnamed network" | "unreachable";

/** One cell the map misses, with whom to tell. */
type Failure = {
  readonly cell: string;
  readonly orchestration: string | null;
  readonly owner: string;
  readonly why: Why;
};

/** The map's answer for one reading. */
type Reach = {
  readonly at: string;
  readonly cells: number;
  readonly count: number;
  readonly failures: readonly Failure[];
};

/** A network as the node prints it, of what this reads. */
type Network = {
  readonly name: string;
  readonly shape: string;
};

/** A cell as the node prints it, of what this reads. */
type Cell = {
  readonly name: string;
  readonly type?: string;
  readonly networks?: readonly string[];
};

/** A reading as site.ts hands it to the drawing. */
type Reading = {
  readonly at: string;
  readonly networks: readonly Network[];
  readonly cells: readonly Cell[];
};

/** A network as the drawing holds it, of what this reads. */
type Hub = {
  readonly key: string;
  readonly name: string;
  readonly within: string | null;
  readonly alone?: boolean;
  readonly lead?: { readonly address: string } | null;
};

/** A cell as the drawing holds it, of what this reads. */
type Drawn = {
  readonly address: string;
  readonly hub: Hub | null;
  readonly loose?: Hub;
};

/** What the drawing made of a reading, of what this reads. */
type Model = {
  readonly hubs: ReadonlyMap<string, Hub>;
  readonly byAddress: ReadonlyMap<string, Drawn>;
};

const ORCHESTRATION = "orchestration.";

/** The address an orchestration's network is named for. */
function addressOf(network: string): string {
  return network.slice(ORCHESTRATION.length);
}

/**
 * The orchestration `cell` belongs to, among `orchestrations`: one whose
 * network it hears, else the one its quest's network sits under, as
 * `quest.wcjqsg.85pkb2` sits under wcjqsg's.
 */
function belongsTo(cell: Cell, orchestrations: ReadonlySet<string>): string | null {
  const networks = cell.networks ?? [];
  const heard = networks.find((name) => orchestrations.has(name));
  const above = networks
    .filter((name) => name.startsWith("quest."))
    .flatMap((name) => name.split(".").slice(1))
    .map((address) => `${ORCHESTRATION}${address}`)
    .find((each) => orchestrations.has(each));
  return heard ?? above ?? null;
}

/** Whether the drawing names `hub` by nothing but its network's address. */
function unnamed(hub: Hub): boolean {
  const name = hub.name.trim().toLowerCase();
  return ["", hub.key.toLowerCase(), addressOf(hub.key).toLowerCase()].includes(name);
}

/** Whether `hub` is one `model` draws, under parents it draws too. */
function drawnIn(hub: Hub | undefined, model: Model): boolean {
  const seen = new Set<string>();
  for (let at = hub; at !== undefined; at = at.within ? model.hubs.get(at.within) : undefined) {
    if (seen.has(at.key) || model.hubs.get(at.key) !== at) {
      return false;
    }
    if (!at.within) {
      return true;
    }
    seen.add(at.key);
  }
  return false;
}

/** Why `model` misses `cell`, of an orchestration or none, drawn as `drawn`; null when it doesn't. */
function missed(
  cell: Cell,
  orchestration: string | null,
  drawn: Drawn | undefined,
  model: Model,
): Why | null {
  if (!(drawn && drawnIn(drawn.hub ?? drawn.loose, model))) {
    return "unreachable";
  }
  const alone = drawn.hub === null && drawn.loose?.alone === true;
  if (alone && (orchestration !== null || cell.type === "receptor")) {
    return "alone";
  }
  return drawn.hub && unnamed(drawn.hub) ? "unnamed network" : null;
}

/** The orchestration `drawn` sits in on the map, else the one `cell` belongs to. */
function orchestrationOf(
  cell: Cell,
  drawn: Drawn | undefined,
  orchestrations: ReadonlySet<string>,
): string | null {
  const key = drawn?.hub?.key;
  return key && orchestrations.has(key) ? key : belongsTo(cell, orchestrations);
}

/** Each cell of `reading` that `model`, the map drawn of it, misses. */
function reachOf(reading: Reading, model: Model): Reach {
  const orchestrations = new Set(
    reading.networks
      .filter((network) => network.shape === "orchestration")
      .map((network) => network.name),
  );
  // A cell moving between machines can come from both for a while; the
  // drawing holds it once, under its one address, so it's judged once.
  const cells = [...new Map(reading.cells.map((cell) => [cell.name, cell])).values()];
  const failures: Failure[] = [];
  for (const cell of cells) {
    const drawn = model.byAddress.get(cell.name);
    const orchestration = orchestrationOf(cell, drawn, orchestrations);
    const why = missed(cell, orchestration, drawn, model);
    if (why !== null) {
      failures.push({
        cell: cell.name,
        orchestration: orchestration ? addressOf(orchestration) : null,
        owner: (orchestration && model.hubs.get(orchestration)?.lead?.address) || cell.name,
        why,
      });
    }
  }
  return { at: reading.at, cells: cells.length, count: failures.length, failures };
}

/** Whether the page was opened to say its reach. */
function asked(search: string): boolean {
  return new URLSearchParams(search).has("reach");
}

/** Says each reading's reach on `root`, as the drawing models it. */
function sayReach(root: HTMLElement, events: EventTarget): void {
  events.addEventListener("mycelium:modelled", (event) => {
    const { now, model } = (event as CustomEvent<{ now: Reading; model: Model }>).detail;
    root.dataset["reach"] = JSON.stringify(reachOf(now, model));
  });
}

export type { Failure, Model, Reach, Reading };
export { asked, reachOf, sayReach };
