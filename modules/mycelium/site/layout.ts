// Where the map puts everything, in its own units, which zoom scales: the
// machines in a column at the middle, each network's hub round them in a
// share of a ring as wide as its family, its branches further out, the
// receptors above and the builders below, and each cell packed round its
// hub like a seed in a sunflower. Places never move while the page is
// open: a cell keeps its seed, a disc keeps room for more cells than it
// holds, and an ended cell's seed waits for the next to arrive.

/** A place on the map, in its units. */
export type Point = readonly [number, number];

/** A cell's membrane, in map units. */
export const R = 11;
/** A network's hub. */
export const HUB_R = 21;
/** A machine's mark, half its height. */
export const MACHINE_R = 19;
/** A sunflower's spacing: seeds about 29 apart. */
export const SEED = 16.5;
/** Seeds left empty round a hub, so it has room. */
export const HOLE = 5;
/** Between two neighbouring discs. */
export const GAP = 54;
/** Between a hub's disc and a branch's, so each colony reads as its own. */
export const BRANCH_GAP = 2.4 * GAP;
/** Between the furthest network and the receptors at the edge. */
export const EDGE_GAP = 2 * GAP;
/** The least distance from the machines to a hub. */
export const RING_MIN = 190;
/** Between two machines in the column, and two builders under the ring. */
export const MACHINE_STEP = 96;
/** The least distance between machines, as a share of the ring. */
export const MACHINE_SPREAD = 0.4;
/** Networks on a machine, past which its cords bundle. */
export const CORD_BUNDLE_OVER = 24;
/** How far out a bundling machine sits, as a share of the ring. */
export const MACHINE_ORBIT = 0.3;
/** The turn between one seed and the next, so none falls behind another. */
const GOLDEN = Math.PI * (3 - Math.sqrt(5));

/** A disc keeps room in steps of this many cells. */
const ROOM_STEP = 8;
/** Room kept past what a disc holds, as a share of it. */
const ROOM_SPARE = 1.15;
/** Room kept past that, whatever a disc holds. */
const ROOM_MORE = 3;

/** A cell as the layout reads it: what it is, when it came and where it hangs. */
export interface Seeded {
  readonly id: string;
  readonly since?: string | null;
  readonly hub?: { readonly key: string } | null;
  readonly loose?: { readonly key: string } | null;
}

/** A network's hub as the layout reads it. */
export interface Hub {
  readonly key: string;
  readonly name: string;
  /** The machine most of its cells run on, whose arc it sits in. */
  readonly machine?: string | null;
  /** The network it branches from, or none for one on the ring. */
  readonly within?: string | null;
  /** The receptors', standing at the edge. */
  readonly edge?: boolean;
  /** Cells working alone, which stand after the networks. */
  readonly alone?: boolean;
  readonly children: readonly Hub[];
  readonly cells: readonly Seeded[];
}

/** What the layout reads of a reading. */
export interface Realm {
  readonly hubs: ReadonlyMap<string, Hub>;
  readonly cells: readonly Seeded[];
  readonly byId: ReadonlyMap<string, unknown>;
  readonly machines: readonly { readonly name: string }[];
  readonly builders: readonly { readonly name: string }[];
}

/** Each hub's seeds, in the order its cells first came: an id, or null where one ended. */
export type Seeds = Map<string, (string | null)[]>;

/** Where everything sits. */
export interface Layout {
  readonly at: ReadonlyMap<string, Point>;
  /** The way each hub faces out from the middle, in radians. */
  readonly angle: ReadonlyMap<string, number>;
  /** How far each hub's disc reaches, with the room it keeps. */
  readonly disc: ReadonlyMap<string, number>;
  readonly machines: ReadonlyMap<string, Point>;
  readonly builders: ReadonlyMap<string, Point>;
  /** How far the top hubs stand from the middle. */
  readonly ring: number;
}

const collate = new Intl.Collator("en-CA").compare;

/**
 * Sows a reading into `seeds`: a cell that left its hub frees its seed, and
 * a cell new to a hub takes its first free seed, or one past the last, the
 * oldest first, so every other cell keeps its place.
 */
export const sow = (seeds: Seeds, realm: Realm): void => {
  const where = new Map<string, string | undefined>();
  for (const s of realm.cells) {
    where.set(s.id, (s.hub ?? s.loose)?.key);
  }
  for (const [key, list] of seeds) {
    for (let i = 0; i < list.length; i++) {
      const id = list[i];
      if (id && where.get(id) !== key) {
        list[i] = null;
      }
    }
  }
  for (const h of realm.hubs.values()) {
    const list = seeds.get(h.key) ?? [];
    const have = new Set(list);
    const fresh = h.cells
      .filter((s) => !have.has(s.id))
      .sort((a, b) => (a.since ?? "").localeCompare(b.since ?? "") || a.id.localeCompare(b.id));
    for (const s of fresh) {
      const empty = list.indexOf(null);
      if (empty >= 0) {
        list[empty] = s.id;
      } else {
        list.push(s.id);
      }
    }
    seeds.set(h.key, list);
  }
};

/**
 * The room a disc keeps for `n` seeds, in steps, so a cell arriving doesn't
 * move the hubs round it.
 */
export const roomFor = (n: number): number =>
  Math.max(ROOM_STEP, Math.ceil((n * ROOM_SPARE + ROOM_MORE) / ROOM_STEP) * ROOM_STEP);

/** How far a disc keeping `room` seeds reaches from its hub. */
export const discRadius = (room: number): number => SEED * Math.sqrt(room + HOLE) + R;

/** Where seed `i` sits from its hub. */
export const seedAt = (i: number): Point => {
  const k = i + HOLE;
  const r = SEED * Math.sqrt(k);
  const a = k * GOLDEN;
  return [r * Math.cos(a), r * Math.sin(a)];
};

const roomOf = (realm: Realm, seeds: Seeds): Map<string, number> =>
  new Map([...realm.hubs.keys()].map((key) => [key, roomFor(seeds.get(key)?.length ?? 0)]));

/**
 * What the layout of a reading depends on, which is the same while nothing
 * would move. It is JSON, since the node names the networks, and a name
 * holding a separator must not make two realms read as one.
 */
export const layoutKeyOf = (realm: Realm, seeds: Seeds): string => {
  const room = [...roomOf(realm, seeds)].map(([key, r]) => {
    const h = realm.hubs.get(key);
    return [key, r, h?.machine ?? "", h?.within ?? ""];
  });
  const machines = realm.machines.map((x) => x.name);
  const builders = realm.builders.map((b) => b.name);
  return JSON.stringify([room, machines, builders]);
};

/** A hub's family's width along the ring: its own disc, or its branches side by side. */
const familyWidthOf = (disc: ReadonlyMap<string, number>) => {
  const familyWidth = (h: Hub): number => {
    const kids = h.children.reduce((sum, c) => sum + familyWidth(c) + GAP, 0);
    return Math.max(2 * (disc.get(h.key) ?? 0), kids - GAP);
  };
  return familyWidth;
};

/** Where every hub stands, and the way it faces out. */
interface Placing {
  readonly at: Map<string, Point>;
  readonly angle: Map<string, number>;
}

/** Places a hub and, fanned out beyond it, its branches and theirs. */
const placerOf = (disc: ReadonlyMap<string, number>, placing: Placing) => {
  const familyWidth = familyWidthOf(disc);
  const radius = (h: Hub): number => disc.get(h.key) ?? 0;
  const place = (h: Hub, [x, y]: Point, a: number): void => {
    placing.at.set(h.key, [x, y]);
    placing.angle.set(h.key, a);
    const kids = [...h.children].sort((p, q) => p.name.localeCompare(q.name));
    if (!kids.length) {
      return;
    }
    const reach = radius(h) + BRANCH_GAP + Math.max(...kids.map(radius));
    const spans = kids.map((c) => (familyWidth(c) + GAP) / reach);
    let from = a - spans.reduce((p, q) => p + q, 0) / 2;
    kids.forEach((c, i) => {
      const span = spans[i] ?? 0;
      const ca = from + span / 2;
      from += span;
      const d = radius(h) + BRANCH_GAP + radius(c);
      place(c, [x + Math.cos(ca) * d, y + Math.sin(ca) * d], ca);
    });
  };
  return place;
};

/** The networks on the ring, in turn: the top machine's first, those working alone last. */
const topsOf = (realm: Realm): Hub[] => {
  const machineAt = new Map(realm.machines.map((x, i) => [x.name, i]));
  const arc = (h: Hub): number => machineAt.get(h.machine ?? "") ?? realm.machines.length;
  return [...realm.hubs.values()]
    .filter((h) => !(h.within || h.edge))
    .sort(
      (a, b) =>
        Number(Boolean(a.alone)) - Number(Boolean(b.alone)) ||
        arc(a) - arc(b) ||
        collate(a.name, b.name),
    );
};

/**
 * Once a machine's cords bundle, the ring is too wide for a column of
 * machines at its middle to be told apart: each machine carrying networks
 * moves out to the middle of its own arc, so its trunks start from it and
 * two machines' arcs read as two.
 */
const orbit = (
  tops: readonly Hub[],
  angle: ReadonlyMap<string, number>,
  ring: number,
  machines: Map<string, Point>,
): void => {
  const carried = new Map<string, Hub[]>();
  for (const h of tops) {
    if (h.machine) {
      carried.set(h.machine, [...(carried.get(h.machine) ?? []), h]);
    }
  }
  if (![...carried.values()].some((hs) => hs.length > CORD_BUNDLE_OVER)) {
    return;
  }
  for (const [name, hs] of carried) {
    const [cx, cy] = hs.reduce(
      ([x, y], h) => [x + Math.cos(angle.get(h.key) ?? 0), y + Math.sin(angle.get(h.key) ?? 0)],
      [0, 0],
    );
    const a = Math.atan2(cy, cx);
    machines.set(name, [Math.cos(a) * ring * MACHINE_ORBIT, Math.sin(a) * ring * MACHINE_ORBIT]);
  }
};

/**
 * Where everything in `realm` sits, its cells sown as `seeds` says: the
 * machines at the middle, each top hub round them in a share of the ring
 * as wide as its family, each branch further out from its parent, the
 * receptors above everything and the builders below.
 */
export const layOut = (realm: Realm, seeds: Seeds): Layout => {
  const disc = new Map([...roomOf(realm, seeds)].map(([key, r]) => [key, discRadius(r)]));
  const familyWidth = familyWidthOf(disc);
  // Each machine's networks take one arc of the ring, clockwise from the
  // top, the top machine's first, so its cords never cross the middle
  // and a machine opened is the arc it carries.
  const tops = topsOf(realm);
  const widths = tops.map((h) => familyWidth(h) + GAP);
  const total = widths.reduce((a, b) => a + b, 0);
  const column = realm.machines.length;
  const machineSpan = ((column - 1) * MACHINE_STEP) / 2 + MACHINE_R;
  const ring = Math.max(RING_MIN + machineSpan, total / (2 * Math.PI));
  const placing: Placing = { at: new Map(), angle: new Map() };
  const place = placerOf(disc, placing);
  // The machines stand in a column, so the first machine's arc is turned
  // to sit over it, at the top, and the last one's comes round below.
  const firstRun = tops.findIndex((h) => h.machine !== tops[0]?.machine);
  const lead = widths.slice(0, firstRun < 0 ? widths.length : firstRun).reduce((a, b) => a + b, 0);
  let used = -lead / 2;
  tops.forEach((h, i) => {
    const width = widths[i] ?? 0;
    const a = -Math.PI / 2 + ((used + width / 2) / total) * 2 * Math.PI;
    used += width;
    const d = ring + (disc.get(h.key) ?? 0);
    place(h, [Math.cos(a) * d, Math.sin(a) * d], a);
  });
  // The receptors stand at the edge, above everything the ring and its
  // branches reach, since they are where the outside comes in.
  const reaches = [...placing.at].map(([k, [x, y]]) => Math.hypot(x, y) + (disc.get(k) ?? 0));
  const outer = Math.max(ring, ...reaches);
  const edge = [...realm.hubs.values()].find((h) => h.edge);
  if (edge) {
    place(edge, [0, -(outer + EDGE_GAP + (disc.get(edge.key) ?? 0))], -Math.PI / 2);
  }
  // Machines stand in a column, apart in proportion to the ring, since a
  // machine's mark is its name, wide and short: a row of them would need
  // several times the room to keep their boxes apart.
  const step = Math.max(MACHINE_STEP, ring * MACHINE_SPREAD);
  const machines = new Map<string, Point>(
    realm.machines.map((x, i) => [x.name, [0, (i - (column - 1) / 2) * step]]),
  );
  // A builder carries no cord, and inside the ring a mark of words would
  // stand on the hubs once the map is zoomed out to its whole, so the
  // builders stand under the ring, below everything it and its branches
  // reach, as the receptors stand above it: under the machines' column, a
  // step apart, kept there when the machines move out to their arcs.
  const foot = outer + EDGE_GAP + MACHINE_R;
  const builders = new Map<string, Point>(
    realm.builders.map((b, j) => [b.name, [0, foot + j * MACHINE_STEP]]),
  );
  orbit(tops, placing.angle, ring, machines);
  return { at: placing.at, angle: placing.angle, disc, machines, builders, ring };
};

/** How far a hub's cells reach from it as sown now, so its ground fits the colony it holds. */
export const colonyRadius = (seeds: Seeds, key: string): number => {
  const n = seeds.get(key)?.length ?? 0;
  return n ? SEED * Math.sqrt(n - 1 + HOLE) + R : HUB_R;
};

/** Each cell's place: its hub's, plus its seed's. */
export const placeCells = (realm: Realm, layout: Layout, seeds: Seeds): Map<string, Point> => {
  const pos = new Map<string, Point>();
  for (const h of realm.hubs.values()) {
    const [hx, hy] = layout.at.get(h.key) ?? [0, 0];
    (seeds.get(h.key) ?? []).forEach((id, i) => {
      if (!(id && realm.byId.has(id))) {
        return;
      }
      const [dx, dy] = seedAt(i);
      pos.set(id, [hx + dx, hy + dy]);
    });
  }
  return pos;
};

/** The shortest tree joining `points` (Prim's), as pairs of indices: the hyphae a network grows. */
export const shortestTree = (points: readonly Point[]): [number, number][] => {
  const n = points.length;
  const edges: [number, number][] = [];
  if (n < 2) {
    return edges;
  }
  const best = new Array<number>(n).fill(Number.POSITIVE_INFINITY);
  const from = new Array<number>(n).fill(-1);
  const done = new Array<boolean>(n).fill(false);
  best[0] = 0;
  for (let step = 0; step < n; step++) {
    let u = -1;
    for (let i = 0; i < n; i++) {
      if (!done[i] && (u < 0 || (best[i] ?? 0) < (best[u] ?? 0))) {
        u = i;
      }
    }
    done[u] = true;
    const parent = from[u] ?? -1;
    if (parent >= 0) {
      edges.push([parent, u]);
    }
    const [ux, uy] = points[u] ?? [0, 0];
    for (let v = 0; v < n; v++) {
      const [vx, vy] = points[v] ?? [0, 0];
      const d = (vx - ux) ** 2 + (vy - uy) ** 2;
      if (!done[v] && d < (best[v] ?? 0)) {
        best[v] = d;
        from[v] = u;
      }
    }
  }
  return edges;
};

/** The control point of a line from `p` to `q` bowed `by` of its length, as hyphae grow. */
export const control = ([x1, y1]: Point, [x2, y2]: Point, by: number): Point => [
  (x1 + x2) / 2 - (y2 - y1) * by,
  (y1 + y2) / 2 + (x2 - x1) * by,
];
