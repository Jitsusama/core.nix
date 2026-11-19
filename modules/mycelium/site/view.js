// Your mycelium, now: takes each reading site.ts hands it as a mycelium:now
// event, made live from the node's records as mycelium's own networks view
// makes them, and draws it as the network it is. The
// machines sit at the root; each orchestration is a hub on a ring around
// them, its branches further out; its cells pack round it like seeds in a
// sunflower, joined by hyphae that branch from the hub; and faint
// cross-hyphae join cells that share a repository or a group. Places never
// move while the page is open: a cell keeps its seed, a new one takes the
// next, and one that ends leaves its seed empty. A click centres on a cell
// or a hub and says what it is in the panel beside the map.
// Read-only: nothing here acts on the network.
//
// Long because it is the drawing as it was served by hand, brought in whole
// so the map moves to the feed without changing what it draws; it is split
// by what changes for one reason as it is rewritten in TypeScript on the
// kit (modules/mycelium/site/README.md says what remains). What each key
// asks is decided in keys.ts, which this imports as the module it's built to.
import { EVERY_PAGE_ACT, EVERY_TREE_ACT, findKey, mapKey, pageKey, stepped, switchedTo, treeKey } from "./keys.js";

(() => {
  // The most time between two readings while the node moves: site.ts hands
  // one over at most this often, and a message heard is flown over it.
  const EVERY_MS = 2000;
  const SVG = "http://www.w3.org/2000/svg";

  // The drawing's measures, in the map's own units, which zoom scales.
  const R = 11; // a cell's membrane
  const NUCLEUS = 6.5;
  const QUIET_DOT = 5.5; // the ring inside a quiet cell
  const HUB_R = 21; // an orchestration's hub
  const MACHINE_R = 19;
  const SPOKES = 8; // round a network's hub, which no cell has, so the two never read alike
  const SPOKE_FROM = 3; // how far out past the hub's ring a spoke starts
  const SPOKE_LONG = 6;
  const LENT_CORNER = 0.32; // how much of a lent machine's shorter side each corner keeps
  const SEED = 16.5; // a sunflower's spacing: seeds about 29 apart
  const HOLE = 5; // seeds left empty round a hub, so it has room
  const GAP = 54; // between two neighbouring discs
  const BRANCH_GAP = 2.4 * GAP; // between a hub's disc and a branch's, so each colony reads as its own
  const EDGE_GAP = 2 * GAP; // between the furthest network and the receptors at the edge
  const DISH_LIP = 0.6; // how far above a receptor's middle its dish's lips reach, in R
  const DISH_DEPTH = 1.6; // its curve's pull below the middle, which puts its floor half an R down
  const RING_MIN = 190; // the least distance from the machines to a hub
  const MACHINE_STEP = 96;
  const MACHINE_SPREAD = 0.4; // the least distance between machines, as a share of the ring
  const GOLDEN = Math.PI * (3 - Math.sqrt(5));
  // Zoom, as screen pixels per unit.
  const K_MIN = 0.15;
  const K_MAX = 3.2;
  const K_LEAST = 0.005; // the floor under even the biggest whole map
  const WHOLE_SLACK = 0.8; // how much further out than the whole map zooming may go
  const K_NEAR = 1.6; // how close a chosen cell is brought
  const FIT_PAD = 28; // screen pixels kept clear round the whole map
  const LABEL_ROOM = 150; // screen pixels a hub's name may reach past it
  const ROOM_SHARE = 0.16; // at most this share of a narrow view's width
  const GLIDE_MS = 320;
  const GLIDE_UNDER = 2000; // cells, past which the view steps rather than glides
  const LATELY = 6; // messages listed for a cell
  const FAULT_ROWS = 8; // cells with no purpose listed by name, the rest counted
  // What mycelium calls a cell that said nothing of itself, which is no
  // purpose however it reads.
  const UNNAMED = "a session without a name";
  const NO_PURPOSE = "No purpose written yet.";

  const view = document.getElementById("map-view");
  const svg = document.getElementById("map-draw");
  const world = document.getElementById("world");
  const labelsEl = document.getElementById("labels");
  const asof = document.getElementById("asof");
  const panelBody = document.getElementById("panel-body");
  const find = document.getElementById("find");
  const findLine = document.getElementById("find-line");
  const findCount = document.getElementById("find-count");
  const trail = document.getElementById("trail");
  const trailSteps = document.getElementById("trail-steps");
  // What the find line matched, in the order n steps through them, and
  // the one the keys were last taken to.
  let found = [];
  let foundSet = new Set();
  let foundAt = -1;
  let findVisited = false; // whether this find has opened a match yet
  let trailShown = "";
  const keys = document.getElementById("keys");
  const where = document.getElementById("where");
  const tree = document.getElementById("tree");
  // The tree scrolls with the panel's one column, not on its own.
  const treeBox = document.getElementById("panel");
  const still = matchMedia("(prefers-reduced-motion: reduce)");

  // ---- Shapes: each orchestration is known by its own. ----

  const SHAPES = {
    diamond: (s) => `M0,${-s}L${s},0L0,${s}L${-s},0Z`,
    triangle: (s) => `M0,${-s}L${s * 0.95},${s * 0.72}L${-s * 0.95},${s * 0.72}Z`,
    square: (s) => {
      const h = s * 0.8;
      return `M${-h},${-h}H${h}V${h}H${-h}Z`;
    },
    plus: (s) => {
      const t = s * 0.36;
      return `M${-t},${-s}H${t}V${-t}H${s}V${t}H${t}V${s}H${-t}V${t}H${-s}V${-t}H${-t}Z`;
    },
    circle: (s) => {
      const r = s * 0.82;
      return `M${-r},0a${r},${r} 0 1,0 ${2 * r},0a${r},${r} 0 1,0 ${-2 * r},0Z`;
    },
    hexagon: (s) => {
      const p = [];
      for (let i = 0; i < 6; i++) {
        const a = (Math.PI / 3) * i + Math.PI / 6;
        p.push(`${(Math.cos(a) * s).toFixed(2)},${(Math.sin(a) * s).toFixed(2)}`);
      }
      return `M${p.join("L")}Z`;
    },
    down: (s) => `M0,${s}L${s * 0.95},${-s * 0.72}L${-s * 0.95},${-s * 0.72}Z`,
    star: (s) => {
      const p = [];
      for (let i = 0; i < 10; i++) {
        const r = i % 2 ? s * 0.45 : s;
        const a = (Math.PI / 5) * i - Math.PI / 2;
        p.push(`${(Math.cos(a) * r).toFixed(2)},${(Math.sin(a) * r).toFixed(2)}`);
      }
      return `M${p.join("L")}Z`;
    },
    bar: (s) => `M${-s},${-s * 0.42}H${s}V${s * 0.42}H${-s}Z`,
  };
  const HUB_SHAPES = ["square", "triangle", "plus", "hexagon", "circle", "down", "star"];

  // A hub's shape comes from its name, so it stays the same on every visit
  // while the hubs beside it come and go; two names landing on one shape
  // are told apart in the order of their names.
  const hash = (text) => {
    let h = 0;
    for (const c of text) h = (h * 31 + c.charCodeAt(0)) >>> 0;
    return h;
  };
  const shapesFor = (keys) => {
    const taken = new Set();
    const out = new Map();
    for (const key of [...keys].sort()) {
      let i = hash(key) % HUB_SHAPES.length;
      for (let n = 0; n < HUB_SHAPES.length && taken.has(HUB_SHAPES[i]); n++) i = (i + 1) % HUB_SHAPES.length;
      if (taken.size >= HUB_SHAPES.length) taken.clear();
      taken.add(HUB_SHAPES[i]);
      out.set(key, HUB_SHAPES[i]);
    }
    return out;
  };

  // A network's spokes round a ring of radius r, the first straight up.
  const spokesPath = (r) => {
    let d = "";
    for (let i = 0; i < SPOKES; i++) {
      const a = (2 * Math.PI * i) / SPOKES;
      const [dx, dy] = [Math.sin(a), -Math.cos(a)];
      const [a0, a1] = [r + SPOKE_FROM, r + SPOKE_FROM + SPOKE_LONG];
      d += `M${(dx * a0).toFixed(2)},${(dy * a0).toFixed(2)}L${(dx * a1).toFixed(2)},${(dy * a1).toFixed(2)}`;
    }
    return d;
  };
  // A receptor's dish of half-width s, open upward, as the canvas paints it.
  const dishPath = (s) => `M${-s},${(-s * DISH_LIP).toFixed(2)}Q0,${(s * DISH_DEPTH).toFixed(2)} ${s},${(-s * DISH_LIP).toFixed(2)}`;

  const glyph = (shape, size = 12) =>
    `<svg class="glyph" viewBox="${-size} ${-size} ${size * 2} ${size * 2}" aria-hidden="true"><path d="${SHAPES[shape](size * 0.9)}" fill="currentColor"/></svg>`;

  // ---- Words. ----

  const esc = (text) =>
    String(text).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  const collate = new Intl.Collator("en-CA").compare;
  const SMALL = new Set(["a", "an", "and", "as", "at", "by", "for", "from", "in", "into", "of", "on", "or", "the", "to", "with"]);

  // Words of `text` up to `room` characters, cut at a whole word and never
  // ending on a small one.
  const cut = (text, room) => {
    const kept = [];
    let length = 0;
    for (const w of text.split(/\s+/)) {
      if (length + w.length > room && kept.length > 0) break;
      kept.push(w);
      length += w.length + 1;
    }
    while (kept.length > 1 && SMALL.has(kept[kept.length - 1].toLowerCase())) kept.pop();
    return kept.join(" ").replace(/[:,;]$/, "");
  };

  // A name as drawn on the map: at most `room` characters, cut between
  // words, since the panel and the tree carry it whole.
  const NAME_ROOM = 40;
  const clipped = (text, room = NAME_ROOM) => {
    if (text.length <= room) return text;
    const cut = text.slice(0, room - 3);
    const at = cut.lastIndexOf(" ");
    const kept = (at > room / 2 ? cut.slice(0, at) : cut).split(" ");
    // Never end on a word that leads into the cut, as "replace the...".
    while (kept.length > 1 && SMALL.has(kept[kept.length - 1].toLowerCase())) kept.pop();
    return `${kept.join(" ").replace(/[\s:,;.-]+$/, "")}...`;
  };

  // `text` in lines of at most `room` characters, broken between words.
  const lines = (text, room) => {
    const out = [""];
    for (const w of text.split(/\s+/)) {
      const last = out[out.length - 1];
      if (last && last.length + 1 + w.length > room) out.push(w);
      else out[out.length - 1] = last ? `${last} ${w}` : w;
    }
    return out;
  };

  // A session's short name: the one its window goes by ("Git upkeep" of
  // "π - Git upkeep - kestrel"), else its project's title cut short where
  // the window's own was cut mid-word.
  const shortName = (s) => {
    const parts = (s.window || "").split(" - ");
    const middle = parts.length >= 3 ? parts.slice(1, -1).join(" - ") : "";
    if (middle && !/\u2026|\.\.\./.test(middle)) return middle;
    if (s.purpose) return cut(s.purpose, 26);
    // A cell that has said nothing of itself yet goes by where it runs,
    // never by its code.
    return middle.replace(/\u2026|\.\.\./g, "").trim() || `Unnamed cell on ${s.machine || "no machine"}`;
  };

  // The receptors stand together at the realm's edge, as one group of
  // their own: the sources the network hears from, never a network.
  const OUTSIDE = "#outside";
  // What a receptor heard, in the node's own counts: how much came in
  // today and this hour, and how much of it was addressed to him.
  const sensedWords = (n) => {
    const today = Number(n.today) || 0;
    const hour = Number(n.hour) || 0;
    const his = Number(n.addressed) || 0;
    if (!today) return "Listening; nothing has come in today";
    return `${plural(today, "arrival")} today, ${his ? `${NUMBER.format(his)} addressed to you` : "none addressed to you"}, ${hour ? `${NUMBER.format(hour)} this hour` : "none this hour"}`;
  };
  // What a group of cells is counted as: receptors when it holds only them.
  const kindWord = (cells) => (cells.length && cells.every((s) => s.kind === "receptor") ? "receptor" : "cell");
  // The group a cell is walked in: its network, or the receptors' edge.
  const netOf = (s) => s?.hub || (s?.loose?.edge ? s.loose : null);

  const splitTitle = (title) => {
    const at = title.indexOf(": ");
    return at > 0 ? [title.slice(0, at), title.slice(at + 2)] : [title, ""];
  };

  // One formatter each, made once: toLocaleString makes a new one on every
  // call, which at 10k cells was most of what a tree row cost.
  const NUMBER = new Intl.NumberFormat("en-CA");
  // Dollars to the cent below a hundred, where cents still tell cells
  // apart, and whole above it, where they are noise.
  const WHOLE = new Intl.NumberFormat("en-CA", { maximumFractionDigits: 0 });
  const CENTS = new Intl.NumberFormat("en-CA", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const money = (usd) => (usd == null ? "" : `$${(usd >= 100 ? WHOLE : CENTS).format(usd)}`);
  const HOURS = { hour: "2-digit", minute: "2-digit", hour12: false };
  const CLOCK = [new Intl.DateTimeFormat("en-CA", HOURS), new Intl.DateTimeFormat("en-CA", { ...HOURS, second: "2-digit" })];
  const clock = (iso, seconds = true) => CLOCK[+seconds].format(new Date(iso));
  // A time off the reading's own day carries its day, or yesterday's
  // 06:49 reads as this morning's.
  const DAY = new Intl.DateTimeFormat("en-GB", { day: "numeric", month: "short" });
  const DAY_MS = 86_400_000;
  const midnight = (t) => new Date(t).setHours(0, 0, 0, 0);
  const dayClock = (t) => {
    const days = Math.round((midnight(t) - midnight(current.at)) / DAY_MS);
    const hm = clock(t, false);
    return days === 0 ? hm : days === -1 ? `yesterday at ${hm}` : days === 1 ? `tomorrow at ${hm}` : `${DAY.format(new Date(t))} at ${hm}`;
  };
  const atWords = (t) => {
    const said = dayClock(t);
    return /^\d/.test(said) ? (said.includes(" ") ? `on ${said}` : `at ${said}`) : said;
  };
  const plural = (n, one, many = `${one}s`) => `${NUMBER.format(n)} ${n === 1 ? one : many}`;
  // What a key says depends on the data alone, so it is worked out once
  // for each reading.
  const perReading = (fn) => {
    const memo = new Map();
    let of = null;
    return (key) => {
      if (of !== current) {
        memo.clear();
        of = current;
      }
      if (!memo.has(key)) memo.set(key, fn(key));
      return memo.get(key);
    };
  };
  const gib = (bytes) => (bytes == null ? "" : `${(bytes / 2 ** 30).toFixed(0)} GiB`);

  // What cells spent today: each session's own sum, since local midnight,
  // of the list price pi puts on every call it makes. The bill is
  // spend-watch's to say; this is what the sessions say of themselves.
  const spentBy = (s) => s.usage?.cost_today || 0;
  const spends = (cells) => cells.some((s) => s.usage?.cost_today != null);
  // The sum is kept per list, and each reading brings new lists.
  const SPENT = new WeakMap();
  const spentOf = (cells) => {
    if (!SPENT.has(cells)) SPENT.set(cells, cells.reduce((t, s) => t + spentBy(s), 0));
    return SPENT.get(cells);
  };
  const spentWords = (cells) => (spends(cells) ? `, ${money(spentOf(cells))} today` : "");

  // ---- The model, from what mycelium says. ----

  const STANDINGS = ["asks", "stuck", "moving", "quiet", "dormant"];
  const STANDING_WORD = { asks: "needs you", stuck: "stuck", moving: "moving", quiet: "quiet", dormant: "dormant" };
  // What waits on the mesh is the node's judgement where it serves one: a
  // question waits on each peer it went to until that peer answers it or
  // anything in its thread, and a hold stands until its sender ends it.
  // A node that judges nothing leaves the page to guess from the latest
  // messages, where a question waits on each cell it named until that
  // cell says anything later; with no reply link, any word since counts
  // as an answer, and the page then says "no word since" rather than
  // "no answer yet". Either way a question younger than WAIT_GRACE_MS is
  // talk under way, not a hold-up.
  const WAIT_GRACE_MS = 3 * 60_000;
  const owing = (now, cells, byAddress) => {
    for (const s of cells) {
      s.owes = [];
      s.holds = [];
    }
    const grown = (since) => Date.parse(now.at) - Date.parse(since) >= WAIT_GRACE_MS;
    if (now.waits) {
      for (const x of now.waits) {
        if (x.intent === "hold") byAddress.get(x.from)?.holds.push({ to: x.groups || [], at: x.since, until: x.until });
        else if (x.intent === "question" && grown(x.since)) {
          for (const address of x.waiting_on || []) if (address !== x.from) byAddress.get(address)?.owes.push({ from: x.from, at: x.since });
        }
      }
    } else {
      const sent = [...(now.messages || [])].sort((a, b) => a.at.localeCompare(b.at));
      sent.forEach((x, i) => {
        if (x.intent !== "question" || !grown(x.at)) return;
        for (const g of x.groups || []) {
          const address = g.match(/^peer\.(.+)$/)?.[1].toUpperCase();
          const s = address && address !== x.from ? byAddress.get(address) : null;
          if (s && !sent.slice(i + 1).some((y) => y.from === address)) s.owes.push({ from: x.from, at: x.at });
        }
      });
    }
    for (const s of cells) s.owes.sort((a, b) => a.at.localeCompare(b.at));
  };
  // What the page says of a question waiting, by how it knows: the node's
  // judgement, or its own guess from the latest messages.
  const unansweredWords = () => (current?.judged ? "with no answer yet" : "with no word since");
  // Where a hold went, as the mesh writes it (@PEER, #machine, #quest/x),
  // said as the page names each: a cell by its short name, a network as
  // the drawers name it, the machine as every cell on it.
  const holdTarget = (to) => {
    const peer = to.match(/^@(.+)$/)?.[1].toUpperCase();
    if (peer) return current?.byAddress.get(peer)?.short || peer;
    if (to === "#machine") return "every cell on its machine";
    const key = to.replace(/^#/, "").replace("/", ".");
    const hub = current?.hubs.get(key);
    if (hub) return hub.name;
    return current?.shared.some((x) => x.key === key) ? sharedWords(key) : key;
  };
  const holdWords = (h) => `holding ${h.to.map(holdTarget).join(" and ") || "its peers"} since ${dayClock(h.at)}${h.until ? `, until ${dayClock(h.until)} at most` : ""}`;
  const WAIT_GAP = 3; // between a mark and its waiting ring
  const HOLD_REACH = 7; // how far a hold's wall stands out past any waiting ring
  const WAIT_FULL_MS = 30 * 60_000; // a question this old swells its ring fully
  const SWELL_BASE = 2;
  const SWELL_PER_QUESTION = 2;
  const SWELL_FOR_AGE = 6;
  const SWELL_MAX = 12;
  const ageOf = (owes) => Math.max(0, Date.parse(current.at) - Date.parse(owes[0].at));
  const swell = (owes) =>
    owes?.length ? Math.min(SWELL_MAX, SWELL_BASE + SWELL_PER_QUESTION * (owes.length - 1) + SWELL_FOR_AGE * Math.min(1, ageOf(owes) / WAIT_FULL_MS)) : 0;
  const minutesWord = (ms) => (ms < 60_000 ? "under a minute" : `${Math.round(ms / 60_000)} min`);
  const waitingWords = (owes) => `${owes.length === 1 ? "a question" : `${owes.length} questions`} waiting`;

  const STANDING_OF_CELL = { "needs-you": "asks", stuck: "stuck", moving: "moving", quiet: "quiet", dormant: "dormant" };
  const SHARED_WORD = { project: "quest", repository: "repository", group: "group" };
  // A shared network as the panel names it: "the backwater repository".
  const sharedWords = (id) => {
    const n = current?.shared.find((x) => x.key === id);
    return n ? `the ${n.short} ${SHARED_WORD[n.shape]}` : id;
  };

  // The map before mycelium drew networks itself: sessions with their mesh
  // groups and the swarm's teams, read as the networks view reads them, so
  // the page draws either until every machine has the networks view.
  const fromSessions = (now) => {
    const lower = (key) => `orchestration.${key.toLowerCase()}`;
    const holders = new Map();
    for (const s of now.sessions) {
      for (const g of s.groups || []) {
        if (!holders.has(g)) holders.set(g, []);
        holders.get(g).push(s.address);
      }
    }
    const teams = now.teams.map((t) => ({
      kind: "network",
      name: lower(t.key),
      shape: "orchestration",
      short: t.key.toLowerCase(),
      title: t.title || null,
      within: t.within ? lower(t.within) : null,
      cells: holders.get(lower(t.key)) || [],
    }));
    const shared = [...holders]
      .filter(([g, cells]) => !g.startsWith("orchestration.") && cells.length >= 2)
      .map(([g, cells]) => ({ kind: "network", name: g, shape: g.startsWith("quest.") ? "project" : g.startsWith("repo.") ? "repository" : "group", short: g.replace(/^(repo|quest)\./, ""), title: null, within: null, cells }));
    const counted = new Set([...teams, ...shared].map((n) => n.name));
    const cells = now.sessions.map((s) => ({
      id: s.id,
      name: s.address,
      type: "session",
      machine: s.machine,
      quest: s.quest,
      since: s.since,
      title: s.purpose || s.title,
      window: s.purpose ? s.title : undefined,
      standing: s.wants ? "needs-you" : s.paused ? "stuck" : s.parked ? "dormant" : s.state === "busy" ? "moving" : "quiet",
      doing: s.step,
      asks: s.wants,
      repo: s.repo,
      usage: { cost_today: s.cost_today, context: s.context, turns_today: s.turns_today, last_turn: s.last_turn },
      networks: (s.groups || []).filter((g) => counted.has(g)),
      leads: now.teams.filter((t) => t.lead === s.id).map((t) => lower(t.key)),
    }));
    return { at: now.at, networks: [...teams, ...shared], cells, machines: now.machines, messages: now.messages, waits: now.waits, balance: now.balance, ties: now.ties };
  };

  const model = (raw) => {
    const now = raw.networks ? raw : fromSessions(raw);
    // The model draws a basin's machines as one stand (model.ts), the
    // stand its cells are drawn from; each cell still says which machine
    // it runs on, as its host.
    const machines = now.machines || [];
    const standOf = new Map(machines.flatMap((x) => (x.members || []).map((y) => [y.name, x.name])));
    const known = new Set(now.networks.map((n) => n.name));
    const orchestrations = now.networks.filter((n) => n.shape === "orchestration");
    const shapes = shapesFor(orchestrations.map((n) => n.name));
    const hubs = new Map();
    // An orchestration the node left untitled is named by its quest's
    // network, the same id on the quest's side, so it never reads as a
    // bare id where a title is known.
    const questTitle = new Map(now.networks.filter((n) => n.title && n.name.startsWith("quest.")).map((n) => [n.name.slice("quest.".length), n.title]));
    for (const o of orchestrations) {
      const n = o.title ? o : { ...o, title: questTitle.get(o.name.slice(o.name.indexOf(".") + 1)) || null };
      const [name, carries] = splitTitle(n.title || n.short || n.name);
      hubs.set(n.name, {
        key: n.name,
        name,
        // What it is for: the part of its title after the name, else the
        // title itself, which says it whole; with no title, nothing.
        carries: n.title ? carries || n.title : "",
        shape: shapes.get(n.name),
        within: n.within && known.has(n.within) ? n.within : null,
        children: [],
        cells: [],
      });
    }
    for (const h of hubs.values()) if (h.within) hubs.get(h.within)?.children.push(h);
    const depth = (h) => (h.within ? 1 + depth(hubs.get(h.within)) : 0);
    // A cell moving between machines can arrive from both for a while,
    // the one it left still listing it. It is one cell under one address,
    // so the page keeps the copy started most recently, with the groups
    // either copy names.
    const once = new Map();
    for (const c of now.cells) {
      const had = once.get(c.name);
      if (!had) once.set(c.name, c);
      else {
        const [keep, other] = (c.since || "") > (had.since || "") ? [c, had] : [had, c];
        once.set(c.name, { ...keep, networks: [...new Set([...keep.networks, ...other.networks])] });
      }
    }
    // A network sits within another, so a cell on a quest that sits under
    // an orchestration works in that orchestration, even when the node
    // names only the quest: it is alone only in no orchestration at all.
    const withinOf = new Map(now.networks.map((n) => [n.name, n.within]));
    const orchestrationOf = (name) => {
      const seen = new Set();
      for (let at = name; at && !seen.has(at); at = withinOf.get(at)) {
        if (hubs.has(at)) return hubs.get(at);
        seen.add(at);
      }
      return null;
    };
    const cells = [...once.values()].map((c) => {
      const on = [...new Set(c.networks.map(orchestrationOf).filter(Boolean))];
      const own = on.find((h) => c.quest && c.quest.toLowerCase().endsWith(`-${h.key.split(".").pop()}`));
      const deepest = [...on].sort((a, b) => depth(b) - depth(a))[0];
      const hub = own || deepest || null;
      const windowed = !c.window && (c.title || "").startsWith("\u03c0 - ");
      const receptor = c.type === "receptor";
      const s = {
        id: c.id || c.name,
        // A receptor is where the outside comes in (mail, calls, the
        // calendar): a cell of the realm's edge, never one working alone.
        kind: receptor ? "receptor" : "session",
        senses: receptor ? c.counts || {} : null,
        address: c.name,
        machine: standOf.get(c.machine) ?? c.machine,
        host: c.machine,
        since: c.since,
        window: c.window || (windowed ? c.title : null),
        // What it is for: its quest's title, else what it said it works
        // on, as mycelium names it; never a window's title, nor mycelium's
        // stand-in for a cell that said nothing.
        purpose: windowed || c.title === UNNAMED ? null : c.title || null,
        quest: c.quest || null,
        doing: receptor ? sensedWords(c.counts || {}) : c.doing || null,
        wants: c.asks || null,
        repo: c.repo || null,
        usage: c.usage || {},
        standing: receptor ? (c.state === "reading" ? "moving" : "quiet") : STANDING_OF_CELL[c.standing] || "quiet",
        hub,
        also: on.filter((h) => h !== hub),
        networks: c.networks,
        leads: (c.leads || []).filter((l) => hubs.has(l)),
      };
      return { ...s, short: shortName(s) };
    });
    // A cell no network counts works alone, which is no fault: it gathers
    // with the others like it on its own machine, so each cell stays under
    // the machine it runs on.
    const loose = new Map();
    const outside = { key: OUTSIDE, name: "Receptors", carries: "", shape: "circle", within: null, children: [], cells: [], edge: true };
    for (const s of cells) {
      if (s.kind === "receptor") {
        s.hub = null;
        s.loose = outside;
        outside.cells.push(s);
        continue;
      }
      if (s.hub) {
        s.hub.cells.push(s);
        continue;
      }
      const on = s.machine || "";
      if (!loose.has(on)) loose.set(on, { key: on ? `#loose.${on}` : "#loose", name: on ? `Working alone on ${on}` : "Working alone", carries: "cells that work alone, in no network", shape: "circle", within: null, children: [], cells: [], alone: true });
      s.loose = loose.get(on);
      s.loose.cells.push(s);
    }
    // Where they run is said only to tell two such groups apart.
    if (loose.size === 1) for (const h of loose.values()) h.name = "Working alone";
    for (const h of loose.values()) hubs.set(h.key, h);
    if (outside.cells.length) {
      const senses = outside.cells.map((s) => s.purpose || s.address).sort(collate);
      outside.carries = `where ${senses.length > 1 ? `${senses.slice(0, -1).join(", ")} and ${senses.at(-1)}` : senses[0]} come in from outside, each listened to for the cells that read it`;
      hubs.set(OUTSIDE, outside);
    }
    for (const h of hubs.values()) h.lead = cells.find((s) => s.leads.includes(h.key)) || null;
    // A hub is placed by the machine most of its cells run on. That is
    // where it sits on the map and in the tree, never what it belongs to:
    // a network belongs to no machine, so the trail and the drawers name
    // every machine that feeds it.
    const listed = new Set(machines.map((x) => x.name));
    for (const h of hubs.values()) {
      const counts = new Map();
      for (const s of h.cells) if (listed.has(s.machine)) counts.set(s.machine, (counts.get(s.machine) || 0) + 1);
      h.machine = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0] || machines[0]?.name || null;
    }
    // Every machine feeding a network, its branches' cells counted in, so
    // a network fed from two machines draws a cord from each: how many
    // cells each runs, most first.
    const under = (h) => [...h.cells, ...h.children.flatMap(under)];
    for (const h of hubs.values()) {
      const counts = new Map();
      for (const s of under(h)) if (listed.has(s.machine)) counts.set(s.machine, (counts.get(s.machine) || 0) + 1);
      h.feeds = [...counts].sort((a, b) => b[1] - a[1] || collate(a[0], b[0]));
    }
    const byAddress = new Map(cells.map((s) => [s.address, s]));
    const byId = new Map(cells.map((s) => [s.id, s]));
    owing(now, cells, byAddress);
    for (const h of hubs.values()) h.owes = h.cells.flatMap((s) => s.owes);
    const shared = now.networks
      .filter((n) => SHARED_WORD[n.shape])
      .map((n) => ({ key: n.name, shape: n.shape, short: n.short || n.name, title: n.title, cells: n.cells.map((a) => byAddress.get(a)).filter(Boolean) }))
      .filter((n) => n.cells.length >= 2);
    const repositories = new Set(cells.map((s) => s.repo).filter(Boolean)).size || shared.filter((n) => n.shape === "repository").length;
    // A feeding is drawn only where both its ends are, as a tie is.
    const fed = Array.isArray(now.fed) ? now.fed.filter((f) => byAddress.has(f.receptor) && byAddress.has(f.cell)) : null;
    return { at: now.at, hubs, cells, byAddress, byId, shared, machines, standOf, messages: now.messages || [], judged: Array.isArray(now.waits), balance: now.balance || null, ties: Array.isArray(now.ties) ? now.ties : null, fed, repositories };
  };

  // ---- Places that never move. ----

  // Each hub's seeds, in the order its cells first came: an id, or null
  // where a cell has ended. Kept while the page is open.
  const seeds = new Map();
  const sow = (m) => {
    const where = new Map();
    for (const s of m.cells) where.set(s.id, (s.hub || s.loose).key);
    for (const [key, list] of seeds) {
      for (let i = 0; i < list.length; i++) if (list[i] && where.get(list[i]) !== key) list[i] = null;
    }
    for (const h of m.hubs.values()) {
      const list = seeds.get(h.key) || [];
      const have = new Set(list);
      const fresh = h.cells.filter((s) => !have.has(s.id)).sort((a, b) => (a.since || "").localeCompare(b.since || "") || a.id.localeCompare(b.id));
      for (const s of fresh) {
        const empty = list.indexOf(null);
        if (empty >= 0) list[empty] = s.id;
        else list.push(s.id);
      }
      seeds.set(h.key, list);
    }
  };

  // A disc keeps room for more cells than it holds, in steps, so a cell
  // arriving doesn't move the hubs round it.
  const roomFor = (n) => Math.max(8, Math.ceil((n * 1.15 + 3) / 8) * 8);
  const discRadius = (room) => SEED * Math.sqrt(room + HOLE) + R;
  const seedAt = (i) => {
    const k = i + HOLE;
    const r = SEED * Math.sqrt(k);
    const a = k * GOLDEN;
    return [r * Math.cos(a), r * Math.sin(a)];
  };

  // Where everything sits, in map units: the machines at the origin, each
  // top hub round them in a share of the ring as wide as its family, and
  // each branch further out from its parent.
  let layout = null;
  let layoutKey = "";
  const lay = (m) => {
    const room = new Map();
    for (const h of m.hubs.values()) room.set(h.key, roomFor((seeds.get(h.key) || []).length));
    const key = [...room].map(([k, r]) => `${k}:${r}:${m.hubs.get(k)?.machine || ""}`).join("|") + `|${m.machines.map((x) => x.name).join(",")}`;
    if (key === layoutKey && layout) return layout;
    layoutKey = key;
    const disc = new Map([...room].map(([k, r]) => [k, discRadius(r)]));
    // Each machine's networks take one arc of the ring, clockwise from the
    // top, the top machine's first, so its cords never cross the middle
    // and a machine opened is the arc it carries.
    const machineAt = new Map(m.machines.map((x, i) => [x.name, i]));
    const arc = (h) => machineAt.get(h.machine) ?? m.machines.length;
    const tops = [...m.hubs.values()].filter((h) => !h.within && !h.edge).sort((a, b) => Boolean(a.alone) - Boolean(b.alone) || arc(a) - arc(b) || collate(a.name, b.name));
    const familyWidth = (h) => {
      const kids = h.children.reduce((sum, c) => sum + familyWidth(c) + GAP, 0);
      return Math.max(2 * disc.get(h.key), kids - GAP);
    };
    const widths = tops.map((h) => familyWidth(h) + GAP);
    const total = widths.reduce((a, b) => a + b, 0);
    const machineSpan = ((m.machines.length - 1) * MACHINE_STEP) / 2 + MACHINE_R;
    const ring = Math.max(RING_MIN + machineSpan, total / (2 * Math.PI));
    const at = new Map();
    const angle = new Map();
    const place = (h, x, y, a) => {
      at.set(h.key, [x, y]);
      angle.set(h.key, a);
      const kids = [...h.children].sort((p, q) => p.name.localeCompare(q.name));
      if (!kids.length) return;
      const reach = disc.get(h.key) + BRANCH_GAP + Math.max(...kids.map((c) => disc.get(c.key)));
      const spans = kids.map((c) => (familyWidth(c) + GAP) / reach);
      let from = a - spans.reduce((p, q) => p + q, 0) / 2;
      kids.forEach((c, i) => {
        const ca = from + spans[i] / 2;
        from += spans[i];
        const d = disc.get(h.key) + BRANCH_GAP + disc.get(c.key);
        place(c, x + Math.cos(ca) * d, y + Math.sin(ca) * d, ca);
      });
    };
    // The machines stand in a column, so the first machine's arc is turned
    // to sit over it, at the top, and the last one's comes round below.
    const firstRun = tops.findIndex((h) => h.machine !== tops[0]?.machine);
    const lead = widths.slice(0, firstRun < 0 ? widths.length : firstRun).reduce((a, b) => a + b, 0);
    let used = -lead / 2;
    tops.forEach((h, i) => {
      const a = -Math.PI / 2 + ((used + widths[i] / 2) / total) * 2 * Math.PI;
      used += widths[i];
      const d = ring + disc.get(h.key);
      place(h, Math.cos(a) * d, Math.sin(a) * d, a);
    });
    // The receptors stand at the edge, above everything the ring and its
    // branches reach, since they are where the outside comes in.
    const edge = m.hubs.get(OUTSIDE);
    if (edge) {
      const outer = Math.max(ring, ...[...at].map(([k, [x, y]]) => Math.hypot(x, y) + disc.get(k)));
      place(edge, 0, -(outer + EDGE_GAP + disc.get(OUTSIDE)), -Math.PI / 2);
    }
    // Machines stand in a column, apart in proportion to the ring, since a
    // machine's mark is its name, wide and short: a row of them would
    // need several times the room to keep their boxes apart.
    const step = Math.max(MACHINE_STEP, ring * MACHINE_SPREAD);
    const machines = new Map(m.machines.map((x, i) => [x.name, [0, (i - (m.machines.length - 1) / 2) * step]]));
    // Once a machine's cords bundle, the ring is too wide for a row of
    // machines at its middle to be told apart: each machine carrying
    // networks moves out to the middle of its own arc, so its trunks start
    // from it and two machines' arcs read as two.
    const carried = new Map();
    for (const h of tops) if (h.machine) carried.set(h.machine, [...(carried.get(h.machine) || []), h]);
    if ([...carried.values()].some((hs) => hs.length > CORD_BUNDLE_OVER)) {
      for (const [name, hs] of carried) {
        const [cx, cy] = hs.reduce(([x, y], h) => [x + Math.cos(angle.get(h.key)), y + Math.sin(angle.get(h.key))], [0, 0]);
        const a = Math.atan2(cy, cx);
        machines.set(name, [Math.cos(a) * ring * MACHINE_ORBIT, Math.sin(a) * ring * MACHINE_ORBIT]);
      }
    }
    layout = { at, angle, disc, machines, ring };
    return layout;
  };

  // How far a hub's cells reach from it, as sown now rather than as
  // room is kept, so the ground fits the colony it holds.
  const COLONY_PAD = 12;
  const colonyRadius = (h) => {
    const n = (seeds.get(h.key) || []).length;
    return n ? SEED * Math.sqrt(n - 1 + HOLE) + R : HUB_R;
  };

  // Each cell's place: its hub's, plus its seed's.
  const placeCells = (m, L) => {
    const pos = new Map();
    for (const h of m.hubs.values()) {
      const [hx, hy] = L.at.get(h.key);
      (seeds.get(h.key) || []).forEach((id, i) => {
        if (!id || !m.byId.has(id)) return;
        const [dx, dy] = seedAt(i);
        pos.set(id, [hx + dx, hy + dy]);
      });
    }
    return pos;
  };

  // The places are kept for as long as the reading and the layout are:
  // seeds are sown only from a new reading, and a choice moves nothing.
  let placedFor = null;
  const placed = (m, L) => {
    if (placedFor?.m !== m || placedFor.L !== L) placedFor = { m, L, pos: placeCells(m, L) };
    return placedFor.pos;
  };

  // The shortest tree joining `points` (Prim's), as pairs of indices; the
  // hyphae a network grows between its cells, short ones first.
  const shortestTree = (points) => {
    const n = points.length;
    const edges = [];
    if (n < 2) return edges;
    const best = new Array(n).fill(Infinity);
    const from = new Array(n).fill(-1);
    const done = new Array(n).fill(false);
    best[0] = 0;
    for (let step = 0; step < n; step++) {
      let u = -1;
      for (let i = 0; i < n; i++) if (!done[i] && (u < 0 || best[i] < best[u])) u = i;
      done[u] = true;
      if (from[u] >= 0) edges.push([from[u], u]);
      const [ux, uy] = points[u];
      for (let v = 0; v < n; v++) {
        if (done[v]) continue;
        const d = (points[v][0] - ux) ** 2 + (points[v][1] - uy) ** 2;
        if (d < best[v]) {
          best[v] = d;
          from[v] = u;
        }
      }
    }
    return edges;
  };

  // A gently bowed line from one place to another, as hyphae grow.
  const control = ([x1, y1], [x2, y2], by) => [(x1 + x2) / 2 - (y2 - y1) * by, (y1 + y2) / 2 + (x2 - x1) * by];
  // Every line drawn, as a curve in map units, so names can keep off them.
  let lineShapes = [];
  let talkShapes = [];
  // A bowed line as its three points, for the ground layer to paint.
  const curve = (p, q, by = 0.12) => {
    const line = [p, control(p, q, by), q];
    lineShapes.push(line);
    return line;
  };
  // What a line opens when it is clicked, kept on the line itself;
  // `within` marks one that lives inside a network's ground, which there
  // gives the click to the ground.
  const opens = (line, kind, id, within = false) => Object.assign(line, { opens: { kind, id }, within });

  // ---- The ground layer: every network's ground and every line, painted
  // on one canvas under the marks. Thousands of lines cost one element
  // rather than thousands, and a frame repaints only what is on screen.
  // The inks are read off hidden probes wearing the same classes, so the
  // stylesheet stays the one place a look is said, both looks included. ----
  const groundEl = document.getElementById("map-ground");
  const groundCtx = groundEl.getContext("2d");
  const inkProbe = document.getElementById("ink-probe");
  const RECEDED = 0.22; // what the find line doesn't match recedes to this
  const GROUND_PAD = 8; // px past the map's edge a shape still paints
  // Each layer is a list of shapes in map units: a disc {tag: "circle",
  // cls, at, r} or a line {tag: "path", cls, w, line: [p, m, q]}.
  let groundLayers = [];
  let talkLayer = []; // talk of the last ten minutes, over the lines
  let talkBetween = 0; // of it, the talk with two ends on the map to join
  let fedShapes = []; // each receptor's feeding of a cell, the last ten minutes
  let fedLayer = []; // those feedings, over the talk
  let cellLayer = []; // the cells, over everything else on the ground
  // Each network's ground, [key, x, y, r], so a click on it opens it.
  let groundsAt = [];
  const inks = new Map();
  const dashOf = (v) => (v === "none" ? [] : v.split(/[\s,]+/).map(parseFloat).filter(Number.isFinite));
  // A computed paint of no opacity is none: the browser writes its alpha
  // last, so a clear one ends ", 0)".
  const paintOf = (v) => (v === "none" || v === "transparent" || v.endsWith(", 0)") ? null : v);
  // A cell's inks depend on its own classes too (chosen, far, hovered),
  // so its probe wears them on a parent group, as the marks once did.
  const inkOf = (tag, cls, parent = "") => {
    const id = `${svg.classList.contains("focused") ? 1 : 0}|${tag}|${cls}|${parent}`;
    let ink = inks.get(id);
    if (!ink) {
      const NS_SVG = "http://www.w3.org/2000/svg";
      const holder = inkProbe.appendChild(document.createElementNS(NS_SVG, "g"));
      holder.setAttribute("class", parent);
      const el = holder.appendChild(document.createElementNS(NS_SVG, tag));
      el.setAttribute("class", cls);
      const s = getComputedStyle(el);
      const held = parseFloat(getComputedStyle(holder).opacity);
      const own = parseFloat(s.opacity);
      ink = {
        stroke: paintOf(s.stroke),
        fill: paintOf(s.fill),
        width: parseFloat(s.strokeWidth) || 1,
        dash: dashOf(s.strokeDasharray),
        alpha: (Number.isFinite(held) ? held : 1) * (Number.isFinite(own) ? own : 1),
        cap: s.strokeLinecap,
      };
      holder.remove();
      inks.set(id, ink);
    }
    return ink;
  };
  // A change of look clears the inks and paints again.
  const relook = () => {
    inks.clear();
    paintGround();
  };
  new MutationObserver(relook).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", relook);
  matchMedia("(forced-colors: active)").addEventListener("change", relook);

  // Lines stop short of every name shown, as a map knocks its lines out
  // round its words, so a name reads whole wherever it had to sit. The
  // boxes are the plates placeLabels put on screen, in screen px; discs
  // are never knocked out, since a plate never sits over a mark.
  const KNOCK_PX = 3;
  let knocks = [];
  let paintedKnocks = null;
  const knockedOut = ([x, y, bw, bh], px, py) => px > x - KNOCK_PX && px < x + bw + KNOCK_PX && py > y - KNOCK_PX && py < y + bh + KNOCK_PX;
  // One clip holds as many holes as overlap none of each other, since an
  // even-odd hole inside a hole would draw again; holes that do overlap go
  // in a further clip, and clips intersect, so what is left lies outside
  // every plate. A stack of one clip per plate cost a large paint tenfold.
  const clipKnocks = (g, w, h) => {
    const holes = knocks.map(([x, y, bw, bh]) => [x - KNOCK_PX, y - KNOCK_PX, bw + 2 * KNOCK_PX, bh + 2 * KNOCK_PX]);
    const clips = [];
    for (const hole of holes) {
      const free = clips.find((c) => !c.some((o) => overlap(o, hole)));
      if (free) free.push(hole);
      else clips.push([hole]);
    }
    for (const c of clips) {
      const p = new Path2D();
      p.rect(-GROUND_PAD, -GROUND_PAD, w + 2 * GROUND_PAD, h + 2 * GROUND_PAD);
      for (const hole of c) p.rect(...hole);
      g.clip(p, "evenodd");
    }
  };
  // A placing that moved the plates after this frame's paint paints
  // again, once, before the browser shows the frame; a caller that
  // places and then paints costs nothing more.
  const knocksMoved = () => {
    queueMicrotask(() => {
      if (paintedKnocks !== knocks) paintGround();
    });
  };

  const paintGround = () => {
    paintedKnocks = knocks;
    const [w, h] = size();
    const dpr = window.devicePixelRatio || 1;
    const [pw, ph] = [Math.round(w * dpr), Math.round(h * dpr)];
    if (groundEl.width !== pw || groundEl.height !== ph) {
      groundEl.width = pw;
      groundEl.height = ph;
    }
    const g = groundCtx;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    const finding = world.classList.contains("finding");
    const base = finding ? RECEDED : 1;
    const hovered = hover ? `c:${current?.byId.get(hover)?.address}` : null;
    const X = (x) => x * k + tx;
    const Y = (y) => y * k + ty;
    const off = (x0, y0, x1, y1) => x1 < -GROUND_PAD || y1 < -GROUND_PAD || x0 > w + GROUND_PAD || y0 > h + GROUND_PAD;
    for (const layer of [...groundLayers, talkLayer, fedLayer, cellLayer]) {
      // Consecutive shapes in one ink share a path, so a frame strokes
      // once per ink rather than once per line.
      let open = null;
      const finish = () => {
        if (!open) return;
        const ink = open.ink;
        g.globalAlpha = open.alpha * ink.alpha;
        if (open.tag === "circle" && ink.fill) {
          g.fillStyle = ink.fill;
          g.fill();
        }
        if (ink.stroke) {
          g.strokeStyle = ink.stroke;
          g.lineWidth = open.w || ink.width;
          g.lineCap = ink.cap || "butt";
          g.setLineDash(ink.dash);
          g.stroke();
        }
        open = null;
      };
      // A layer is all discs or all lines, so the knock-outs clip a line
      // layer once rather than once per ink. The cells are discs with the
      // odd wall among them, which the names never knock out.
      const knocked = knocks.length > 0 && layer !== cellLayer && layer[0]?.tag === "path";
      if (knocked) {
        g.save();
        clipKnocks(g, w, h);
      }
      for (const s of layer) {
        let shape;
        if (s.tag === "circle") {
          const [cx, cy, r] = [X(s.at[0]), Y(s.at[1]), s.r * k];
          if (off(cx - r, cy - r, cx + r, cy + r)) continue;
          shape = () => {
            g.moveTo(cx + r, cy);
            g.arc(cx, cy, r, 0, 2 * Math.PI);
          };
        } else {
          const [[x1, y1], [x2, y2], [x3, y3]] = s.line.map(([x, y]) => [X(x), Y(y)]);
          if (off(Math.min(x1, x2, x3), Math.min(y1, y2, y3), Math.max(x1, x2, x3), Math.max(y1, y2, y3))) continue;
          shape = () => {
            g.moveTo(x1, y1);
            g.quadraticCurveTo(x2, y2, x3, y3);
          };
        }
        // A cell's parts recede one by one while finding, as its mark did;
        // everything else recedes with its layer.
        const ink = s.key ? inkOf(s.tag, s.cls, s.key === hovered ? `${s.parent} hover` : s.parent) : inkOf(s.tag, s.cls);
        const alpha = s.key ? (finding && !foundSet.has(s.key) ? RECEDED : 1) : base;
        // The line under the pointer is drawn thicker, so what a click
        // would open is seen before it is clicked.
        // A line sized by what it carries still grows when lit.
        const carries = s.w && LIT.test(s.cls || "") ? Math.max(s.w, ink.width) : s.w;
        const w = s.line && s.line === touched ? (carries || ink.width) + TOUCHED_GROWS : carries;
        // Discs are filled one by one, so overlapping grounds don't merge.
        if (!open || open.ink !== ink || open.w !== w || s.tag === "circle") {
          finish();
          open = { ink, w, tag: s.tag, alpha };
          g.beginPath();
        }
        shape();
      }
      finish();
      if (knocked) g.restore();
    }
    g.globalAlpha = 1;
    g.setLineDash([]);
  };

  // What the canvas draws as lines, said for whoever checks the map from
  // outside (the web-design kit's map.js): each cord, hypha, talk and fed arc
  // the canvas has on screen, as a polyline in the viewport's CSS pixels,
  // worked out fresh on each call from the lines the name placer avoids.
  // Where a line is knocked out round a name, it is said as two, ending
  // either side, since nothing is drawn there.
  const LINE_STEPS = 8;
  const LINE_STEP_PX = 3; // the most a said line moves between points
  const lineSteps = (xs, ys) => Math.max(LINE_STEPS, Math.ceil((Math.hypot(xs[1] - xs[0], ys[1] - ys[0]) + Math.hypot(xs[2] - xs[1], ys[2] - ys[1])) / LINE_STEP_PX));
  // How many lines the canvas paints, by kind, said for the site's own
  // checks: the lines across networks, the talk arcs, the talk held, the
  // talk with two ends to draw (talk inside one folded network has one),
  // the feedings, and how many networks are folded into their grounds.
  groundEl.painted = () => ({
    across: (groundLayers[1] || []).length,
    talk: talkLayer.length,
    fed: fedLayer.length,
    said: talk.size,
    between: talkBetween,
    folded: folded.size,
  });
  groundEl.dataset.lines = "";
  groundEl.lines = () => {
    const box = groundEl.getBoundingClientRect();
    const [w, h] = [box.width, box.height];
    const out = [];
    for (const [p, c, q] of [...lineShapes, ...talkShapes, ...fedShapes]) {
      const xs = [p[0], c[0], q[0]].map((x) => x * k + tx);
      const ys = [p[1], c[1], q[1]].map((y) => y * k + ty);
      if (Math.max(...xs) < 0 || Math.max(...ys) < 0 || Math.min(...xs) > w || Math.min(...ys) > h) continue;
      const steps = knocks.length ? lineSteps(xs, ys) : LINE_STEPS;
      let line = [];
      for (let i = 0; i <= steps; i++) {
        const t = i / steps;
        const u = 1 - t;
        const [x, y] = [u * u * xs[0] + 2 * u * t * xs[1] + t * t * xs[2], u * u * ys[0] + 2 * u * t * ys[1] + t * t * ys[2]];
        if (knocks.some((b) => knockedOut(b, x, y))) {
          if (line.length > 1) out.push(line);
          line = [];
          continue;
        }
        line.push({ x: box.left + x, y: box.top + y });
      }
      if (line.length > 1) out.push(line);
    }
    return out;
  };

  // The line under a point of the map, in CSS px from its corner: the
  // nearest within reach, feedings first and then talk, since each is drawn
  // over what comes after it. On a
  // network's ground only the lines that cross grounds count, so the
  // ground itself still opens its network.
  const TOUCH_PX = 6;
  const TOUCHED_GROWS = 2;
  const LIT = /\blit\b/;
  let touched = null;
  const lineAt = (x, y, onGround) => {
    let best = null;
    let nearest = TOUCH_PX;
    for (const shapes of [fedShapes, talkShapes, lineShapes]) {
      for (const line of shapes) {
        if (!line.opens || (onGround && line.within)) continue;
        const [p, c, q] = line.map(([px, py]) => [px * k + tx, py * k + ty]);
        if (x < Math.min(p[0], c[0], q[0]) - nearest || x > Math.max(p[0], c[0], q[0]) + nearest) continue;
        if (y < Math.min(p[1], c[1], q[1]) - nearest || y > Math.max(p[1], c[1], q[1]) + nearest) continue;
        let a = p;
        for (let i = 1; i <= LINE_PIECES; i++) {
          const t = i / LINE_PIECES;
          const u = 1 - t;
          const b = [u * u * p[0] + 2 * u * t * c[0] + t * t * q[0], u * u * p[1] + 2 * u * t * c[1] + t * t * q[1]];
          const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
          const along = Math.max(0, Math.min(1, ((x - a[0]) * dx + (y - a[1]) * dy) / (dx * dx + dy * dy || 1)));
          const d = Math.hypot(x - a[0] - along * dx, y - a[1] - along * dy);
          if (d < nearest) [best, nearest] = [line, d];
          a = b;
        }
      }
      if (best) return best;
    }
    return best;
  };

  // The network whose ground is under a point of the map, the innermost
  // where grounds overlap.
  const groundAt = (x, y) => {
    let best = null;
    for (const [key, gx, gy, r] of groundsAt) {
      if ((x - gx) ** 2 + (y - gy) ** 2 <= r * r && (!best || r < best[1])) best = [key, r];
    }
    return best?.[0] || null;
  };

  // ---- What changed since he last looked. ----

  const SEEN_KEY = "mycelium-seen";
  const readSeen = () => {
    try {
      return JSON.parse(localStorage.getItem(SEEN_KEY) || "null");
    } catch {
      return null;
    }
  };
  let changed = new Set();
  let current = null;
  const judgeChanged = () => {
    const seen = readSeen();
    changed = new Set();
    if (!seen || !current) return;
    for (const s of current.cells) if (seen[s.id] !== s.standing) changed.add(s.id);
  };
  const keepSeen = () => {
    if (!current) return;
    const seen = {};
    for (const s of current.cells) seen[s.id] = s.standing;
    try {
      localStorage.setItem(SEEN_KEY, JSON.stringify(seen));
    } catch {
      // A browser refusing storage only loses the faint rings next visit.
    }
  };

  // ---- Drawing the map. ----

  let L = null; // the layout
  let pos = new Map(); // each cell's place
  let near = null; // what the chosen thing touches: {ids, hubs, links}
  let hover = null;
  // The machines the networks run on are a layer of their own, off until
  // "m" or opening a machine puts it on: where a cell runs says nothing of
  // the network it is part of, so the map is networks and cells alone.
  let machinesShown = false;
  const shownMachines = () => (machinesShown ? current.machines : []);

  // Each mark says which level the arrows walk it on and which network it
  // sits in, so a probe can judge the map by level from outside.
  const TOP_LEVEL = "top";
  const LOOSE_LEVEL = "loose";

  const cellClasses = (s) => {
    const cls = ["cell"];
    if (chosen?.kind === "cell" && chosen.id === s.address) cls.push("chosen");
    if (near && !near.ids.has(s.id)) cls.push("far");
    return cls.join(" ");
  };
  // A cell is painted on the ground layer. The SVG keeps one invisible
  // mark for it, to be clicked, hovered, found and walked by its data-key.
  // The mark says only whether it is chosen: it is never seen, so whether
  // it is far is the paint's to say, and a new choice leaves it alone.
  const cellMarkClass = (s) => (chosen?.kind === "cell" && chosen.id === s.address ? "cell chosen m-hit" : "cell m-hit");
  const cellMarkup = (s, [x, y]) => {
    const net = esc(netOf(s)?.key || "");
    return `<circle class="${cellMarkClass(s)}" data-key="c:${esc(s.address)}" data-level="${net || LOOSE_LEVEL}" data-within="${net}"${s.standing === "asks" ? ' data-asks=""' : ""} cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${R + 4}"/>`;
  };
  // What a cell paints, innermost last: what changed, what waits on it,
  // its membrane and what it is doing.
  const cellShapes = (s, at) => {
    const key = `c:${s.address}`;
    const parent = cellClasses(s);
    const parts = [];
    const disc = (cls, r) => parts.push({ tag: "circle", cls, at, r, key, parent });
    if (changed.has(s.id)) disc("m-changed", R + 4);
    // A receptor is a dish open to the outside, its dot inside: no
    // membrane, so it never reads as a session at work.
    if (s.kind === "receptor") {
      const [x, y] = at;
      parts.push({ tag: "path", cls: "m-receptor", line: [[x - R, y - R * DISH_LIP], [x, y + R * DISH_DEPTH], [x + R, y - R * DISH_LIP]], key, parent });
      disc(s.standing === "moving" ? "m-nucleus" : "m-quiet", QUIET_DOT);
      return parts;
    }
    // A ring round what holds others up: dashed, since it waits rather than
    // works, and wider with each question and with the oldest one's age.
    const grow = swell(s.owes);
    if (grow) disc("m-waits", R + WAIT_GAP + grow);
    // A hold is a wall across the cell's hypha, as a septum closes one:
    // a bar through it, painted under the membrane, so only its ends show.
    // It stands upright, since a cell's name sits beside it.
    if (s.holds.length) {
      const [x, y] = at;
      const reach = R + WAIT_GAP + grow + HOLD_REACH;
      parts.push({ tag: "path", cls: "m-holds", line: [[x, y - reach], [x, y], [x, y + reach]], key, parent });
    }
    if (s.standing === "dormant") disc("m-dormant", R);
    else {
      const ring = s.standing === "asks" ? " m-asks" : s.standing === "stuck" ? " m-stuck" : "";
      disc(`m-membrane${ring}`, R);
      if (s.standing === "moving" || ring) disc("m-nucleus", NUCLEUS);
      else disc("m-quiet", QUIET_DOT);
    }
    return parts;
  };

  // A network is live while any of its cells is at work or waiting on
  // him; one whose cells are all quiet or dormant is drawn in the quieter
  // inks, so where the work is reads before anything is read.
  const LIVE = new Set(["asks", "stuck", "moving"]);
  const isLive = (h) => h.cells.some((s) => LIVE.has(s.standing));
  const quiet = (h) => (isLive(h) ? "" : " quiet");

  const hubClass = (h) => {
    const cls = ["hub"];
    if (chosen?.kind === "net" && chosen.id === h.key) cls.push("chosen");
    if (near && !near.hubs.has(h.key)) cls.push("far");
    if (!isLive(h)) cls.push("quiet");
    return cls.join(" ");
  };
  // A network's hub is a knot: its ring, spokes out of it where its
  // hyphae leave, and its own shape solid in the middle. The receptors'
  // edge holds a dish instead, and a group working alone a bare ring,
  // since neither is a network.
  const hubMarkup = (h, [x, y]) => {
    const middle = h.edge ? `<path class="m-hub-dish" d="${dishPath(HUB_R * 0.55)}"/>` : `<path class="m-hub-shape" d="${SHAPES[h.shape](HUB_R * 0.5)}"/>`;
    return (
      `<g class="${hubClass(h)}" data-key="h:${esc(h.key)}" data-level="${TOP_LEVEL}" data-within=""${h.within ? ` data-branch-of="${esc(h.within)}"` : ""} transform="translate(${x.toFixed(1)},${y.toFixed(1)})">` +
      `<circle class="m-hit" r="${HUB_R + 6}"/>${h.edge || h.alone ? "" : `<path class="m-spokes" d="${spokesPath(HUB_R)}"/>`}<circle class="m-hub" r="${HUB_R}"/>` +
      `${middle}</g>`
    );
  };

  // A network nobody leads, shared by cells under two networks or more,
  // is a knot of its own: the same spokes and ring, empty in the middle
  // since nobody leads it. It stands among its cells, clear of every
  // network's ground, so its cells are the bridge between the networks
  // they're under and no line ever runs from one network's mark to
  // another's. Shared by cells of one network only, it stays the faint
  // hyphae inside that network's ground.
  let knotsAt = new Map();
  const KNOT_CLEAR = HUB_R + 14; // from a ground's edge to a knot's middle
  const KNOT_TRIES = 8; // pushes out of grounds before a knot stays put
  const knotClass = (n) => {
    const cls = ["knot"];
    if (chosen?.kind === "net" && chosen.id === n.key) cls.push("chosen");
    if (near && !near.shared.has(n.key)) cls.push("far");
    if (!isLive(n)) cls.push("quiet");
    return cls.join(" ");
  };
  const knotMarkup = (n, [x, y]) =>
    `<g class="${knotClass(n)}" data-key="n:${esc(n.key)}" data-level="${TOP_LEVEL}" data-within="" transform="translate(${x.toFixed(1)},${y.toFixed(1)})">` +
    `<circle class="m-hit" r="${HUB_R + 6}"/><path class="m-spokes" d="${spokesPath(HUB_R)}"/><circle class="m-hub" r="${HUB_R}"/></g>`;
  const homesOf = (n) => new Set(n.cells.map((s) => (s.hub || s.loose)?.key).filter(Boolean));
  // Each knot starts at the middle of its cells and is pushed straight out
  // of any ground, machine or knot it lands on, a few times at most.
  const placeKnots = (m, machines) => {
    const at = new Map();
    const taken = [...groundsAt.map(([, x, y, r]) => [x, y, r + KNOT_CLEAR]), ...machines.map((x) => [...(L.machines.get(x.name) || [0, 0]), MACHINE_R + KNOT_CLEAR])];
    for (const n of m.shared) {
      const on = n.cells.filter((s) => pos.has(s.id));
      if (on.length < 2 || homesOf(n).size < 2) continue;
      let [x, y] = on.reduce(([a, b], s) => [a + pos.get(s.id)[0] / on.length, b + pos.get(s.id)[1] / on.length], [0, 0]);
      for (let i = 0; i < KNOT_TRIES; i++) {
        const hit = taken.find(([tx, ty, r]) => Math.hypot(x - tx, y - ty) < r);
        if (!hit) break;
        const [tx, ty, r] = hit;
        const d = Math.hypot(x - tx, y - ty);
        const [ux, uy] = d > 1 ? [(x - tx) / d, (y - ty) / d] : [Math.cos(Math.atan2(ty, tx)), Math.sin(Math.atan2(ty, tx))];
        [x, y] = [tx + ux * (r + 1), ty + uy * (r + 1)];
      }
      at.set(n.key, [x, y]);
      taken.push([x, y, HUB_R * 2 + KNOT_CLEAR]);
    }
    return at;
  };

  const machineClass = (x) => (near && !near.machines.has(x.name) ? "machine far" : "machine");
  // A machine's mark is its name, boxed, drawn with the words on the map;
  // here it keeps only the place the keys and the walk stand on.
  const machineMarkup = (x, [mx, my]) =>
    `<g class="${machineClass(x)}" data-key="m:${esc(x.name)}" data-level="${TOP_LEVEL}" data-within="" transform="translate(${mx},${my})">` +
    `<circle class="m-hit" r="${MACHINE_R}"/></g>`;

  // Each phase of a redraw is measured under its own name, so where the
  // time goes at any size reads from outside, layout included.
  const timed = (name, fn) => {
    const started = performance.now();
    fn();
    performance.clearMeasures(name);
    performance.measure(name, { start: started });
  };

  // A ground shape keeps what its ink is made of: its own classes, and a
  // test of whether the choice touches it, asked again on every draw. What
  // the choice leaves untouched steps back; a hypha or cross-hypha it
  // touches is also drawn strong.
  // A quiet shape is drawn faint until the choice touches it: the lines
  // that join cells across networks, which drawn in full hid the cells.
  const inked = (base, on, tail = "", strong = false, quiet = false) => ({ base, on, tail, strong, quiet });
  const inkOfShape = (s) => {
    const on = near && s.on();
    return `${s.base}${near ? (on ? (s.strong ? " lit" : "") : " far") : ""}${s.quiet && !on ? " faint" : ""}${s.tail}`;
  };
  // What is far is painted first, so what the choice touches lies on top.
  const farFirst = (a, b) => (b.cls.includes(" far") ? 1 : 0) - (a.cls.includes(" far") ? 1 : 0);
  const inkGround = () => {
    for (const layer of groundLayers) {
      for (const s of layer) s.cls = inkOfShape(s);
      layer.sort(farFirst);
    }
  };
  const paintCells = () => {
    cellLayer = [];
    for (const s of current.cells) if (pos.has(s.id) && !isFolded(s)) cellLayer.push(...cellShapes(s, pos.get(s.id)));
  };
  const sameSet = (a, b) => a.size === b.size && [...a].every((key) => b.has(key));
  // The trees hyphae grow, kept for the reading and layout they grew in:
  // which cells a tree joins says the rest, so a new choice or fold
  // reuses them rather than growing every tree again.
  let treesFor = null;
  const treeOf = (key, points) => {
    if (treesFor?.m !== current || treesFor.L !== L) treesFor = { m: current, L, trees: new Map() };
    let tree = treesFor.trees.get(key);
    if (!tree) {
      tree = shortestTree(points);
      treesFor.trees.set(key, tree);
    }
    return tree;
  };

  // A new choice on the map as drawn keeps every mark where it is and inks
  // again only what the choice changes, so a Return at thousands of cells
  // costs classes rather than a fresh map of them. A new reading, layout
  // or fold draws the whole map.
  let drawnFor = null;
  const restyleMarks = () => {
    const setClass = (el, cls) => {
      if (el.getAttribute("class") !== cls) el.setAttribute("class", cls);
    };
    for (const el of world.querySelectorAll("g.hub")) setClass(el, hubClass(current.hubs.get(el.dataset.key.slice(2))));
    const shared = new Map(current.shared.map((n) => [n.key, n]));
    for (const el of world.querySelectorAll("g.knot")) setClass(el, knotClass(shared.get(el.dataset.key.slice(2))));
    const machines = new Map(current.machines.map((x) => [x.name, x]));
    for (const el of world.querySelectorAll("g.machine")) setClass(el, machineClass(machines.get(el.dataset.key.slice(2))));
    for (const el of world.querySelectorAll(".cell.chosen")) setClass(el, "cell m-hit");
    if (chosen?.kind === "cell") world.querySelector(`[data-key="c:${CSS.escape(chosen.id)}"]`)?.setAttribute("class", "cell chosen m-hit");
  };
  const restyle = () => {
    inkGround();
    paintCells();
    restyleMarks();
  };
  // The same map folded another way keeps its marks too: a network that
  // folded takes its cells' marks away, one that unfolded adds them.
  const refoldMarks = (marks, was) => {
    restyleMarks();
    for (const key of folded) if (!was.has(key)) for (const el of marks.querySelectorAll(`circle.cell[data-within="${CSS.escape(key)}"]`)) el.remove();
    const opened = new Set([...was].filter((key) => !folded.has(key)));
    if (!opened.size) return;
    const added = current.cells.filter((s) => opened.has(homeOf(s)) && pos.has(s.id)).map((s) => cellMarkup(s, pos.get(s.id)));
    marks.insertAdjacentHTML("beforeend", added.join(""));
  };
  // What every draw ends with, whole or a restyle.
  const drawnAgain = () => {
    svg.classList.toggle("focused", !!near);
    if (pointed && !spots.has(pointed)) pointed = null;
    paintTalk({ paint: false });
    placePointer();
    labelsFor();
    if (auto) fit(false);
    else frame();
  };

  const draw = () => timed("mycelium-draw", drawMap);
  const drawMap = () => {
    if (!current) return;
    const m = current;
    L = lay(m);
    pos = placed(m, L);
    near = chosen ? nearOf(chosen) : null;
    const folds = foldsNow();
    const same = drawnFor?.m === m && drawnFor.L === L && drawnFor.machines === machinesShown;
    if (same && sameSet(folds, folded)) {
      restyle();
      drawnAgain();
      return;
    }
    const marks = same ? world.querySelector("g.nodes") : null;
    const was = folded;
    drawnFor = { m, L, machines: machinesShown };
    const machines = shownMachines();
    const cords = [];
    const hyphae = [];
    const cross = [];
    const nodes = [];
    // The network as a graph too, for the keys to walk: every machine, hub
    // and cell a node, its cords and hyphae one kind of edge and its
    // cross-hyphae another.
    spots = new Map();
    sizes = new Map();
    lineShapes = [];
    treeAdj = new Map();
    crossAdj = new Map();
    for (const x of machines) {
      spots.set(`m:${x.name}`, L.machines.get(x.name));
      sizes.set(`m:${x.name}`, MACHINE_R);
    }
    for (const h of m.hubs.values()) {
      spots.set(`h:${h.key}`, L.at.get(h.key));
      sizes.set(`h:${h.key}`, HUB_R);
    }
    folded = folds;
    for (const s of m.cells) {
      if (!pos.has(s.id) || isFolded(s)) continue;
      spots.set(`c:${s.address}`, pos.get(s.id));
      sizes.set(`c:${s.address}`, R);
    }
    const cellKey = (id) => `c:${m.byId.get(id).address}`;
    // Cords: machine to each top hub, and each hub to its branches; a cord
    // thickens with the cells it carries.
    // Each hub's ground, a faint disc its cells are sown in, so whose cell
    // is whose reads at a glance however close two colonies sit.
    const colonies = [];
    groundsAt = [];
    for (const h of m.hubs.values()) {
      const at = L.at.get(h.key);
      const r = colonyRadius(h) + COLONY_PAD;
      groundsAt.push([h.key, at[0], at[1], r]);
      colonies.push({ tag: "circle", ...inked("m-colony", () => near.hubs.has(h.key), `${quiet(h)}${folded.has(h.key) ? " folded" : ""}`), at, r });
      // A network holding others up is ringed round its whole ground, so it
      // reads from the whole map, where a cell's own ring is too small.
      const grow = swell(h.owes);
      if (grow) colonies.push({ tag: "circle", ...inked("m-waits m-held", () => near.hubs.has(h.key)), at, r: r + WAIT_GAP + grow });
    }
    knotsAt = placeKnots(m, machines);
    for (const [key, at] of knotsAt) {
      spots.set(`n:${key}`, at);
      sizes.set(`n:${key}`, HUB_R);
    }
    // A machine carrying more than CORD_BUNDLE_OVER networks bundles its
    // cords as a mycelium does: one trunk out each way, splitting into a
    // twig for each network, so a crowd reads as parts, not a dark mass.
    const cordWeight = (cells) => Math.min(CORD_MOST, Math.max(1, 1.5 + Math.log2(1 + cells) * 0.9));
    // A cross-hypha is as nourished as the network it carries is wide: the
    // dots of a repository shared by fifty cells stand larger than a pair's.
    const crossWeight = (cells) => Math.min(CROSS_MOST, 1.25 + Math.log2(1 + cells) * 0.45);
    const tops = new Map();
    for (const h of m.hubs.values()) if (!h.within && h.machine) tops.set(h.machine, (tops.get(h.machine) || 0) + 1);
    const trunks = new Map();
    for (const h of m.hubs.values()) {
      const at = L.at.get(h.key);
      const weight = cordWeight(h.cells.length);
      if (h.within) {
        link(treeAdj, `h:${h.within}`, `h:${h.key}`);
        cords.push({ tag: "path", ...inked("m-cord", () => near.hubs.has(h.key) && near.hubs.has(h.within), quiet(h)), w: weight, line: opens(curve(L.at.get(h.within), at, 0.08), "net", h.key) });
      } else if (!machinesShown) {
        // With no machine drawn, a top network hangs from nothing.
      } else if ((tops.get(h.machine) || 0) > CORD_BUNDLE_OVER) {
        link(treeAdj, `m:${h.machine}`, `h:${h.key}`);
        const turn = (((L.angle.get(h.key) + Math.PI / 2) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
        const way = `${h.machine}|${Math.floor(turn / ((2 * Math.PI) / BUNDLE_WAYS))}`;
        if (!trunks.has(way)) trunks.set(way, { machine: h.machine, hubs: [] });
        trunks.get(way).hubs.push(h);
      } else {
        const root = L.machines.get(h.machine) || [0, 0];
        if (h.machine) link(treeAdj, `m:${h.machine}`, `h:${h.key}`);
        cords.push({ tag: "path", ...inked("m-cord", () => near.hubs.has(h.key) && near.root && near.machines.has(h.machine), quiet(h)), w: weight, line: opens(curve(root, at, 0.06), "net", h.key) });
      }
    }
    // A machine feeding a network it isn't the root of draws its own cord
    // to the network's hub, weighted by the cells it runs there.
    for (const h of m.hubs.values()) {
      if (h.within || !machinesShown) continue;
      for (const [machine, count] of h.feeds) {
        if (machine === h.machine || !L.machines.has(machine)) continue;
        link(crossAdj, `m:${machine}`, `h:${h.key}`);
        cords.push({ tag: "path", ...inked("m-cord m-feed", () => near.hubs.has(h.key) && near.machines.has(machine), quiet(h)), w: cordWeight(count), line: opens(curve(L.machines.get(machine), L.at.get(h.key), -0.06), "net", h.key) });
      }
    }
    for (const { machine, hubs } of trunks.values()) {
      const root = L.machines.get(machine) || [0, 0];
      // The trunk ends at the mean of its networks' bearings, part way out.
      const [cx, cy] = hubs.reduce(([x, y], h) => [x + Math.cos(L.angle.get(h.key)), y + Math.sin(L.angle.get(h.key))], [0, 0]);
      const a = Math.atan2(cy, cx);
      const fork = [Math.cos(a) * L.ring * BUNDLE_REACH, Math.sin(a) * L.ring * BUNDLE_REACH];
      const lit = () => near.root && near.machines.has(machine) && hubs.some((h) => near.hubs.has(h.key));
      const live = hubs.some(isLive) ? "" : " quiet";
      cords.push({ tag: "path", ...inked("m-cord", lit, live), w: cordWeight(hubs.reduce((n, h) => n + h.cells.length, 0)), line: opens(curve(root, fork, 0.04), "machine", machine) });
      for (const h of hubs) cords.push({ tag: "path", ...inked("m-cord m-twig", () => lit() && near.hubs.has(h.key), quiet(h)), w: 1, line: opens(curve(fork, L.at.get(h.key), 0.04), "net", h.key) });
    }
    for (const h of m.hubs.values()) {
      const at = L.at.get(h.key);
      // Hyphae branching from the hub out to each of its cells.
      const list = folded.has(h.key) ? [] : (seeds.get(h.key) || []).filter((id) => id && pos.has(id));
      const points = [at, ...list.map((id) => pos.get(id))];
      for (const [a, b] of treeOf(`h|${h.key}|${list.length}`, points)) {
        const ida = a === 0 ? null : list[a - 1];
        const idb = list[b - 1];
        const ka = ida ? cellKey(ida) : `h:${h.key}`;
        link(treeAdj, ka, cellKey(idb));
        hyphae.push({ tag: "path", ...inked("m-hypha", () => (near.ids.has(idb) || (ida && near.ids.has(ida))) && near.hyphae.has(idb), "", true), line: opens(curve(points[a], points[b], 0.1), "cell", cellKey(idb).slice(2), true) });
      }
    }
    // Cross-hyphae: the cells of each shared network, joined by the
    // shortest tree, from its knot where it has one, and a cell on a
    // second orchestration joined to its hub.
    for (const n of m.shared) {
      const knot = knotsAt.get(n.key);
      // A folded network's cells share through its ground: the line runs to
      // the ground's edge nearest the knot, once for all its cells, so a
      // share is never lost to a fold.
      const ends = [];
      const grounds = new Set();
      for (const s of n.cells) {
        if (!pos.has(s.id)) continue;
        if (!isFolded(s)) {
          ends.push([`c:${s.address}`, pos.get(s.id)]);
          continue;
        }
        const home = homeOf(s);
        if (!knot || grounds.has(home)) continue;
        grounds.add(home);
        const hub = m.hubs.get(home);
        const [hx, hy] = L.at.get(home);
        const d = Math.hypot(knot[0] - hx, knot[1] - hy) || 1;
        const r = Math.min(d, hub ? colonyRadius(hub) : HUB_R);
        ends.push([`h:${home}`, [hx + ((knot[0] - hx) * r) / d, hy + ((knot[1] - hy) * r) / d]]);
      }
      const points = [...(knot ? [knot] : []), ...ends.map(([, p]) => p)];
      const at = (i) => (!knot ? ends[i][0] : i === 0 ? `n:${n.key}` : ends[i - 1][0]);
      for (const [a, b] of treeOf(`s|${n.key}|${knot ? "knot|" : ""}${ends.map(([key]) => key).join(",")}`, points)) {
        link(crossAdj, at(a), at(b));
        cross.push({ tag: "path", ...inked(`m-cross m-${n.shape}`, () => near.shared.has(n.key), "", true, true), w: crossWeight(n.cells.length), line: opens(curve(points[a], points[b], 0.18), "net", n.key) });
      }
    }
    for (const s of m.cells) {
      if (!pos.has(s.id) || isFolded(s)) continue;
      for (const h of s.also) {
        link(crossAdj, `c:${s.address}`, `h:${h.key}`);
        cross.push({ tag: "path", ...inked("m-cross m-bridge", () => near.ids.has(s.id) && near.hubs.has(h.key), "", true, true), w: crossWeight(1), line: opens(curve(pos.get(s.id), L.at.get(h.key), 0.14), "net", h.key) });
      }
    }
    // Machines joined to each other: the link that keeps a realm one.
    const machinesAt = machines.map((x) => L.machines.get(x.name));
    for (let i = 1; i < machinesAt.length; i++) {
      const [a, b] = [`m:${machines[i - 1].name}`, `m:${machines[i].name}`];
      link(treeAdj, a, b);
      const [p, q] = [machinesAt[i - 1], machinesAt[i]];
      const line = [p, [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2], q];
      lineShapes.push(line);
      cords.push({ tag: "path", ...inked("m-link", () => false), line });
    }
    if (!marks) {
      machines.forEach((x) => nodes.push(machineMarkup(x, L.machines.get(x.name))));
      for (const h of m.hubs.values()) nodes.push(hubMarkup(h, L.at.get(h.key)));
      for (const n of m.shared) if (knotsAt.has(n.key)) nodes.push(knotMarkup(n, knotsAt.get(n.key)));
      for (const s of m.cells) if (pos.has(s.id) && !isFolded(s)) nodes.push(cellMarkup(s, pos.get(s.id)));
    }
    paintCells();
    groundLayers = [colonies, cross, cords, hyphae];
    inkGround();
    if (marks) refoldMarks(marks, was);
    else
      world.innerHTML =
        `<g class="nodes">${nodes.join("")}</g>` +
        `<g id="pointer" class="pointer" visibility="hidden"><circle class="p-halo"/><circle class="p-ring"/></g>`;
    drawnAgain();
  };

  // ---- Words on the map, kept at reading size whatever the zoom. ----

  // Each label belongs to a point of the map and is drawn in screen pixels,
  // so names stay 14 and 16 px as the map zooms under them. Every frame
  // puts each on the first side of its point where it covers no other
  // label and no hub or machine, trying first the way out from the
  // machines; a cell's name that fits nowhere waits until there's room.
  const LINE = 19;
  // Sixteen ways round a mark: eight left too few gaps between the cords
  // round a busy hub.
  const COMPASS_WAYS = 16;
  const COMPASS = Array.from({ length: COMPASS_WAYS }, (_, i) => [Math.cos((i * 2 * Math.PI) / COMPASS_WAYS), Math.sin((i * 2 * Math.PI) / COMPASS_WAYS)]);
  let labels = [];
  const labelsFor = () => {
    const m = current;
    const out = [];
    // Once networks fold, only the busiest are named, with what's unfolded,
    // chosen or pointed at; the rest are counted on their machine's plate.
    const capped = folded.size > 0;
    const busiest = new Set(
      capped
        ? [...m.hubs.values()]
            .map((h) => [h, countsOf(h.cells)])
            .sort(([p, a], [q, b]) => b.asks - a.asks || b.stuck - a.stuck || b.moving - a.moving || q.cells.length - p.cells.length)
            .slice(0, HUB_NAMES_MOST)
            .map(([h]) => h.key)
        : [],
    );
    for (const h of m.hubs.values()) {
      const key = `h:${h.key}`;
      if (capped && folded.has(h.key) && !busiest.has(h.key) && key !== pointed && !(chosen?.kind === "net" && chosen.id === h.key)) continue;
      // Once something is open, only what it touches keeps a name: a faded
      // name elsewhere is a word he isn't working on.
      if (near && !near.hubs.has(h.key) && key !== pointed) continue;
      const a = L.angle.get(h.key);
      const n = countsOf(h.cells);
      const sub = [plural(h.cells.length, kindWord(h.cells)), ...["asks", "stuck"].filter((s) => n[s]).map((s) => `${n[s]} ${STANDING_WORD[s]}`)].join(" \u00b7 ");
      const busy = [n.asks, n.stuck, n.moving, h.cells.length];
      out.push({ key: `h:${h.key}`, kind: "hub", rank: 1, busy, at: L.at.get(h.key), r: HUB_R, disc: L.disc.get(h.key), out: [Math.cos(a), Math.sin(a)], name: lines(clipped(h.name), 22), sub, far: near && !near.hubs.has(h.key) });
    }
    // A knot is named by what its cells share, or by its quest's title.
    for (const n of m.shared) {
      const at = knotsAt.get(n.key);
      const key = `n:${n.key}`;
      const open = chosen?.kind === "net" && chosen.id === n.key;
      if (!at || (near && !near.shared.has(n.key) && key !== pointed) || (capped && key !== pointed && !open)) continue;
      const c = countsOf(n.cells);
      const sub = [SHARED_WORD[n.shape], plural(n.cells.length, "cell"), ...["asks", "stuck"].filter((s) => c[s]).map((s) => `${c[s]} ${STANDING_WORD[s]}`)].join(" \u00b7 ");
      const a = Math.atan2(at[1], at[0]);
      out.push({ key, kind: "hub", rank: 1, busy: [c.asks, c.stuck, c.moving, n.cells.length], at, r: HUB_R, disc: HUB_R, out: [Math.cos(a), Math.sin(a)], name: lines(clipped(n.title || n.short), 22), sub });
    }
    const counts = new Map();
    for (const s of m.cells) counts.set(s.machine, (counts.get(s.machine) || 0) + 1);
    // A machine's name is its mark, so it shows in full whatever is open,
    // as a hub's disc does; only when the choice touches it does it say
    // what it carries.
    for (const x of shownMachines()) {
      const key = `m:${x.name}`;
      const far = near && !near.machines.has(x.name) && key !== pointed;
      const off = x.state !== "ready";
      const strong = key === pointed || (chosen?.kind === "machine" && chosen.id === x.name);
      const mark = { key, kind: "machine", mark: true, form: machineForm(x), off, strong, rank: -1, at: L.machines.get(x.name), r: MACHINE_R, name: [x.name], mono: true };
      if (far) {
        out.push({ ...mark, sub: off ? "off" : "" });
        continue;
      }
      const p = x.pressure;
      const n = counts.get(x.name) || 0;
      const nets = capped ? [...m.hubs.values()].filter((h) => !h.within && h.machine === x.name).length : 0;
      const carried = nets ? `${plural(n, "cell")} \u00b7 ${plural(nets, "network")}` : plural(n, "cell");
      // A basin that runs with some of its machines off says how many.
      const down = offWords(x);
      const sub = x.state !== "ready" ? "off" : [carried, down, p ? `cpu ${Math.round(p.cpu)}%` : ""].filter(Boolean).join(" \u00b7 ");
      out.push({ ...mark, sub });
    }
    // One tier of names at a time, so the map is never a wall of words:
    // the whole map names machines and networks; a cell is named only when
    // it is chosen, pointed at, hovered or found, or when it needs him and
    // the map is close enough in to read it as its own. Stuck and waiting
    // cells say so by their rings, and their names are a hover away.
    const named = new Set();
    for (const s of m.cells) if (s.standing === "asks") named.add(s.address);
    if (chosen?.kind === "cell") named.add(chosen.id);
    if (chosen?.kind === "talk") for (const end of chosen.id.split("|")) if (end.startsWith("c:")) named.add(end.slice(2));
    for (const key of foundSet) if (key[0] === "c") named.add(key.slice(2));
    const hoverAddress = hover ? m.byId.get(hover)?.address : undefined;
    if (hoverAddress) named.add(hoverAddress);
    if (pointed?.startsWith("c:")) named.add(pointed.slice(2));
    for (const address of named) {
      const s = m.byAddress.get(address);
      const p = s && !isFolded(s) && pos.get(s.id);
      if (!p) continue;
      const key = `c:${address}`;
      const strong = (chosen?.kind === "cell" && chosen.id === address) || (chosen?.kind === "talk" && chosen.id.split("|").includes(key)) || key === pointed;
      const sub = s.standing === "asks" ? "needs you" : s.standing === "stuck" ? "stuck" : s.owes.length ? waitingWords(s.owes) : s.holds.length ? "holding" : "";
      // What he hovers is named beyond the cap, as what he points at is.
      const hovered = address === hoverAddress;
      const sought = foundSet.has(key);
      // A cell that needs him is named before any network, among the seven,
      // unless something is open and the cell lies outside what it touches.
      const asking = s.standing === "asks" && !(near && !near.ids.has(s.id));
      // An asking cell no one sought is named only once it is drawn
      // rather than folded (placeLabels).
      const small = s.standing === "asks" && !(strong || hovered || sought);
      out.push({ key, kind: "cell", rank: strong || hovered ? 0 : asking ? NEEDS_YOU_RANK : sought ? 2 : 4, at: p, r: R, out: [1, 0], name: [clipped(s.short)], sub, strong, small });
    }
    const open = chosen && (chosen.kind === "net" && knotsAt.has(chosen.id) ? "n" : { cell: "c", net: "h", machine: "m" }[chosen.kind]);
    const openKey = open && `${open}:${chosen.id}`;
    for (const l of out) if (!l.mark && (l.key === pointed || l.key === openKey)) l.rank = 0;
    // Cells carry no busyness, so they tie by address: with more asking
    // cells than names, the same ones keep their names from one reading to
    // the next rather than jumping with the feed's order.
    out.sort((a, b) => a.rank - b.rank || busier(a.busy, b.busy) || (a.kind === "cell" && b.kind === "cell" ? a.key.localeCompare(b.key) : 0));
    // A machine open can touch hundreds of networks: only the busiest of
    // them keep a name, after the machines' marks and what is open and
    // pointed at.
    if (near && out.length > NAMES_OPEN) out.length = Math.max(NAMES_OPEN, out.filter((l) => l.mark).length);
    labels = out;
    labelsEl.innerHTML = out
      .map((l) => {
        const name = l.name.map((t, j) => `<tspan x="0" dy="${j ? LINE : 0}">${esc(t)}</tspan>`).join("");
        const below = (l.name.length - 1) * LINE + (l.mark ? MARK_SUB_Y : NAME_SUB_Y);
        const sub = l.sub ? `<text class="t-sub" y="${below}">${esc(l.sub)}</text>` : "";
        const here = l.mark && l.key === pointed && pointing();
        const cls = `tag t-${l.kind}${l.mark ? " t-mark" : ""}${l.form ? ` t-${l.form}` : ""}${here ? " here" : ""}${l.off ? " off" : ""}${l.far ? " far" : ""}${l.strong ? " strong" : ""}${l.rank === 0 ? " engaged" : ""}${foundSet.has(l.key) ? " match" : ""}`;
        const words = `<text class="t-name${l.mono ? " mono" : ""}">${name}</text>`;
        if (!l.mark) return `<g class="${cls}" data-name="${esc(l.key)}"><rect class="t-plate" rx="3"/>${words}${sub}</g>`;
        // A mark takes the pointer itself, and its box is drawn round its
        // name alone, so what it carries hangs outside it: the box and the
        // counts are two names, and the cords between them run free.
        const counts = l.sub ? `<text class="t-sub" y="${below}" data-name="${esc(l.key)} counts">${esc(l.sub)}</text>` : "";
        return `<g class="${cls}" data-mark="${esc(l.key)}"><g data-name="${esc(l.key)}"><rect class="t-plate" rx="0"/><rect class="t-box"/>${words}</g>${counts}</g>`;
      })
      .join("");
    measureLabels();
  };
  // Every box is read before any plate is written: a read after a write
  // makes the browser lay the whole map out again, once per name.
  // A name's box depends only on its words and its kind, so each is
  // measured once: measuring lays out the whole map, which at thousands of
  // cells costs more than everything else a redraw does.
  const boxCache = new Map();
  const BOX_CACHE_MAX = 4000;
  // A box taken in a fallback face is wrong once the real one arrives.
  document.fonts?.addEventListener("loadingdone", () => boxCache.clear());
  const boxKey = (l) => `${l.kind}|${l.mono ? 1 : 0}|${l.strong ? 1 : 0}|${l.name.join("\n")}|${l.sub || ""}`;
  const measureLabels = () => {
    const nodes = labelsEl.children;
    if (boxCache.size > BOX_CACHE_MAX) boxCache.clear();
    const boxes = labels.map((l, i) => {
      const key = boxKey(l);
      if (!boxCache.has(key)) {
        const { x, y, width, height } = nodes[i].getBBox();
        const n = labels[i].mark && nodes[i].querySelector(".t-name").getBBox();
        const s = labels[i].mark && nodes[i].querySelector(".t-sub")?.getBBox();
        const plain = (r) => r && { x: r.x, y: r.y, width: r.width, height: r.height };
        boxCache.set(key, { x, y, width, height, name: plain(n), sub: plain(s) });
      }
      return boxCache.get(key);
    });
    // A shut machine's box is whole; a lent one keeps only its corners,
    // open on every side, and a basin's is open at the top. The dashes
    // follow the box's own size, so they are set where the box is, and
    // inline, so they hold over the dashes an off machine is drawn in.
    const outline = (rect, form, [, , w, h]) => {
      if (form === "lent") {
        const c = Math.min(w, h) * LENT_CORNER;
        rect.style.strokeDasharray = `${(2 * c).toFixed(1)} ${(w - 2 * c).toFixed(1)} ${(2 * c).toFixed(1)} ${(h - 2 * c).toFixed(1)}`;
        rect.style.strokeDashoffset = c.toFixed(1);
      } else if (form === "basin") {
        rect.style.strokeDasharray = `0 ${w.toFixed(1)} ${(w + 2 * h).toFixed(1)}`;
      }
    };
    const fit = (rect, [x, y, w, h]) => {
      rect.setAttribute("x", x.toFixed(1));
      rect.setAttribute("y", y.toFixed(1));
      rect.setAttribute("width", w.toFixed(1));
      rect.setAttribute("height", h.toFixed(1));
    };
    boxes.forEach((b, i) => {
      const l = labels[i];
      const padded = (r, pad) => [r.x - pad, r.y - pad, r.width + 2 * pad, r.height + 2 * pad];
      l.box = padded(b, l.mark ? MARK_PAD : PLATE_PAD);
      if (l.mark) {
        l.markBox = padded(b.name, MARK_PAD);
        l.subBox = b.sub ? padded(b.sub, PLATE_PAD) : null;
        const box = nodes[i].querySelector(".t-box");
        fit(box, l.markBox);
        outline(box, l.form, l.markBox);
      }
      fit(nodes[i].querySelector(".t-plate"), l.mark ? l.markBox : l.box);
    });
  };
  const NAMES_OPEN = 12; // the most names on the map once something is open
  // Busiest first: asks, then stuck, then moving, then the most cells.
  const busier = (a, b) => {
    if (!a || !b) return 0;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return b[i] - a[i];
    return 0;
  };
  const PLATE_PAD = 3;
  // A machine's box stands clear of its name by this much, and what it
  // carries hangs this far under the name, past the box; another name's
  // carriage sits closer.
  const MARK_PAD = 6;
  const MARK_SUB_Y = 26;
  const NAME_SUB_Y = 18;
  const NAMES_ON_SCREEN = 7; // names at once beside the one he's engaged with
  const NEEDS_YOU_RANK = 0.5; // after what he's engaged with, before every network
  const HUB_NAMES_MOST = 40; // networks named once folding starts, busiest first

  // The circles on screen, in buckets of the screen, so a label can ask
  // which it would cover without asking every cell.
  const BUCKET = 64;
  const circlesOnScreen = (w, h) => {
    const grid = new Map();
    const add = (key, [x, y], r) => {
      const sx = x * k + tx;
      const sy = y * k + ty;
      const sr = r * k + 4;
      if (sx < -sr || sy < -sr || sx > w + sr || sy > h + sr) return;
      for (let gx = Math.floor((sx - sr) / BUCKET); gx <= Math.floor((sx + sr) / BUCKET); gx++)
        for (let gy = Math.floor((sy - sr) / BUCKET); gy <= Math.floor((sy + sr) / BUCKET); gy++) {
          const b = `${gx},${gy}`;
          if (!grid.has(b)) grid.set(b, []);
          grid.get(b).push([sx, sy, sr, key]);
        }
    };
    // A hub's spokes are part of its mark, so a name keeps clear of them too.
    const reachOf = (key) => sizes.get(key) + (key[0] === "h" || key[0] === "n" ? SPOKE_FROM + SPOKE_LONG : 0);
    for (const [key, at] of spots) add(key, at, reachOf(key));
    return grid;
  };
  // Its own circle too: a name sits beside its mark, never on it.
  const coversCircle = (grid, [x, y, bw, bh]) => {
    for (let gx = Math.floor(x / BUCKET); gx <= Math.floor((x + bw) / BUCKET); gx++)
      for (let gy = Math.floor(y / BUCKET); gy <= Math.floor((y + bh) / BUCKET); gy++)
        for (const [cx, cy, r] of grid.get(`${gx},${gy}`) || []) {
          const dx = cx - Math.max(x, Math.min(cx, x + bw));
          const dy = cy - Math.max(y, Math.min(cy, y + bh));
          if (dx * dx + dy * dy < r * r) return true;
        }
    return false;
  };

  // The lines on screen as short straight pieces, in the same buckets, so
  // a name can ask which it would lie over.
  const LINE_PIECES = 8;
  const linesOnScreen = (w, h) => {
    const grid = new Map();
    const on = ([x, y]) => [x * k + tx, y * k + ty];
    for (const [p0, c0, q0] of [...lineShapes, ...talkShapes, ...fedShapes]) {
      const [p, c, q] = [on(p0), on(c0), on(q0)];
      let a = p;
      for (let i = 1; i <= LINE_PIECES; i++) {
        const t = i / LINE_PIECES;
        const u = 1 - t;
        const b = [u * u * p[0] + 2 * u * t * c[0] + t * t * q[0], u * u * p[1] + 2 * u * t * c[1] + t * t * q[1]];
        const [x0, x1] = [Math.min(a[0], b[0]), Math.max(a[0], b[0])];
        const [y0, y1] = [Math.min(a[1], b[1]), Math.max(a[1], b[1])];
        if (x1 >= 0 && y1 >= 0 && x0 <= w && y0 <= h)
          for (let gx = Math.floor(x0 / BUCKET); gx <= Math.floor(x1 / BUCKET); gx++)
            for (let gy = Math.floor(y0 / BUCKET); gy <= Math.floor(y1 / BUCKET); gy++) {
              const key = `${gx},${gy}`;
              if (!grid.has(key)) grid.set(key, []);
              grid.get(key).push([a, b]);
            }
        a = b;
      }
    }
    return grid;
  };
  // Whether a straight piece crosses a box: clipped to it, Liang-Barsky.
  const pieceInBox = ([[x1, y1], [x2, y2]], [bx, by, bw, bh]) => {
    const [dx, dy] = [x2 - x1, y2 - y1];
    let t0 = 0;
    let t1 = 1;
    for (const [p, q] of [[-dx, x1 - bx], [dx, bx + bw - x1], [-dy, y1 - by], [dy, by + bh - y1]]) {
      if (p === 0) {
        if (q < 0) return false;
        continue;
      }
      const r = q / p;
      if (p < 0) t0 = Math.max(t0, r);
      else t1 = Math.min(t1, r);
      if (t0 > t1) return false;
    }
    return true;
  };
  // How many pieces of line a box would lie over, each counted once though
  // it may sit in several buckets.
  const linesUnder = (grid, box) => {
    const [x, y, bw, bh] = box;
    const under = new Set();
    for (let gx = Math.floor(x / BUCKET); gx <= Math.floor((x + bw) / BUCKET); gx++)
      for (let gy = Math.floor(y / BUCKET); gy <= Math.floor((y + bh) / BUCKET); gy++)
        for (const piece of grid.get(`${gx},${gy}`) || []) if (!under.has(piece) && pieceInBox(piece, box)) under.add(piece);
    return under.size;
  };

  const overlap = (a, b) => a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];

  // Whether a box's middle sits nearer some other mark than its own, which
  // would make the name read as that mark's. The middle, not the edge: a
  // plate spanning two marks is as near both by its edge.
  const NEAR_LOOK = 2; // buckets round a box searched for a nearer mark
  const nearerOther = (grid, [x, y, bw, bh], [ox, oy, or]) => {
    const [mx, my] = [x + bw / 2, y + bh / 2];
    const gap = (cx, cy, r) => Math.hypot(cx - mx, cy - my) - r;
    const own = gap(ox, oy, or);
    for (let gx = Math.floor(x / BUCKET) - NEAR_LOOK; gx <= Math.floor((x + bw) / BUCKET) + NEAR_LOOK; gx++)
      for (let gy = Math.floor(y / BUCKET) - NEAR_LOOK; gy <= Math.floor((y + bh) / BUCKET) + NEAR_LOOK; gy++)
        for (const [cx, cy, r] of grid.get(`${gx},${gy}`) || []) if (Math.hypot(cx - ox, cy - oy) > 1 && gap(cx, cy, r) < own) return true;
    return false;
  };

  // A hidden name leaves layout too, so hundreds waiting for room cost
  // nothing; its box is already kept from when it was measured.
  const hide = (g) => {
    g.setAttribute("visibility", "hidden");
    g.setAttribute("display", "none");
  };

  // Zoomed out past where the machines' column holds their boxes apart,
  // the marks stack, each just under the one above, the stack kept
  // centred on the column; each mark's shift, in screen px.
  const MARK_GAP = 4;
  const stackMarks = () => {
    const shifts = new Map();
    const marks = labels
      .filter((l) => l.mark)
      .map((l) => ({ l, top: l.at[1] * k + ty - l.markBox[3] / 2 - (l.markBox[1] - l.box[1]), x: l.at[0] * k + tx }))
      .sort((a, b) => a.top - b.top);
    let moved = 0;
    const placed = [];
    for (const one of marks) {
      const [, , bw, bh] = one.l.box;
      const left = one.x - bw / 2;
      const under = placed.filter((p) => p.left < left + bw && left < p.left + p.w).reduce((most, p) => Math.max(most, p.bottom + MARK_GAP), -Infinity);
      const top = Math.max(one.top, under);
      shifts.set(one.l.key, top - one.top);
      moved += top - one.top;
      placed.push({ left, w: bw, bottom: top + bh });
    }
    const mean = moved / Math.max(1, marks.length);
    if (moved) for (const [key, s] of shifts) shifts.set(key, s - mean);
    return shifts;
  };

  // Where each label sits on screen, after every pan or zoom.
  const placeLabels = () => {
    const nodes = labelsEl.children;
    const [w, h] = size();
    const taken = [];
    const cuts = [];
    const stacked = stackMarks();
    const circles = circlesOnScreen(w, h);
    const lines = linesOnScreen(w, h);
    // Names shown so far, past the one he's engaged with: a handful at
    // most, busiest first, since a person tracks about seven things at a
    // glance and a screen of names reads as a wall, not a map.
    let shownNames = 0;
    // The names the cap left off, kept on the layer so a check can tell a
    // name lost to the cap from one that found no clear place.
    const capped = [];
    for (let i = 0; i < labels.length; i++) {
      const l = labels[i];
      const g = nodes[i];
      const sx = l.at[0] * k + tx;
      const sy = l.at[1] * k + ty;
      const [bx, by, bw, bh] = l.box;
      // A machine's mark stands on its point, its box centred there, so
      // its cords run to the box's edge; marks come first, so every name
      // keeps clear of them.
      if (l.mark) {
        const [nx, ny, nw, nh] = l.markBox;
        const [dx, dy] = [sx - nx - nw / 2, sy - ny - nh / 2 + (stacked.get(l.key) || 0)];
        const placed = [bx + dx, by + dy, bw, bh];
        if (placed[0] > w || placed[1] > h || placed[0] + bw < 0 || placed[1] + bh < 0) {
          hide(g);
          continue;
        }
        g.removeAttribute("visibility");
        g.removeAttribute("display");
        taken.push(placed);
        // Its cords stop just short of the box's hairline, as they do round
        // a name, and the counts under it are cleared too, so no cord runs
        // through them.
        cuts.push([nx + dx, ny + dy, nw, nh]);
        if (l.subBox) cuts.push([l.subBox[0] + dx, l.subBox[1] + dy, l.subBox[2], l.subBox[3]]);
        g.setAttribute("transform", `translate(${dx.toFixed(1)},${dy.toFixed(1)})`);
        continue;
      }
      // A cell that needs him is named once it is drawn rather than
      // folded into its network's ground: from there his eye can find it,
      // and the cap keeps the screen to seven names whatever the zoom.
      if (l.small && l.r * k < FOLD_PX) {
        hide(g);
        continue;
      }
      if (l.rank > 0 && shownNames >= NAMES_ON_SCREEN) {
        capped.push(l.key);
        hide(g);
        continue;
      }
      // A hub's name sits past its cells while they're small, and beside
      // the hub once they're large enough to read on their own.
      const gap = (l.kind === "hub" ? Math.min(l.disc * k, Math.max(l.r * k, 110)) : l.r * k) + 6;
      const order = [...COMPASS].sort((p, q) => q[0] * l.out[0] + q[1] * l.out[1] - (p[0] * l.out[0] + p[1] * l.out[1]));
      // Each side at the near distance first, then further out, so a name
      // walks clear of a crowd before it ever sits on one.
      // No further: a name walked far from its mark reads as naming its
      // neighbour, which is worse than a cord passing under its plate.
      const reaches = l.kind === "hub" ? [gap, Math.max(gap, l.disc * k + 10), l.disc * k + 40] : [gap, gap + 14, gap + 32];
      const tries = reaches.flatMap((g2) => order.map(([ux, uy]) => [sx + ux * g2 - bw / 2 + (ux * bw) / 2, sy + uy * g2 - bh / 2 + (uy * bh) / 2]));
      const boxOf = ([x, y]) => [x, y, bw, bh];
      const clearOfNames = (t) => !taken.some((o) => overlap(boxOf(t), o));
      const clearOfMarks = (t) => !coversCircle(circles, boxOf(t));
      const clearOfLines = (t) => linesUnder(lines, boxOf(t)) === 0;
      const seen = ([x, y]) => x >= 0 && y >= 0 && x + bw <= w && y + bh <= h;
      // Of the places allowed, the one over fewest lines, the nearest
      // where they tie: so a line runs under a name's plate only where
      // every place near enough has one.
      const least = (places) => {
        let best = null;
        let fewest = Infinity;
        for (const t of places) {
          const n = linesUnder(lines, boxOf(t));
          if (n < fewest) [best, fewest] = [t, n];
          if (!n) break;
        }
        return best;
      };
      // Never over a mark or another name, and nearer its own mark than any
      // other where it can be.
      const own = [sx, sy, l.r * k + 4];
      // A name never lies over a mark or a line: one with no clear place
      // near its own mark waits for a hover or a closer view.
      const allowed = tries.filter((t) => seen(t) && clearOfNames(t) && clearOfMarks(t) && (l.rank === 0 || clearOfLines(t)));
      let spot = least(allowed.filter((t) => !nearerOther(circles, boxOf(t), own))) || least(allowed);
      // What he's engaged with (pointed at, open) keeps its name, pulled on
      // screen rather than left half off the edge, and over a mark only
      // where nowhere else will do. Any other name shows whole or not at
      // all: half a name at the map's edge reads as cut, not as more
      // beyond. A mark off the map has nothing on screen to name.
      const markSeen = sx >= 0 && sy >= 0 && sx <= w && sy <= h;
      const inView = ([x, y]) => [Math.max(0, Math.min(w - bw, x)), Math.max(0, Math.min(h - bh, y))];
      if (!spot && l.rank === 0 && markSeen) {
        const pulled = tries.map(inView);
        spot = least(pulled.filter((t) => clearOfNames(t) && clearOfMarks(t))) || least(pulled.filter(clearOfNames)) || inView(tries[0]);
      }
      if (!spot) {
        hide(g);
        continue;
      }
      g.removeAttribute("visibility");
      g.removeAttribute("display");
      if (l.rank > 0) shownNames++;
      taken.push(boxOf(spot));
      cuts.push(boxOf(spot));
      g.setAttribute("transform", `translate(${(spot[0] - bx).toFixed(1)},${(spot[1] - by).toFixed(1)})`);
    }
    labelsEl.dataset.capped = capped.join(" ");
    // Only a plate over a line is knocked out, since elsewhere a knock-out
    // cuts nothing, and the ground paints again only when those plates
    // change: at thousands of cells a paint for nothing costs every press.
    // The pieces asked about are chords of curves, so the reach is wider
    // than the knock-out by what a curve may bow off its chord.
    const reach = ([x, y, bw, bh]) => [x - KNOCK_REACH_PX, y - KNOCK_REACH_PX, bw + 2 * KNOCK_REACH_PX, bh + 2 * KNOCK_REACH_PX];
    const cutting = cuts.filter((b) => linesUnder(lines, reach(b)) > 0);
    const sameBox = (a, b) => a.every((v, j) => Math.abs(v - b[j]) < KNOCK_SAME_PX);
    const same = cutting.length === knocks.length && cutting.every((b) => knocks.some((o) => sameBox(b, o)));
    if (same) return;
    knocks = cutting;
    knocksMoved();
  };
  const KNOCK_SAME_PX = 0.5;
  const KNOCK_REACH_PX = 8;

  // ---- The pointer: where the keys have walked to. ----

  let pointed = null; // a node's key: m:machine, h:hub or c:address
  let spots = new Map(); // each node's place
  let sizes = new Map(); // each node's radius
  let treeAdj = new Map(); // cords and hyphae: what hangs from what
  let crossAdj = new Map(); // cross-hyphae: what is shared
  const link = (adj, a, b) => {
    for (const [p, q] of [
      [a, b],
      [b, a],
    ]) {
      if (!adj.has(p)) adj.set(p, new Set());
      adj.get(p).add(q);
    }
  };
  const KIND_OF = { c: "cell", h: "net", n: "net", m: "machine" };
  const choiceOf = (key) => ({ kind: KIND_OF[key[0]], id: key.slice(2) });
  const keyOf = (c) => {
    if (!c) return null;
    if (c.kind === "cell") return `c:${c.id}`;
    if (c.kind === "machine") return `m:${c.id}`;
    return current?.hubs.has(c.id) ? `h:${c.id}` : null;
  };
  const firstKey = () => (shownMachines()[0] ? `m:${shownMachines()[0].name}` : spots.keys().next().value);
  const pointing = () => view.contains(document.activeElement) || tree.contains(document.activeElement);

  // Where the keys rest, and the level they walk, said on the marks too.
  const markHere = () => {
    for (const el of world.querySelectorAll("[data-here]")) el.removeAttribute("data-here");
    if (pointed) world.querySelector(`[data-key="${CSS.escape(pointed)}"]`)?.setAttribute("data-here", "");
    const { hub } = levelOf(pointed);
    view.dataset.level = hub ? hub.key : pointed?.[0] === "c" ? LOOSE_LEVEL : TOP_LEVEL;
    view.dataset.within = hub?.key || "";
  };
  const placePointer = () => {
    const ring = document.getElementById("pointer");
    if (!ring) return;
    const p = pointed && spots.get(pointed);
    markHere();
    markFound();
    drawTrail();
    // On a machine the keys ring its box, which is its mark, not its point.
    const onMark = pointed?.[0] === "m";
    for (const g of labelsEl.querySelectorAll(".t-mark")) g.classList.toggle("here", onMark && pointing() && g.dataset.mark === pointed);
    ring.setAttribute("visibility", p && pointing() && !onMark ? "visible" : "hidden");
    if (!p) return;
    ring.setAttribute("transform", `translate(${p[0].toFixed(1)},${p[1].toFixed(1)})`);
    for (const c of ring.children) c.setAttribute("r", String((sizes.get(pointed) || R) + 6));
  };

  // A walk past the edge of the view brings the map along.
  const keepInView = (key) => {
    const p = spots.get(key);
    if (!p) return;
    const [w, h] = seen();
    const sx = p[0] * k + tx;
    const sy = p[1] * k + ty;
    const EDGE = 90;
    if (sx >= EDGE && sx <= w - EDGE && sy >= EDGE && sy <= h - EDGE) return;
    auto = false;
    goTo([k, w / 2 - k * p[0], h / 2 - k * p[1]]);
  };

  const countsWords = (cells) => {
    const n = countsOf(cells);
    const bits = STANDINGS.filter((s) => n[s]).map((s) => `${n[s]} ${STANDING_WORD[s]}`);
    const held = cells.filter((s) => s.owes.length).length;
    return `${plural(cells.length, "cell")}${bits.length ? `: ${bits.join(", ")}` : ""}${held ? `; ${held} with ${held === 1 ? "a question" : "questions"} waiting` : ""}`;
  };
  const cellWords = (s) =>
    (s.standing === "asks" ? (s.wants ? `needs you: ${s.wants}` : STANDING_WORD.asks) : s.standing === "moving" ? `moving: ${s.doing || "in a turn"}` : STANDING_WORD[s.standing]) +
    (s.owes.length ? `; ${waitingWords(s.owes)}` : "") +
    (s.holds.length ? `; ${s.holds.length === 1 ? "a hold" : `${s.holds.length} holds`} standing` : "");

  // What a node is, in words: the name a tree item goes by.
  const describe = perReading((key) => {
    const m = current;
    const id = key.slice(2);
    if (key[0] === "c") {
      const s = m.byAddress.get(id);
      return s ? `${s.short}, ${cellWords(s)}` : id;
    }
    if (key[0] === "h") {
      const h = m.hubs.get(id);
      return h ? `${h.name}, ${hubWords(h)}, a network${h.children.length ? ` with ${plural(h.children.length, "branch", "branches")}` : ""}` : id;
    }
    const x = current.machines.find((y) => y.name === id);
    return `${id}, ${machineWords(id)}, ${x?.members ? `a basin of ${plural(x.members.length, "machine")}` : "a machine"}`;
  });
  // The words a tree row shows under a name are the words its name says
  // after it, so what's read aloud holds what's seen.
  const machineWords = (id) => {
    const x = current.machines.find((y) => y.name === id);
    const cells = cellsOn(id);
    const state = x?.state !== "ready" ? "off" : offWords(x) ? `ready, ${offWords(x)}` : "ready";
    return `${state}, ${plural(cells.length, "cell")}${spentWords(cells)}`;
  };
  // A machine's cells, one list per reading, so its sum is kept with it.
  const cellsOn = perReading((id) => current.cells.filter((s) => s.machine === id));
  // A network with its branches, every cell under it, one list per hub.
  const ALL_CELLS = new WeakMap();
  const allCells = (h) => {
    if (!ALL_CELLS.has(h)) ALL_CELLS.set(h, h.children.length ? [...h.cells, ...h.children.flatMap(allCells)] : h.cells);
    return ALL_CELLS.get(h);
  };
  const hubWords = (h) =>
    `${countsWords(h.cells)}${h.children.length && spends(allCells(h)) ? `, ${money(spentOf(allCells(h)))} today with its branches` : spentWords(h.cells)}`;
  // What a node is and where it hangs, said as the keys land on it.
  const landing = (key) => {
    const m = current;
    const id = key.slice(2);
    let on = "";
    if (key[0] === "c") on = m.byAddress.get(id)?.hub ? `, on ${m.byAddress.get(id).hub.name}` : "";
    if (key[0] === "h" && m.hubs.get(id)?.within) on = `, a branch of ${m.hubs.get(m.hubs.get(id).within).name}`;
    return `${describe(key)}${on}.`;
  };

  const point = (key, { say = false, follow = true } = {}) => {
    if (!key || !spots.has(key)) return;
    pointed = key;
    treeActive = key;
    const opened = expanded.size;
    openTo(key);
    placePointer();
    labelsFor();
    placeLabels();
    if (follow) keepInView(key);
    // The tree is drawn again only when a branch had to open to show the
    // key; otherwise its one tab stop moves to the row already there.
    const row = expanded.size === opened && tree.querySelector(`[data-key="${CSS.escape(key)}"]`);
    if (row) {
      for (const x of tree.querySelectorAll('[tabindex="0"]')) x.tabIndex = -1;
      row.tabIndex = 0;
    } else renderTree();
    // The tree scrolls with the whole panel now, so its row is brought into
    // view only while the keys are in the tree, never from the map.
    if (tree.contains(document.activeElement)) tree.querySelector(`[data-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: "nearest" });
    if (say) {
      where.textContent = `${landing(key)} ${levelWords(key)}`;
      showPeek(key[0] === "c" ? null : choiceOf(key));
    }
  };

  // The arrows walk one level at a time, so where they go is never a
  // surprise: the whole map's level holds the machines and the networks,
  // and a network's level holds only its cells. Return or zooming in goes
  // down a level, "-" or zooming out comes back up.
  const levelOf = (key) => {
    if (!key || key[0] !== "c") return { hub: null, keys: topKeys() };
    const s = current.byAddress.get(key.slice(2));
    const hub = netOf(s);
    const keys = (hub ? hub.cells : current.cells.filter((x) => !netOf(x))).map((x) => `c:${x.address}`).filter((k2) => spots.has(k2));
    return { hub, keys };
  };
  const topKeys = () => [...spots.keys()].filter((key) => key[0] !== "c");
  const levelWords = (key) => {
    const { hub } = levelOf(key);
    if (key?.[0] !== "c") return `Arrows walk the ${machinesShown ? "machines and networks" : "networks"}; Return goes in.`;
    return `Arrows walk ${hub?.edge ? "the receptors" : hub ? `the cells of ${hub.name}` : "the cells working alone"}; - comes back out.`;
  };
  // Into a network: the keys land on its cell nearest `around`, by
  // default the middle of the view, which is where the eye already is.
  const into = (hubKey, around) => {
    const hub = current.hubs.get(hubKey.slice(2));
    if (!hub) return null;
    const [w, h] = seen();
    const [cx, cy] = around || [(w / 2 - tx) / k, (h / 2 - ty) / k];
    let pick = null;
    let best = Infinity;
    for (const s of hub.cells) {
      const q = spots.get(`c:${s.address}`);
      if (!q) continue;
      const d = Math.hypot(q[0] - cx, q[1] - cy);
      if (d < best) {
        best = d;
        pick = `c:${s.address}`;
      }
    }
    return pick;
  };

  // A step within the level: to the nearest thing that way, measured
  // along the arrow, among what lies within 45 degrees of it. Nothing
  // there, and the keys stay put and say so; nothing outside the level
  // is a stop. The cone alone can't promise the opposite arrow comes
  // back, so the last step is remembered and the opposite arrow retraces
  // it.
  const WAY = { "1,0": "to the right", "-1,0": "to the left", "0,-1": "up", "0,1": "down" };
  let lastStep = null;
  let lastNothing = "";
  const walk = (dx, dy) => {
    if (!pointed || !spots.has(pointed)) {
      point(firstKey(), { say: true });
      return;
    }
    if (lastStep && lastStep.to === pointed && lastStep.dx === -dx && lastStep.dy === -dy && spots.has(lastStep.from) && levelOf(pointed).keys.includes(lastStep.from)) {
      const from = lastStep.from;
      lastStep = { from: pointed, to: from, dx, dy };
      lastNothing = "";
      point(from, { say: true });
      return;
    }
    const [px, py] = spots.get(pointed);
    let next = null;
    let nearest = Infinity;
    let square = Infinity;
    for (const key of levelOf(pointed).keys) {
      const q = spots.get(key);
      if (key === pointed || !q) continue;
      const vx = q[0] - px;
      const vy = q[1] - py;
      const along = vx * dx + vy * dy;
      const off = Math.abs(vx * dy - vy * dx);
      if (along <= 0 || off > along) continue;
      if (along < nearest || (along === nearest && off < square)) {
        nearest = along;
        square = off;
        next = key;
      }
    }
    if (next) {
      lastStep = { from: pointed, to: next, dx, dy };
      lastNothing = "";
      point(next, { say: true });
      return;
    }
    // A live region says nothing for the same words twice, so a second
    // press the same way is said differently.
    const way = WAY[`${dx},${dy}`];
    const again = lastNothing === `${pointed} ${way}`;
    lastNothing = `${pointed} ${way}`;
    where.textContent = `${again ? "Still nothing" : "Nothing"} ${way} on this level; the keys stay on ${landing(pointed)}`;
  };

  // ---- Pan and zoom. ----

  let k = 1;
  let tx = 0;
  let ty = 0;
  let auto = true; // the view follows the whole map until he moves it
  let glide = 0;

  const frame = () => {
    world.setAttribute("transform", `translate(${tx.toFixed(2)},${ty.toFixed(2)}) scale(${k.toFixed(4)})`);
    placeLabels();
    paintGround();
    refold();
  };

  // ---- Folding: past FOLD_OVER cells, a network too small to read, or
  // off the map, draws as its ground and hub, and its cells unfold as he
  // comes close. What's open, chosen, pointed at or found never folds. ----
  const FOLD_OVER = 400; // cells, at or under which nothing folds
  const CORD_BUNDLE_OVER = 24; // networks on a machine, past which its cords bundle
  const CORD_MOST = 6; // px, the heaviest a cord or trunk is drawn
  const CROSS_MOST = 4; // px, the largest a cross-hypha's dots are drawn
  const BUNDLE_WAYS = 12; // trunks out of a bundling machine, one each way
  const BUNDLE_REACH = 0.8; // how far out a trunk forks, as a share of the ring
  const MACHINE_ORBIT = 0.3; // how far out a bundling machine sits, as a share of the ring
  const FOLD_PX = 4; // a cell's radius on screen under which its network folds: 8 px across
  let folded = new Set();
  let refolding = 0;
  const homeOf = (s) => (s ? (s.hub || s.loose).key : null);
  const isFolded = (s) => folded.has(homeOf(s));
  const foldsNow = () => {
    const m = current;
    const out = new Set();
    // Cells drawn too small to read as their kind fold at any count; past
    // FOLD_OVER, networks off the map fold as well.
    const small = R * k < FOLD_PX;
    const many = m && m.cells.length > FOLD_OVER;
    if (!m || !L || !(small || many)) return out;
    const keep = new Set();
    if (chosen?.kind === "net") keep.add(chosen.id);
    if (chosen?.kind === "cell") keep.add(homeOf(m.byAddress.get(chosen.id)));
    // A talk open keeps both its ends' networks open, so its line has cells to join.
    if (chosen?.kind === "talk") for (const end of chosen.id.split("|")) if (end.startsWith("c:")) keep.add(homeOf(m.byAddress.get(end.slice(2))));
    if (pointed?.startsWith("c:")) keep.add(homeOf(m.byAddress.get(pointed.slice(2))));
    for (const key of foundSet) if (key[0] === "c") keep.add(homeOf(m.byAddress.get(key.slice(2))));
    const [w, h] = size();
    for (const hub of m.hubs.values()) {
      if (keep.has(hub.key)) continue;
      const [x, y] = L.at.get(hub.key);
      const r = (colonyRadius(hub) + COLONY_PAD) * k;
      const [sx, sy] = [x * k + tx, y * k + ty];
      if (small || sx + r < 0 || sx - r > w || sy + r < 0 || sy - r > h) out.add(hub.key);
    }
    return out;
  };
  // A view that folds differently from what's drawn redraws once, on the
  // next frame; the redraw folds by the same view, so it settles there.
  const refold = () => {
    if (refolding || !current) return;
    const now = foldsNow();
    if (now.size === folded.size && [...now].every((key) => folded.has(key))) return;
    refolding = requestAnimationFrame(() => {
      refolding = 0;
      draw();
    });
  };

  // The map's size is kept until it changes: measuring it straight after a
  // frame's write would lay out every mark again on every frame.
  let mapSize = null;
  let gutter = 0;
  new ResizeObserver(() => {
    mapSize = null;
  }).observe(svg);
  // Wide enough, the drawer lies over the map's right side, so the map
  // keeps that side as a gutter: nothing is centred or fitted under it,
  // and opening the drawer moves nothing. Narrower, the drawer covers
  // the whole map and there is no gutter.
  const wide = matchMedia("(min-width: 60rem) and (min-height: 34rem)");
  wide.addEventListener("change", () => {
    mapSize = null;
  });
  const size = () => {
    if (!mapSize) {
      const b = svg.getBoundingClientRect();
      mapSize = [b.width, b.height];
      // offsetWidth, since the drawer's slide is a transform.
      gutter = wide.matches ? treeBox.offsetWidth : 0;
    }
    return mapSize;
  };
  // The part of the map the drawer never covers, which views are made for.
  const seen = () => {
    const [w, h] = size();
    return [Math.max(w - gutter, w / 2), h];
  };

  const boundsOf = (points, pad) => {
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const [x, y] of points) {
      x0 = Math.min(x0, x - pad);
      y0 = Math.min(y0, y - pad);
      x1 = Math.max(x1, x + pad);
      y1 = Math.max(y1, y + pad);
    }
    return [x0, y0, x1, y1];
  };

  // The view that shows `box` whole, with room for the names past it.
  const viewOf = ([x0, y0, x1, y1], most = 1.4, room = LABEL_ROOM) => {
    const [w, h] = seen();
    // On a narrow view the names sit on the map, clear of its lines,
    // rather than taking most of the width as a margin.
    const side = Math.min(room, w * ROOM_SHARE);
    const kk = Math.max(kFloor(), Math.min(most, (w - 2 * (FIT_PAD + side)) / (x1 - x0), (h - 2 * (FIT_PAD + 44)) / (y1 - y0)));
    return [kk, w / 2 - kk * ((x0 + x1) / 2), h / 2 - kk * ((y0 + y1) / 2)];
  };

  const steps = (animate) => !animate || still.matches || (current?.cells.length || 0) > GLIDE_UNDER;
  // Set while a redraw follows at once, which frames the view itself.
  let drawNext = false;
  const goTo = ([k2, tx2, ty2], animate = true) => {
    cancelAnimationFrame(glide);
    // Past GLIDE_UNDER cells each frame of a glide repaints more than a
    // frame allows, so the view steps once rather than stuttering.
    if (steps(animate)) {
      [k, tx, ty] = [k2, tx2, ty2];
      if (!drawNext) frame();
      return;
    }
    const [k1, tx1, ty1] = [k, tx, ty];
    const t0 = performance.now();
    const step = (t) => {
      // A frame's time can be up to a frame before t0, so p starts at 0.
      const p = Math.min(1, Math.max(0, (t - t0) / GLIDE_MS));
      const e = 1 - (1 - p) ** 3;
      k = k1 + (k2 - k1) * e;
      tx = tx1 + (tx2 - tx1) * e;
      ty = ty1 + (ty2 - ty1) * e;
      frame();
      if (p < 1) glide = requestAnimationFrame(step);
    };
    glide = requestAnimationFrame(step);
  };

  const wholeBox = () => {
    const points = shownMachines().map((x) => L.machines.get(x.name));
    for (const h of current.hubs.values()) {
      const [x, y] = L.at.get(h.key);
      const r = L.disc.get(h.key);
      points.push([x - r, y - r], [x + r, y + r]);
    }
    return boundsOf(points, 0);
  };

  const fit = (animate = true) => {
    auto = true;
    if (!current) return;
    goTo(viewOf(wholeBox()), animate);
  };

  // The farthest out the view goes: K_MIN, or further when the whole map
  // needs it, so a big network can always be seen whole.
  const kFloor = () => {
    if (!current || !L) return K_MIN;
    const [w, h] = seen();
    const [x0, y0, x1, y1] = wholeBox();
    const whole = Math.min((w - 2 * FIT_PAD) / (x1 - x0), (h - 2 * (FIT_PAD + 44)) / (y1 - y0));
    return Math.max(K_LEAST, Math.min(K_MIN, whole * WHOLE_SLACK));
  };

  const zoomAt = (factor, sx, sy) => {
    auto = false;
    const k2 = Math.max(kFloor(), Math.min(K_MAX, k * factor));
    tx = sx - ((sx - tx) * k2) / k;
    ty = sy - ((sy - ty) * k2) / k;
    k = k2;
    frame();
    levelByZoom();
  };

  const zoomBy = (factor) => {
    const [w, h] = seen();
    zoomAt(factor, w / 2, h / 2);
  };

  // Zooming changes the level the keys are on: a network drawn large
  // enough to fill much of the view is walked by its cells, and one drawn
  // small again by the networks. The gap between the two keeps a zoom
  // from flickering between levels.
  const LEVEL_IN = 0.38; // a network's disc over the view's short side
  const LEVEL_OUT = 0.12;
  const levelByZoom = () => {
    if (!pointed || !pointing() || !L) return;
    const [w, h] = seen();
    const span = Math.min(w, h);
    if (pointed[0] === "h") {
      const disc = L.disc.get(pointed.slice(2));
      const cell = disc && disc * k > span * LEVEL_IN ? into(pointed) : null;
      if (cell) point(cell, { say: true, follow: false });
    } else if (pointed[0] === "c") {
      const { hub } = levelOf(pointed);
      const disc = hub && L.disc.get(hub.key);
      if (disc && disc * k < span * LEVEL_OUT) point(`h:${hub.key}`, { say: true, follow: false });
    }
  };

  // ---- A peek: what a network or machine is for and how it stands, in
  // three short lines, without opening it. The pointer's peek sits by the
  // pointer; the keys' peek sits in the map's lower corner, clear of the
  // mark and its name. It says nothing the drawer doesn't, so a reader
  // skips it and hears the walk's own words instead. ----
  const peekEl = document.getElementById("peek");
  const PEEK_GAP = 18; // px between the pointer and the peek
  const PEEK_PURPOSE = 90; // characters of purpose a peek carries
  let peeking = null; // the key or line it is about
  const peekOf = (c) => {
    const m = current;
    if (!m || !c) return null;
    if (c.kind === "machine") {
      const cells = cellsOn(c.id);
      const { v, words } = groupHealth(cells);
      return [c.id, `A machine running ${plural(cells.length, "cell")}.`, v, words];
    }
    if (c.kind !== "net") return null;
    const h = m.hubs.get(c.id);
    if (h) {
      const all = allCells(h);
      const { v, words } = groupHealth(all);
      // A peek says what the drawer would say first: what the network is
      // for when its cells say, else what it is made of.
      const said = networkPurpose(h);
      const branches = h.children.length ? `, ${plural(h.children.length, "branch", "branches")} among them` : "";
      return [h.name, said === NO_PURPOSE ? `A network of ${plural(all.length, "cell")}${branches}.` : said, v, words];
    }
    const n = m.shared.find((x) => x.key === c.id);
    if (!n) return null;
    const { v, words } = groupHealth(n.cells);
    return [n.short, n.title || `A network sharing the ${n.short} ${SHARED_WORD[n.shape]}.`, v, words];
  };
  const hidePeek = () => {
    if (!peeking) return;
    peeking = null;
    peekEl.hidden = true;
  };
  // The thing already open in the drawer needs no peek.
  const isChosen = (c) => chosen && chosen.kind === c.kind && chosen.id === c.id;
  const showPeek = (c, at) => {
    const id = c ? `${c.kind}:${c.id}` : null;
    const words = c && !isChosen(c) ? peekOf(c) : null;
    if (!words) return hidePeek();
    if (id !== peeking) {
      peeking = id;
      const [name, purpose, v, health] = words;
      peekEl.innerHTML = `<p class="peek-name">${esc(clipped(name))}</p><p>${esc(clipped(purpose, PEEK_PURPOSE))}</p><p class="mono">${verdictMark(v)}: ${esc(health)}</p>`;
      peekEl.hidden = false;
    }
    const [w, h] = seen();
    const pw = peekEl.offsetWidth;
    const ph = peekEl.offsetHeight;
    const [x, y] = at || [PEEK_GAP, h - ph - PEEK_GAP];
    const left = at && x + PEEK_GAP + pw > w ? x - PEEK_GAP - pw : at ? x + PEEK_GAP : x;
    const top = at ? Math.min(Math.max(y + PEEK_GAP, 0), Math.max(h - ph, 0)) : Math.max(y, 0);
    peekEl.style.transform = `translate(${Math.max(left, 0).toFixed(0)}px, ${top.toFixed(0)}px)`;
  };
  // The key of the mark under the pointer: a disc on the map, or a
  // machine's boxed name among the words.
  const keyUnder = (el) => el?.closest?.("[data-key]")?.dataset.key || el?.closest?.("[data-mark]")?.dataset.mark || null;
  // What the pointer is over, as a peek would name it: a hub or machine,
  // a line that opens a network, or a network's ground.
  const peekUnder = (e, line) => {
    const node = keyUnder(e.target);
    if (node) return node[0] === "c" ? null : choiceOf(node);
    if (line) return line.opens.kind === "net" || line.opens.kind === "machine" ? line.opens : null;
    const box = svg.getBoundingClientRect();
    const ground = groundAt((e.clientX - box.left - tx) / k, (e.clientY - box.top - ty) / k);
    return ground ? { kind: "net", id: ground } : null;
  };

  // The line under the pointer, if any.
  const lineUnder = (e) => {
    const box = svg.getBoundingClientRect();
    const [x, y] = [e.clientX - box.left, e.clientY - box.top];
    return lineAt(x, y, !!groundAt((x - tx) / k, (y - ty) / k));
  };
  // Dragging moves the map; a press that barely moves is a click.
  const pointers = new Map();
  let dragged = false;
  let pinch = null;
  svg.addEventListener("pointerdown", (e) => {
    hidePeek();
    if (e.button !== 0 && e.pointerType === "mouse") return;
    svg.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, [e.clientX, e.clientY]);
    dragged = false;
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]) };
    }
  });
  svg.addEventListener("pointermove", (e) => {
    const last = pointers.get(e.pointerId);
    if (!last) {
      const g = e.target.closest?.(".cell");
      const id = g ? current?.byAddress.get(g.dataset.key.slice(2))?.id : null;
      const line = keyUnder(e.target) ? null : lineUnder(e);
      svg.classList.toggle("on-line", !!line);
      const box = view.getBoundingClientRect();
      showPeek(peekUnder(e, line), [e.clientX - box.left, e.clientY - box.top]);
      if (id !== hover || line !== touched) {
        const named = id !== hover;
        hover = id;
        touched = line;
        if (named) {
          labelsFor();
          placeLabels();
        }
        paintGround();
      }
      return;
    }
    const dx = e.clientX - last[0];
    const dy = e.clientY - last[1];
    pointers.set(e.pointerId, [e.clientX, e.clientY]);
    if (pointers.size === 2 && pinch) {
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
      const box = svg.getBoundingClientRect();
      zoomAt(d / pinch.d, (a[0] + b[0]) / 2 - box.left, (a[1] + b[1]) / 2 - box.top);
      pinch.d = d;
      dragged = true;
      return;
    }
    if (!dragged && Math.hypot(dx, dy) < 1 && Math.abs(dx) + Math.abs(dy) < 4) return;
    dragged = true;
    auto = false;
    tx += dx;
    ty += dy;
    frame();
  });
  const release = (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.delete(e.pointerId);
    if (pointers.size < 2) pinch = null;
    if (e.type === "pointerup" && !dragged && pointers.size === 0) {
      const hit = document.elementFromPoint(e.clientX, e.clientY);
      const node = keyUnder(hit);
      // A network's ground opens the network; empty map lets go.
      const box = svg.getBoundingClientRect();
      const ground = node ? null : groundAt((e.clientX - box.left - tx) / k, (e.clientY - box.top - ty) / k);
      // A line opens what it carries, before the ground it may cross.
      const line = node ? null : lineUnder(e);
      const key = node || (!line && ground && `h:${ground}`);
      if (key) {
        pointed = key;
        visit(choiceOf(key));
      } else if (line) {
        // The keys go where the click went, and the map says what opened,
        // so a screen reader hears it too: a talk's keys go to its first end.
        const c = line.opens;
        const to = keyOf(c) || (c.kind === "talk" ? c.id.split("|")[0] : null);
        if (to) pointed = to;
        talkFrom = null;
        visit(c);
        where.textContent = `Opened ${openedWords(c)}; its details are in the drawer.`;
      } else letGo();
    }
  };
  svg.addEventListener("pointerup", release);
  svg.addEventListener("pointercancel", release);
  svg.addEventListener("pointerleave", () => {
    hidePeek();
    if (touched) {
      touched = null;
      svg.classList.remove("on-line");
      paintGround();
    }
    if (hover && !pointers.size) {
      hover = null;
      labelsFor();
      placeLabels();
    }
  });
  svg.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const box = svg.getBoundingClientRect();
      const amount = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
      zoomAt(Math.exp(-amount * 0.0018), e.clientX - box.left, e.clientY - box.top);
    },
    { passive: false },
  );
  // On the map, arrows and h, j, k, l walk the network; with shift they
  // move the map instead; Return opens what the walk has reached.
  const PAN = 80;
  view.addEventListener("keydown", (e) => {
    const asked = current ? mapKey(e) : null;
    if (!asked) return;
    e.preventDefault();
    if (asked.act === "pan") {
      auto = false;
      tx -= asked.way[0] * PAN;
      ty -= asked.way[1] * PAN;
      frame();
    } else if (asked.act === "walk") walk(asked.way[0], asked.way[1]);
    else {
      if (!pointed) point(firstKey(), { say: true });
      else {
        const key = pointed;
        visit(choiceOf(key));
        // Opening a network goes down to its level, at its middle cell.
        const cell = key[0] === "h" ? into(key, spots.get(key)) : null;
        if (cell) point(cell, { follow: false });
        where.textContent = `Opened ${describe(key)}; its details are in the drawer. ${levelWords(pointed)}`;
      }
    }
  });
  // The first time the map takes focus, the walk starts at what's chosen,
  // or at the first machine, the root of everything.
  view.addEventListener("focus", () => {
    if (current && !pointed) point(keyOf(chosen) || firstKey(), { say: true, follow: false });
  });
  const showPointing = () =>
    requestAnimationFrame(() => {
      svg.classList.toggle("pointing", pointing());
      placePointer();
    });
  for (const el of [view, tree]) {
    el.addEventListener("focusin", showPointing);
    el.addEventListener("focusout", showPointing);
  }
  document.querySelector(".zoom").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-zoom]");
    if (!b) return;
    if (b.dataset.zoom === "in") zoomBy(1.25);
    else if (b.dataset.zoom === "out") zoomBy(0.8);
    else if (b.dataset.zoom === "now") toggleNow();
    else fit();
  });

  // ---- Choosing: a cell or a network, said in the URL. ----

  let chosen = null; // {kind: "cell" | "net" | "machine" | "talk", id}

  // A talk stays open after it ends, so long as both ends are still here:
  // its details then say nothing passed, rather than the view jumping out.
  const talkEndsKnown = (id) => {
    const ends = id.split("|");
    return ends.length === 2 && ends.every((end) => (end[0] === "h" ? current.hubs.has(end.slice(2)) : end[0] === "c" && current.byAddress.has(end.slice(2))));
  };
  const knows = (c) =>
    !!c &&
    !!current &&
    (c.kind === "cell"
      ? current.byAddress.has(c.id)
      : c.kind === "machine"
        ? current.machines.some((x) => x.name === c.id)
        : c.kind === "talk"
          ? talkEndsKnown(c.id)
          : current.hubs.has(c.id) || current.shared.some((n) => n.key === c.id));

  // What a chosen thing touches: for a cell, its hub and the path of hyphae
  // to it, every cell it shares a network with, and the hubs it is on; for
  // a network, its cells and its branches.
  // The cells, and their networks, that any of these cells has talked
  // with in the last ten minutes.
  const talkedWith = (cells, ids, hubs) => {
    const own = new Set(cells.map((s) => `c:${s.address}`));
    for (const e of talk.keys()) {
      const [a, b] = e.split("|");
      const other = own.has(a) ? b : own.has(b) ? a : null;
      if (!other) continue;
      if (other[0] === "h") hubs.add(other.slice(2));
      const s = other[0] === "c" && current.byAddress.get(other.slice(2));
      if (!s) continue;
      ids.add(s.id);
      if (s.hub) hubs.add(s.hub.key);
    }
  };
  const nearOf = (c) => {
    const m = current;
    const ids = new Set();
    const hubs = new Set();
    const hyphae = new Set();
    const shared = new Set();
    let root = false;
    const machines = new Set();
    if (c.kind === "talk") {
      for (const end of c.id.split("|")) {
        if (end[0] === "h") hubs.add(end.slice(2));
        else if (m.byAddress.has(end.slice(2))) ids.add(m.byAddress.get(end.slice(2)).id);
      }
    } else if (c.kind === "machine") {
      machines.add(c.id);
      root = true;
      // Its own networks, and those it feeds from beside another machine.
      for (const h of m.hubs.values()) if (!h.within && (h.machine === c.id || h.feeds.some(([x]) => x === c.id))) hubs.add(h.key);
      for (const s of m.cells) if (s.machine === c.id) ids.add(s.id);
    } else if (c.kind === "cell") {
      const s = m.byAddress.get(c.id);
      if (!s) return null;
      ids.add(s.id);
      hyphae.add(s.id);
      const home = s.hub || s.loose;
      if (home) hubs.add(home.key);
      for (const h of s.also) hubs.add(h.key);
      for (const n of m.shared) {
        if (!n.cells.includes(s)) continue;
        shared.add(n.key);
        for (const o of n.cells) ids.add(o.id);
      }
      // The path of hyphae from the hub out to it: the tree's edges whose
      // far end is on the way.
      if (home && L) {
        const list = (seeds.get(home.key) || []).filter((id) => id && pos.has(id));
        const points = [L.at.get(home.key), ...list.map((id) => pos.get(id))];
        const parent = new Map();
        for (const [a, b] of treeOf(`h|${home.key}|${list.length}`, points)) parent.set(b, a);
        let i = list.indexOf(s.id) + 1;
        while (i > 0) {
          hyphae.add(list[i - 1]);
          i = parent.get(i) ?? 0;
        }
      }
    } else {
      const h = m.hubs.get(c.id);
      if (h) {
        hubs.add(h.key);
        if (!h.within) {
          root = true;
          if (h.machine) machines.add(h.machine);
          for (const [x] of h.feeds) machines.add(x);
        }
        if (h.within) hubs.add(h.within);
        for (const s of h.cells) {
          ids.add(s.id);
          hyphae.add(s.id);
        }
        // What joins it directly stands out with it: its branches' cells,
        // cells of other networks that are members of it too, and the
        // cells its own have talked with lately, with their networks.
        for (const ch of h.children) {
          hubs.add(ch.key);
          for (const s of ch.cells) ids.add(s.id);
        }
        for (const s of m.cells) {
          if (s.hub === h || !s.networks.includes(h.key)) continue;
          ids.add(s.id);
          if (s.hub) hubs.add(s.hub.key);
        }
        talkedWith(h.cells, ids, hubs);
      } else {
        const n = m.shared.find((x) => x.key === c.id);
        if (!n) return null;
        shared.add(n.key);
        for (const s of n.cells) {
          ids.add(s.id);
          if (s.hub) hubs.add(s.hub.key);
        }
      }
    }
    return { ids, hubs, hyphae, shared, root, machines };
  };

  // Bring the chosen thing to the middle of the map: a cell close up, a
  // network whole.
  const centre = (c, animate = true) => {
    if (!L) return;
    auto = false;
    const [w, h] = seen();
    if (c.kind === "cell") {
      const s = current.byAddress.get(c.id);
      const p = s && pos.get(s.id);
      if (!p) return;
      const kk = Math.max(k, K_NEAR);
      goTo([kk, w / 2 - kk * p[0], h / 2 - kk * p[1]], animate);
    } else if (c.kind === "talk") {
      const points = c.id.split("|").map((end) => spots.get(end)).filter(Boolean);
      if (points.length) goTo(viewOf(boundsOf(points, HUB_R * 2), 1.6), animate);
    } else if (c.kind === "machine") {
      const p = L.machines.get(c.id);
      const points = [p];
      for (const hub of current.hubs.values()) {
        if (hub.within || (hub.machine !== c.id && !hub.feeds.some(([x]) => x === c.id))) continue;
        const [x, y] = L.at.get(hub.key);
        points.push([x, y]);
      }
      goTo(viewOf(boundsOf(points, HUB_R * 2), 1.4), animate);
    } else {
      const hub = current.hubs.get(c.id);
      let points;
      if (hub) {
        const keys = [hub.key, ...hub.children.map((x) => x.key)];
        points = keys.flatMap((key) => {
          const [x, y] = L.at.get(key);
          const r = L.disc.get(key);
          return [
            [x - r, y - r],
            [x + r, y + r],
          ];
        });
        // The whole of the cord it hangs from comes into view too, out to
        // its machine or the network it branches from, so the line that was
        // clicked is never cut off at the edge.
        const root = hub.within ? L.at.get(hub.within) : machinesShown && L.machines.get(hub.machine);
        if (root) points.push(root);
        // So do the cords from every other machine feeding it.
        if (!hub.within && machinesShown) for (const [x] of hub.feeds) if (L.machines.has(x)) points.push(L.machines.get(x));
      } else {
        const n = current.shared.find((x) => x.key === c.id);
        points = (n?.cells || []).map((s) => pos.get(s.id)).filter(Boolean);
      }
      if (points.length) goTo(viewOf(boundsOf(points, R * 2), 2), animate);
    }
  };

  const hashOf = (c) => (c ? `#${c.kind}=${encodeURIComponent(c.id)}` : "");
  const fromHash = () => {
    const m = location.hash.match(/^#(cell|net|machine|talk)=(.+)$/);
    return m ? { kind: m[1], id: decodeURIComponent(m[2]) } : null;
  };

  // A visit goes in the history, so back returns from it.
  // Steps within one find replace each other, so the browser's back leaves
  // the find whole rather than replaying it match by match.
  let depthIn = 0;
  // The cell a talk was opened from, which its back steps to.
  let talkFrom = null;
  const visit = (c, { replace = false } = {}) => {
    hidePeek();
    if (hashOf(c) === location.hash) {
      centre(c);
      return;
    }
    if (replace) history.replaceState(history.state, "", hashOf(c) || location.pathname);
    else history.pushState({ mycelium: ++depthIn }, "", hashOf(c) || location.pathname);
    apply(true);
  };
  // What an open thing hangs from, as something to open: a cell's
  // network, a branch's network, or the whole map for a network on a
  // machine, a shared network or a machine.
  const upFrom = (c) => {
    if (c.kind === "talk") return talkFrom && c.id.split("|").includes(talkFrom) ? { kind: "cell", id: talkFrom.slice(2) } : null;
    const key = keyOf(c);
    const up = key ? treeUp(key) : null;
    return up?.[0] === "h" && current.hubs.has(up.slice(2)) ? { kind: "net", id: up.slice(2) } : null;
  };
  // "-" climbs one level, whatever the history holds: back from a find's
  // steps is never sideways.
  const back = () => {
    if (!chosen) return;
    const up = upFrom(chosen);
    history.pushState({ mycelium: ++depthIn }, "", (up && hashOf(up)) || location.pathname);
    apply(true);
    sayBack();
  };
  // Letting go closes the details and clears the choice where the view
  // stands: nothing moves and the walk stays where it was, since only "-"
  // steps back out.
  const letGo = () => {
    hidePeek();
    if (!chosen) return;
    const inPanel = treeBox.contains(document.activeElement);
    // One press closes the drawer whole, Now with it.
    nowOpen = false;
    history.replaceState(null, "", location.pathname);
    apply(false);
    if (inPanel) view.focus({ preventScroll: true });
    if (view.contains(document.activeElement)) where.textContent = "Let go; the drawer is closed.";
  };
  const sayBack = () => {
    if (!view.contains(document.activeElement)) return;
    const key = keyOf(chosen);
    const on = pointed && spots.has(pointed) ? ` The keys are on ${describe(pointed)}. ${levelWords(pointed)}` : "";
    where.textContent = key ? `Back on ${describe(key)}.${on}` : chosen ? `Back on ${placeless(chosen)}.${on}` : `Back to the whole map.${on}`;
  };
  // A basin's machine is drawn as its basin, so a link to it by its own
  // name, as other sites write one, opens the basin, and the address says so.
  const standFor = (c) => {
    const stand = c?.kind === "machine" ? current?.standOf.get(c.id) : null;
    if (!stand) return c;
    const as = { kind: "machine", id: stand };
    history.replaceState(history.state, "", hashOf(as));
    return as;
  };
  const apply = (animate) => {
    const c = standFor(fromHash());
    chosen = knows(c) ? c : null;
    if (chosen?.kind === "machine") machinesShown = true;
    const key = keyOf(chosen);
    if (key) {
      pointed = key;
      treeActive = key;
      openTo(key);
    }
    // A view that steps is moved before the map is drawn, so the one draw
    // folds for where the view lands; drawn first, the step would unfold
    // what it comes close to, and the map would be drawn twice.
    if (chosen && L && steps(animate)) {
      drawNext = true;
      centre(chosen, animate);
      drawNext = false;
      draw();
    } else {
      draw();
      if (chosen) centre(chosen, animate);
      else if (animate) fit(true);
    }
    panel();
  };
  window.addEventListener("popstate", () => {
    apply(true);
    sayBack();
  });

  // ---- The panel. ----

  const countsOf = (cells) => {
    const n = Object.fromEntries(STANDINGS.map((s) => [s, 0]));
    for (const s of cells) n[s.standing]++;
    return n;
  };
  const countsLine = (cells) => {
    const n = countsOf(cells);
    const bits = [plural(cells.length, kindWord(cells))];
    for (const s of STANDINGS) if (n[s]) bits.push(`${n[s]} ${STANDING_WORD[s]}`);
    return bits.map((b) => b.replaceAll(" ", "\u00a0")).join("\u00a0\u00b7 ");
  };

  const stateGlyph = (s) => {
    if (s.kind === "receptor") return RECEPTOR_GLYPH(s.standing === "moving");
    const ring = s.standing === "asks" ? " m-asks" : s.standing === "stuck" ? " m-stuck" : "";
    const inner =
      s.standing === "dormant"
        ? `<circle class="m-dormant" r="9"/>`
        : `<circle class="m-membrane${ring}" r="9"/>${s.standing === "quiet" ? '<circle class="m-quiet" r="4.5"/>' : '<circle class="m-nucleus" r="5.2"/>'}`;
    return `<svg class="glyph" viewBox="-12 -12 24 24" aria-hidden="true">${inner}</svg>`;
  };

  const cellRow = (s) =>
    `<li><button type="button" class="row" data-go="cell=${esc(s.address)}">${stateGlyph(s)}<span class="what"><span class="name">${esc(s.short)}</span>` +
    `<span class="fact">${esc(s.standing === "asks" ? (s.wants ? `asks: ${s.wants}` : STANDING_WORD.asks) : s.standing === "moving" ? s.doing || "in a turn" : STANDING_WORD[s.standing])}</span></span></button></li>`;

  const hubRow = (h, level = 0) =>
    `<li${level ? ` class="branch"` : ""}><button type="button" class="row" data-go="net=${esc(h.key)}">${glyph(h.shape, 10)}<span class="what"><span class="name">${esc(h.name)}</span>` +
    `<span class="fact">${esc(countsLine(h.cells))}</span></span></button></li>`;

  // ---- At a glance: what a thing is for, what it is doing and how it
  // stands, in one order at the head of every drawer, so a cell, a
  // network, a branch and a machine all read alike. ----

  const CONTEXT_WATCH = 0.75; // a context this full is worth a look
  const CONTEXT_TEND = 0.9; // and this full wants tending
  const HOUR_MS = 3_600_000;
  const DOINGS_SHOWN = 2;
  const PRESSURE_WATCH = 85; // percent of cpu or memory worth a look
  const CHARGE_TEND = 20; // percent of battery left that wants tending
  const LEASE_WATCH_MS = 30 * 60_000; // a sandbox this near its end is worth a look
  // A machine a provider lent says so in what it declares: it leaves at
  // its lease's end. Bare metal never does, so the map tells them apart by
  // what each machine says of itself, never by its name or its provider's.
  const leavesAt = (x) => x?.leaves_at || x?.declared?.leaves_at || null;
  const isSandbox = (x) => leavesAt(x) !== null;
  // A box borrowed from a pool of cloud machines says which basin it is
  // in, in what it declares; never by its name, which the node makes up.
  // A basin's machines are drawn as the one basin they are in (model.ts).
  const isBasin = (x) => Boolean(x?.basin);
  // How many of a basin's machines are off while it runs, in words, or
  // nothing where none is.
  const offWords = (x) => {
    const off = (x?.members || []).filter((y) => y.state !== "ready").length;
    return off ? `${plural(off, "machine")} off` : "";
  };
  // Every machine as itself, a basin's taken out of it.
  const everyMachine = () => current.machines.flatMap((x) => x.members || [x]);
  const machineForm = (x) => (isBasin(x) ? "basin" : isSandbox(x) ? "lent" : "shut");
  const leftWords = (iso) => {
    const ms = Date.parse(iso) - Date.parse(current.at);
    if (!(ms > 0)) return "its time is up";
    const h = Math.floor(ms / HOUR_MS);
    const min = Math.floor((ms % HOUR_MS) / 60_000);
    const span = h && min ? `${h} h ${min} min` : h ? `${h} h` : `${min} min`;
    return `${span} left, until ${dayClock(iso)}`;
  };
  const sandboxWords = (x) => {
    return `a cloud sandbox, ${leftWords(leavesAt(x))}`;
  };
  const VERDICT = ["Well", "Watch", "Tend"];
  const contextShare = (s) => (s.usage?.context?.limit ? s.usage.context.used / s.usage.context.limit : 0);
  const compactCount = (n) => (n >= 1e6 ? `${NUMBER.format(+(n / 1e6).toFixed(1))}M` : `${NUMBER.format(Math.round(n / 1000))}k`);
  const contextWords = (s) => {
    const c = s.usage?.context;
    return c?.limit ? `context ${Math.round(contextShare(s) * 100)}% of ${compactCount(c.limit)}` : "";
  };
  const agoWords = (iso) => {
    const ms = Date.parse(current.at) - Date.parse(iso);
    if (!(ms >= 0)) return atWords(iso);
    return ms < 60_000 ? "under a minute ago" : ms < HOUR_MS ? `${Math.round(ms / 60_000)} min ago` : atWords(iso);
  };
  // Tend where anything waits on him or is stuck, owes an answer or is
  // nearly out of context; watch where a context is filling; else well.
  const verdictOf = (cells) => {
    let v = 0;
    for (const s of cells) {
      const share = contextShare(s);
      if (s.standing === "asks" || s.standing === "stuck" || s.owes.length || share >= CONTEXT_TEND) return 2;
      if (share >= CONTEXT_WATCH) v = 1;
    }
    return v;
  };
  const verdictMark = (v) => `<strong class="verdict v${v}">${VERDICT[v]}</strong>`;
  const groupHealth = (cells) => {
    const n = countsOf(cells);
    const owes = cells.reduce((t, s) => t + s.owes.length, 0);
    const full = cells.filter((s) => contextShare(s) >= CONTEXT_TEND).length;
    const filling = cells.filter((s) => contextShare(s) >= CONTEXT_WATCH && contextShare(s) < CONTEXT_TEND).length;
    const bits = [
      n.asks ? `${n.asks} ${n.asks === 1 ? "needs" : "need"} you` : "",
      n.stuck ? `${n.stuck} stuck` : "",
      owes ? `${plural(owes, "question")} waiting` : "",
      full ? `${full} nearly out of context` : "",
      filling ? `${filling} with context filling` : "",
      ...["moving", "quiet", "dormant"].filter((s) => n[s]).map((s) => `${n[s]} ${STANDING_WORD[s]}`),
    ].filter(Boolean);
    const spent = spentOf(cells);
    return { v: verdictOf(cells), words: `${bits.join(", ") || "no cells"}${spent ? `; ${money(spent)} today` : ""}` };
  };
  // One level of health: a name, its verdict and why. A level that opens
  // is a whole button; the level the drawer is about is not.
  const healthRow = (label, cells, go) => {
    const { v, words } = groupHealth(cells);
    const body = `<span class="what"><span class="name">${esc(label)}</span><span class="fact">${verdictMark(v)}: ${esc(words)}</span></span>`;
    return go ? `<li><button type="button" class="row" data-go="${esc(go)}">${body}</button></li>` : `<li class="row still">${body}</li>`;
  };
  // What a receptor fed a cell, in words, by the node's word for its kind:
  // GitHub's review asked of Joel, or the calls receptor's summary of a
  // call the cell's quest claims.
  const FED_WORDS = { review: "a review asked of you", summary: "a call's summary" };
  const fedWords = (kind) => (Object.hasOwn(FED_WORDS, kind) ? FED_WORDS[kind] : "something from outside");
  const factList = (facts) => (facts.length ? `<dl class="facts">${facts.map(([k2, v]) => `<div><dt>${k2}</dt><dd>${esc(v)}</dd></div>`).join("")}</dl>` : "");
  const glance = (purpose, doing, health) =>
    `<dl class="glance">` +
    `<div><dt>Purpose</dt><dd>${purpose}</dd></div>` +
    `<div><dt>Doing</dt><dd>${doing}</dd></div>` +
    `<div><dt>Health</dt><dd>${health}</dd></div></dl>`;
  const lastTurnOf = (cells) =>
    cells
      .map((s) => s.usage?.last_turn)
      .filter(Boolean)
      .sort()
      .at(-1);
  // What a set of cells is doing: the moving ones, by what they say.
  const groupDoing = (cells) => {
    const moving = cells.filter((s) => s.standing === "moving");
    if (!moving.length) {
      const asks = cells.filter((s) => s.standing === "asks").length;
      const last = lastTurnOf(cells);
      return asks ? `Nothing moving; ${plural(asks, "cell")} waiting on you.` : `Nothing moving${last ? `; the last turn ended ${agoWords(last)}` : ""}.`;
    }
    const shown = moving.slice(0, DOINGS_SHOWN).map((s) => `${s.short}: ${(s.doing || "in a turn").replace(/[.\s]+$/, "")}`);
    const more = moving.length - shown.length;
    return `${plural(moving.length, "cell")} moving. ${shown.join("; ")}${more ? `; and ${more} more` : ""}.`;
  };
  // The receptors, each said in full, since there are only a few and each
  // hears something different.
  const sensesDoing = (cells) => {
    const reading = cells.filter((s) => s.standing === "moving").length;
    return `${reading ? `${plural(reading, "receptor")} reading` : "Nothing reading"}. ${cells.map((s) => `${s.short}: ${(s.doing || "listening").replace(/[.\s]+$/, "")}`).join("; ")}.`;
  };
  const cellDoing = (s) => {
    if (s.standing === "moving") return s.doing || "In a turn now";
    if (s.standing === "asks") return s.wants ? `Asks you: ${s.wants}` : "Waiting on you";
    if (s.standing === "stuck") return "Stuck: paused, waiting to be resumed";
    if (s.standing === "dormant") return "Parked, its place kept";
    return `Quiet, waiting for its next word${s.usage?.last_turn ? `; its last turn ended ${agoWords(s.usage.last_turn)}` : ""}`;
  };
  const cellHealth = (s) => {
    const u = s.usage || {};
    const owed = s.owes.map((o) => `${current.byAddress.get(o.from)?.short || o.from}'s question waiting since ${dayClock(o.at)}`);
    const today = [u.turns_today != null ? plural(u.turns_today, "turn") : "", u.cost_today ? money(u.cost_today) : ""].filter(Boolean).join(" and ");
    const bits = [s.standing === "asks" ? "needs you" : "", s.standing === "stuck" ? "stuck" : "", ...owed, ...s.holds.map(holdWords), contextWords(s), today ? `${today} today` : ""].filter(Boolean);
    return `<p class="health-line">${verdictMark(verdictOf([s]))}: ${esc(bits.join("; ") || "nothing said")}.</p>`;
  };
  const leaseShort = (x) => {
    const end = isSandbox(x) && leavesAt(x);
    return Boolean(end) && Date.parse(end) - Date.parse(current.at) < LEASE_WATCH_MS;
  };
  // A basin's machines each take a row of their own, named, their cells
  // counted first, since the basin's drawer is the only place they show.
  const machineHealth = (x, { label = "The machine", lead = [], mono = false } = {}) => {
    const p = x.pressure;
    const charge = x.power?.on_battery ? x.power.charge : null;
    const v =
      x.state !== "ready" || x.over?.length || (charge != null && charge < CHARGE_TEND) ? 2 : (p && (p.cpu >= PRESSURE_WATCH || p.memory >= PRESSURE_WATCH)) || x.disk_band === "filling" || leaseShort(x) ? 1 : 0;
    const bits = [
      ...lead,
      x.state !== "ready" ? x.state : "",
      p ? `cpu ${Math.round(p.cpu)}%, memory ${Math.round(p.memory)}%, io ${Math.round(p.io)}%` : "",
      x.over?.length ? `${x.over.join(" and ")} short` : "",
      x.available != null ? `${gib(x.available)} memory free` : "",
      x.disk ? `${gib(x.disk_free)} of ${gib(x.disk)} disk free${x.disk_band === "filling" ? ", filling" : ""}` : "",
      charge != null ? `on battery, ${charge}%` : "",
      isSandbox(x) && leavesAt(x) ? leftWords(leavesAt(x)) : "",
    ].filter(Boolean);
    return `<li class="row still"><span class="what"><span class="name${mono ? " mono" : ""}">${esc(label)}</span><span class="fact">${verdictMark(v)}: ${esc(bits.join("; ") || "nothing said")}</span></span></li>`;
  };

  // The realm's balance, as the balance service reads it each minute: how
  // much of the realm's work each machine carries, and what it holds wrong
  // over most of the last ten minutes. Said in the words `mycelium balance`
  // says it in. A held flag is worth watching, never a fault: placing is
  // the realm's, not the machine's.
  const loadWords = (load) => (typeof load === "number" ? `${Math.round(load * 100)}%` : "");
  const STANDS_WORDS = { over: "over its limits", room: "with room for more", full: "with no room for more" };
  const shareWords = (share) => {
    const stands = share.over ? STANDS_WORDS.over : share.room ? STANDS_WORDS.room : STANDS_WORDS.full;
    const load = loadWords(share.load);
    const each = typeof share.busy_per_core === "number" ? `, ${Number(share.busy_per_core.toFixed(2))} busy a core` : "";
    const sessions = typeof share.busy === "number" ? `; ${plural(share.busy, "cell")} busy and ${NUMBER.format(share.idle || 0)} idle${each}` : "";
    return `${load ? `${load} loaded, ` : ""}${stands}${sessions}`;
  };
  const heldFlags = () => (Array.isArray(current.balance?.held) ? current.balance.held.filter((f) => f && typeof f.flag === "string") : []);
  const flagNames = (f) => [f.hot, f.cool, f.machine].filter(Boolean);
  // In a machine's own drawer, `self` is that machine, said as "it".
  const flagWords = (f, self = null) => {
    const its = (name) => (name === self ? "its" : `${name}'s`);
    if (f.flag === "hot_while_cool" && f.hot === self) return `it is over while ${f.cool} has room, so new work should go to ${f.cool}`;
    if (f.flag === "hot_while_cool" && f.cool === self) return `${f.hot} is over while this has room, so new work should come here`;
    if (f.flag === "hot_while_cool") return `${f.hot} is over while ${f.cool} has room, so new work should go to ${f.cool}`;
    if (f.flag === "stranded" && f.spent === "memory") return `${its(f.machine)} memory is nearly all used while its processors idle`;
    if (f.flag === "stranded") return `${its(f.machine)} processors are spent while half its memory or more is free`;
    if (f.flag === "stale") return `${its(f.machine)} facts are too old to place by`;
    if (f.flag === "imbalanced") return "the machines' loads are uneven, so new work leans to the least loaded";
    return f.flag.replace(/_/g, " ");
  };
  const machineShareRow = (x, label = "Its share") => {
    if (!x.share) return "";
    const held = heldFlags().filter((f) => flagNames(f).includes(x.name));
    const bits = [shareWords(x.share), ...held.map((f) => flagWords(f, x.name))];
    return `<li class="row still"><span class="what"><span class="name">${esc(label)}</span><span class="fact">${verdictMark(held.length ? 1 : 0)}: ${esc(bits.join("; "))}</span></span></li>`;
  };
  const balanceRow = () => {
    if (!current.balance) return "";
    const loads = everyMachine().filter((x) => x.share && loadWords(x.share.load)).map((x) => `${x.name} ${loadWords(x.share.load)}`);
    if (loads.length < 2) return "";
    const held = heldFlags();
    const said = held.length ? held.map((f) => flagWords(f)) : ["even"];
    return `<li class="row still"><span class="what"><span class="name">The balance</span><span class="fact">${verdictMark(held.length ? 1 : 0)}: ${esc(`${said.join("; ")}; loads ${loads.join(", ")}`)}</span></span></li>`;
  };

  // What holds others up, oldest first, said as plainly as the node knows it.
  const heldRow = (s) => {
    const o = s.owes[0];
    const asker = clipped(current.byAddress.get(o.from)?.short || o.from);
    return (
      `<li><button type="button" class="row" data-go="cell=${esc(s.address)}">${stateGlyph(s)}<span class="what"><span class="name">${esc(s.short)}</span>` +
      `<span class="fact">${esc(`${waitingWords(s.owes)}; ${s.owes.length === 1 ? "it came" : "the oldest came"} from ${asker} ${minutesWord(ageOf(s.owes))} ago, ${unansweredWords()}`)}</span></span></button></li>`
    );
  };
  // A hold puts off what others would do, which is a choice and no fault,
  // so it is said beside what waits, and never counted against health.
  const holdRow = (s) =>
    `<li><button type="button" class="row" data-go="cell=${esc(s.address)}">${stateGlyph(s)}<span class="what"><span class="name">${esc(s.short)}</span>` +
    `<span class="fact">${esc(s.holds.map(holdWords).join("; "))}</span></span></button></li>`;
  // Who was born and who ended in the last ten minutes, newest first. A
  // born cell is a way to it; an ended one is only a fact, since there is
  // nothing left to open.
  const LIFE_ROWS = 10;
  const GONE_GLYPH = `<svg class="glyph" viewBox="-12 -12 24 24" aria-hidden="true"><circle class="ghost" r="9"/></svg>`;
  const lifeLines = () => {
    const rows = [
      ...[...born].filter(([id]) => current.byId.has(id)).map(([id, at]) => ({ at, s: current.byId.get(id) })),
      ...[...ended].map(([, e]) => ({ at: e.at, e })),
    ].sort((a, b) => b.at - a.at);
    if (!rows.length) return "";
    const row = ({ at, s, e }) =>
      s
        ? `<li><button type="button" class="row" data-go="cell=${esc(s.address)}">${stateGlyph(s)}<span class="what"><span class="name">${esc(s.short)}</span><span class="fact">born ${esc(atWords(at))}</span></span></button></li>`
        : `<li><p class="row gone">${GONE_GLYPH}<span class="what"><span class="name">${esc(e.short)}</span><span class="fact">ended ${esc(atWords(at))}</span></span></p></li>`;
    const more = rows.length - LIFE_ROWS;
    return (
      `<h3>Born and Ended Lately</h3><ul class="rows">${rows.slice(0, LIFE_ROWS).map(row).join("")}</ul>` +
      (more > 0 ? `<p class="hint">and ${esc(plural(more, "more", "more"))} in the last ten minutes.</p>` : "")
    );
  };
  const heldUp = (cells) => {
    const held = cells.filter((s) => s.owes.length).sort((a, b) => a.owes[0].at.localeCompare(b.owes[0].at));
    const holding = cells.filter((s) => s.holds.length).sort((a, b) => a.holds[0].at.localeCompare(b.holds[0].at));
    return (
      (held.length ? `<h3>Questions Waiting</h3><ul class="rows">${held.map(heldRow).join("")}</ul>` : "") +
      (holding.length ? `<h3>Holds Standing</h3><ul class="rows">${holding.map(holdRow).join("")}</ul>` : "")
    );
  };

  // Signal against noise: how much of the talk asked or found something,
  // against what was sent for context only.
  const SIGNAL = new Set(["question", "finding", "hold", "pivot"]);
  const talkLine = (messages, among) => {
    if (!messages.length) return "";
    const said = messages.filter((x) => SIGNAL.has(x.intent)).length;
    const of = among ? `Of the latest ${current.messages.length} messages, ${plural(messages.length, "touches", "touch")} it` : `Of the latest ${plural(messages.length, "message")}`;
    return `<p class="talk-line">${esc(`${of}: ${said} asked or found something, and ${messages.length - said} ${messages.length - said === 1 ? "was" : "were"} for context only.`)}</p>`;
  };
  const touching = (cells, key) => {
    const peers = new Set(cells.map((s) => `peer.${s.address.toLowerCase()}`));
    const from = new Set(cells.map((s) => s.address));
    return current.messages.filter((x) => from.has(x.from) || (x.groups || []).some((g) => g === key || peers.has(g)));
  };

  const worstFirst = (a, b) => STANDINGS.indexOf(a.standing) - STANDINGS.indexOf(b.standing) || collate(a.short, b.short);

  // Spent today, as a sum that opens on what it counts: the parts it
  // splits into, then the cells that spent most, highest first.
  const SPEND_CELLS = 10;
  const TURN = `<svg class="turn" viewBox="0 0 10 10" aria-hidden="true"><path d="M3 1.5L7 5L3 8.5"/></svg>`;
  const BY_SPEND = new WeakMap();
  const bySpend = (cells) => {
    if (!BY_SPEND.has(cells)) BY_SPEND.set(cells, cells.filter((s) => spentBy(s) > 0).sort((a, b) => spentBy(b) - spentBy(a)));
    return BY_SPEND.get(cells);
  };
  // A part of a sum: its cells, and the row that opens it.
  const spendRow = (go, mark, name, cells, mono = "") => [
    cells,
    `<li><button type="button" class="row" data-go="${esc(go)}">${mark}<span class="what"><span class="name${mono}">${esc(name)}</span>` +
      `<span class="fact">${esc(`${money(spentOf(cells))}, ${plural(cells.length, "cell")}`)}</span></span></button></li>`,
  ];
  // The most a list shows; the rest are counted, and their sum said.
  const PART_WORDS = { machine: ["Machine", "Machines", "machines"], network: ["Network", "Networks", "networks"], branch: ["Branch", "Branches", "branches"], cell: ["Cell", "Cells", "cells"] };
  const spendList = (head, rows, rest, noun) =>
    `<h3>${head}</h3><ul class="rows">${rows.join("")}</ul>` +
    (rest.length ? `<p class="hint">${esc(`${plural(rest.length, `more ${noun}`, `more ${PART_WORDS[noun][2]}`)} spent ${money(rest.reduce((t, cells) => t + spentOf(cells), 0))} between them.`)}</p>` : "");
  const spendCell = (s) =>
    `<li><button type="button" class="row" data-go="cell=${esc(s.address)}">${stateGlyph(s)}<span class="what"><span class="name">${esc(s.short)}</span>` +
    `<span class="fact">${esc([money(spentBy(s)), s.usage?.turns_today != null ? plural(s.usage.turns_today, "turn") : ""].filter(Boolean).join(", "))}</span></span></button></li>`;
  // parts: [a key of PART_WORDS, parts from spendRow in spend order].
  const spendBlock = (cells, parts = []) => {
    if (!spends(cells)) return "";
    const top = bySpend(cells);
    const shown = parts.filter(([, rows]) => rows.length > 1);
    return (
      `<details class="fold spend" data-keep="spend"><summary>${TURN}<span>Spent Today</span><span class="sum mono">${esc(money(spentOf(cells)))}</span></summary>` +
      `<p class="hint">Since midnight, at the list price of each call, as the sessions count it; the bill may differ.</p>` +
      shown
        .map(([noun, rows]) =>
          spendList(
            rows.length > SPEND_CELLS ? `The ${SPEND_CELLS} ${PART_WORDS[noun][1]} That Spent Most` : `By ${PART_WORDS[noun][0]}`,
            rows.slice(0, SPEND_CELLS).map(([, row]) => row),
            rows.slice(SPEND_CELLS).map(([part]) => part),
            noun,
          ),
        )
        .join("") +
      (top.length
        ? spendList(
            top.length > SPEND_CELLS ? `The ${SPEND_CELLS} Cells That Spent Most` : "Its Cells",
            top.slice(0, SPEND_CELLS).map(spendCell),
            top.slice(SPEND_CELLS).map((s) => [s]),
            "cell",
          )
        : "") +
      `</details>`
    );
  };
  // A network's rows count its branches' cells with its own, so a list of
  // networks adds up to the sum above it; on a machine, only the cells
  // that run there.
  // A network's own cells, beside its branches, so the branches and it
  // add up to its sum; a row to read, not a way to go, since it is here.
  const ownSpendRow = (h) =>
    spends(h.cells)
      ? [
          [
            h.cells,
            `<li><span class="row">${glyph(h.shape, 10)}<span class="what"><span class="name">Its own cells</span>` +
              `<span class="fact">${esc(`${money(spentOf(h.cells))}, ${plural(h.cells.length, "cell")}`)}</span></span></span></li>`,
          ],
        ]
      : [];
  const hubSpendRows = (hubs, machine = null) =>
    hubs
      .map((h) => [h, machine ? allCells(h).filter((s) => s.machine === machine) : allCells(h)])
      .filter(([, cells]) => spentOf(cells) > 0)
      .sort(([, a], [, b]) => spentOf(b) - spentOf(a))
      .map(([h, cells]) => spendRow(`net=${h.key}`, glyph(h.shape, 10), h.name, cells));

  // A cell that says nothing of what it is for serves no use he can see,
  // so Now and its machine name it as the fault to tend.
  const noPurpose = (s) => !s.purpose && !s.quest;
  const faultLine = (cells) => {
    const faults = cells.filter(noPurpose);
    if (!faults.length) return "";
    const more = faults.length - FAULT_ROWS;
    return (
      `<p class="asks-line">${esc(`No purpose written yet for ${faults.length === 1 ? "one cell" : plural(faults.length, "cell")}:`)}</p>` +
      `<ul class="rows">${faults.slice(0, FAULT_ROWS).map(cellRow).join("")}</ul>` +
      (more > 0 ? `<p class="hint">${esc(`and ${NUMBER.format(more)} more`)}</p>` : "")
    );
  };
  const overview = () => {
    const m = current;
    const used = new Set(m.cells.map((s) => s.host));
    const asks = m.cells.filter((s) => s.standing === "asks");
    const tops = [...m.hubs.values()].filter((h) => !h.within && !h.alone && !h.edge);
    const sensing = m.hubs.get(OUTSIDE)?.cells.length || 0;
    const levels = m.machines.filter((x) => cellsOn(x.name).length).map((x) => healthRow(x.name, cellsOn(x.name), `machine=${x.name}`));
    return (
      `<h2 id="panel-title">Now</h2>` +
      `<p class="carries">${esc(`Your whole network: ${plural(m.cells.length - sensing, "cell")} in ${plural(tops.length, "orchestration")}${sensing ? `, ${plural(sensing, "receptor")} at its edge` : ""}, on ${used.size} of ${plural(everyMachine().length, "machine")}.`)}</p>` +
      glance(
        esc("Every cell you run, the networks they work in and the machines they run on."),
        esc(groupDoing(m.cells)),
        `<ul class="rows health">${levels.join("")}${levels.length > 1 ? healthRow("All together", m.cells) : ""}${balanceRow()}</ul>`,
      ) +
      (asks.length
        ? `<p class="asks-line">${asks.length === 1 ? "One needs" : `${asks.length} need`} you:</p><ul class="rows">${asks.map(cellRow).join("")}</ul>`
        : `<p class="asks-line quiet">Nobody needs you right now.</p>`) +
      faultLine(m.cells) +
      heldUp(m.cells) +
      lifeLines() +
      talkLine(m.messages) +
      spendBlock(m.cells, [
        [
          "machine",
          m.machines
            .filter((x) => spentOf(cellsOn(x.name)) > 0)
            .sort((a, b) => spentOf(cellsOn(b.name)) - spentOf(cellsOn(a.name)))
            .map((x) => spendRow(`machine=${x.name}`, MACHINE_GLYPH, x.name, cellsOn(x.name), " mono")),
        ],
        ["network", hubSpendRows([...m.hubs.values()].filter((h) => !h.within))],
      ]) +
      `<p class="hint">Walk the map with the arrow keys, or the network below as a tree; Return opens what you reach here.</p>` +
      `<details class="fold legend" data-keep="legend"><summary>${TURN}How to Read the Map</summary>` +
      `<dl>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-membrane" r="11"/><circle class="m-nucleus" r="6.5"/></svg></dt><dd>A cell moving: in a turn now</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-membrane" r="11"/><circle class="m-quiet" r="5.5"/></svg></dt><dd>Quiet: waiting for its next word</dd></div>` +
      `<div><dt>${RECEPTOR_GLYPH(true)}</dt><dd>A receptor: a dish at the network's edge where mail, calls or the calendar come in, its dot filled while it is reading</dd></div>` +
      (current?.fed ? `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-fed" d="M-12,-6Q0,12 12,-6"/></svg></dt><dd>Fed: a dot-and-dash arc from a receptor to the one cell it fed something from outside, such as a review asked of you or a call's summary, drawn for ten minutes</dd></div>` : "") +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-dormant" r="11"/></svg></dt><dd>Dormant: parked, its place kept</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-membrane m-asks" r="11"/><circle class="m-nucleus" r="6.5"/></svg></dt><dd>Needs you, or stuck: its ring says so, and its name shows as you come close</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-waits" r="13"/><circle class="m-membrane" r="9"/><circle class="m-quiet" r="4.5"/></svg></dt><dd>A question waiting: a dashed ring round a cell asked something three minutes or more ago that ${m.judged ? "hasn't answered it yet" : "has said nothing since"}, wider the more questions and the older the first; a network holding any is ringed round its ground</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-holds" d="M0,-14V14"/><circle class="m-membrane" r="9"/><circle class="m-quiet" r="4.5"/></svg></dt><dd>A hold standing: a wall drawn across a cell that asked the others to put something off until it says it's over</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-cord" d="M-14,0H-9M9,0H14" style="--w:2px"/><rect class="m-box" x="-9" y="-6" width="18" height="12"/></svg></dt><dd>A machine: its name in a hairline box, its cords leaving the box's edge; the box is dashed while the machine is off</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><rect class="m-box m-lent" x="-10" y="-7" width="20" height="14" style="stroke-dasharray:9 11 9 5;stroke-dashoffset:4.5"/></svg></dt><dd>A lent machine, here until its lease ends: only its box's corners</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><rect class="m-box m-lent" x="-10" y="-7" width="20" height="14" style="stroke-dasharray:0 20 48"/></svg></dt><dd>A basin: machines borrowed from a pool in the cloud, drawn as one box open at the top and named by the basin; its drawer lists each machine</dd></div>` +
      `<div><dt><svg viewBox="-17 -17 34 34" aria-hidden="true"><path class="m-spokes" d="${spokesPath(10)}" style="stroke-width:1.75"/><circle class="m-hub" r="10"/><path class="m-hub-shape" d="${SHAPES.square(5)}"/></svg></dt><dd>A network's hub, a knot: spokes round its ring, and its own shape in the middle; a click on its ground opens it</dd></div>` +
      `<div><dt><svg viewBox="-17 -17 34 34" aria-hidden="true"><path class="m-spokes" d="${spokesPath(10)}" style="stroke-width:1.75"/><circle class="m-hub" r="10"/></svg></dt><dd>A network with no leader, shared by cells of other networks: the same knot, empty in the middle; the cells it shares are the bridge between them</dd></div>` +
      (m.cells.length > FOLD_OVER
        ? `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-colony folded" r="12"/><circle class="m-hub" r="7"/></svg></dt><dd>A folded network: drawn too small to read, or off the map, so its cells wait in its ground until you come closer or open it</dd></div>`
        : "") +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><g class="hub quiet" transform="scale(0.82)"><path class="m-spokes" d="${spokesPath(10)}"/><circle class="m-hub" r="10"/><path class="m-hub-shape" d="${SHAPES.square(5)}"/></g></svg></dt><dd>A quiet network: none of its cells at work, so drawn lighter</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-cord" d="M-13,0H13" style="--w:4px"/></svg></dt><dd>A cord: machine to hub, or hub to branch; thicker carries more</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-hypha" d="M-13,4Q0,-8 13,4"/></svg></dt><dd>Hyphae: a hub's cells, branching out from it</dd></div>` +
      (WARMS ? `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-talk warm3" d="M-12,6Q0,-12 12,6"/></svg></dt><dd>${current?.ties ? "Talk: an arc between two cells that messaged each other today, thicker the more they said lately, and thinner the longer they have been quiet" : "Talk: an arc between two that messaged in the last ten minutes, thicker the more they said"}</dd></div>` : "") +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-cross m-repository" d="M-13,4Q0,-8 13,4"/></svg></dt><dd>Dotted cross-hyphae: cells sharing a repository, group or quest</dd></div>` +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><circle class="m-changed" r="13"/><circle class="m-membrane" r="11"/><circle class="m-quiet" r="5.5"/></svg></dt><dd>A faint outer ring: changed since you last looked</dd></div>` +
      (PULSES
        ? `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="m-talk warm1" d="M-12,6Q0,-12 12,6"/><circle class="pulse" cx="0" cy="-3" r="4.5"/></svg></dt><dd>One dot is one message, flying straight from the cell that sent it to the one it was for; a ring opens round each. A cell it passes over took no part${still.matches ? "; still while your system asks for less motion" : ""}</dd></div>`
        : "") +
      `<div><dt><svg viewBox="-14 -14 28 28" aria-hidden="true"><path class="birth" d="M-12,8L2,-4"/><circle class="ghost" cx="6" cy="-6" r="6"/></svg></dt><dd>Born: a hypha draws itself out of the hub to the new cell. Ended: its membrane shrinks back, dashed. Both stay listed in Now for ten minutes${PULSES && still.matches ? ", and only listed while your system asks for less motion" : ""}</dd></div>` +
      `</dl></details>`
    );
  };

  const machinePanel = (name) => {
    const m = current;
    const x = m.machines.find((y) => y.name === name);
    if (!x) return overview();
    const power = x.power ? (x.power.on_battery ? `battery, ${x.power.charge}%` : `plugged in, ${x.power.charge}%`) : "";
    const cells = cellsOn(name);
    const pool = x.members || null;
    const what = [pool ? plural(pool.length, "machine") : "", x.model_name, x.os, x.cores ? plural(x.cores, "core") : "", x.memory ? `${gib(x.memory)} memory` : ""]
      .filter(Boolean)
      .join(", ");
    // A basin's health is each of its machines', said one to a row.
    const health = pool
      ? pool.map((y) => machineHealth(y, { label: y.name, mono: true, lead: [plural(cells.filter((s) => s.host === y.name).length, "cell")] }) + machineShareRow(y, `${y.name}'s share`)).join("")
      : machineHealth(x) + machineShareRow(x);
    const says = pool
      ? `A basin, drawn as one machine: ${plural(pool.length, "machine")} borrowed from a pool in the cloud. A cord`
      : isSandbox(x)
        ? `A machine: ${sandboxWords(x)}. A cord`
        : "A machine; a cord";
    const facts = [
      ["Made of", what],
      ["Power", power],
    ].filter(([, v]) => v);
    const alone = [...m.hubs.values()].filter((h) => h.alone && h.machine === name);
    // Every network it feeds, most of its cells first: its cord runs to
    // each, weighted by what it runs there, and none of them is its own.
    const feeds = [...m.hubs.values()]
      .filter((h) => !h.within && !h.alone && !h.edge)
      .map((h) => [h, h.feeds.find(([x]) => x === name)?.[1] || 0])
      .filter(([, n]) => n)
      .sort((a, b) => b[1] - a[1] || collate(a[0].name, b[0].name));
    const feedRow = ([h, n]) => {
      const rest = allCells(h).filter((s) => s.machine !== name);
      const fact = rest.length ? `${plural(n, "cell")} here, ${NUMBER.format(rest.length)} on ${machinesOf(rest)}` : `${plural(n, "cell")}, all here`;
      return `<li><button type="button" class="row" data-go="net=${esc(h.key)}">${glyph(h.shape, 10)}<span class="what"><span class="name">${esc(h.name)}</span><span class="fact">${esc(fact)}</span></span></button></li>`;
    };
    const lone = alone.reduce((sum, h) => sum + h.cells.length, 0);
    return (
      backButton("Now") +
      `<h2 id="panel-title" class="mono">${esc(name)}</h2>` +
      `<p class="carries">${esc(`${says} runs from it to each network with cells here, heavier where more of them run.`)}</p>` +
      glance(
        esc(`Runs ${plural(cells.length, "cell")}, feeding ${feeds.length ? plural(feeds.length, "network") : "no network"}${lone ? `, with ${NUMBER.format(lone)} working alone` : ""}.`),
        esc(groupDoing(cells)),
        `<ul class="rows health">${health}${healthRow(`Its ${plural(cells.length, "cell")}`, cells)}</ul>`,
      ) +
      faultLine(cells) +
      // Its home as a device is the network page, which knows it by name.
      // A sandbox or a basin isn't a device on the home network, so it has
      // no row there.
      (pool || isSandbox(x) ? "" : `<p class="led"><a class="inline" href="../network/#host=${encodeURIComponent(name)}">${esc(name)} on the network page</a></p>`) +
      factList(facts) +
      spendBlock(cells, [["network", hubSpendRows([...m.hubs.values()].filter((h) => !h.within), name)]]) +
      (feeds.length ? `<h3>Its Networks</h3><ul class="rows">${feeds.map(feedRow).join("")}</ul>` : `<p class="hint">It feeds no network.</p>`) +
      (alone.length ? `<h3>Working Alone</h3><ul class="rows">${alone.map((h) => hubRow(h)).join("")}</ul>` : "")
    );
  };

  // Where Joel says something himself: a link the desktop hands the
  // launcher (modules/quickshell/mycelium-link), opening Agents' Talk to a
  // cell or Networks on a network, so the page still writes nothing. The
  // link names it and nothing more, in the letters mycelium-link lets
  // through, so a name it would refuse is never offered as a link that
  // does nothing.
  // A cell's address is 1 to 12 capitals and digits; a network's name
  // takes more.
  const LINK_NAME = { cell: /^[A-Z0-9]{1,12}$/, network: /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/ };
  const launcherLink = (kind, name, words) =>
    LINK_NAME[kind]?.test(name || "") ? `<p class="led"><a class="inline" href="mycelium:${kind}/${name}">${esc(words)}</a></p>` : "";

  const backButton = (label) =>
    `<button type="button" class="back" data-back><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M7 1.5L3 5L7 8.5"/></svg>${esc(label)}</button>`;

  const who = (group) => {
    const peer = group.match(/^peer\.(.+)$/);
    if (peer) return current.byAddress.get(peer[1].toUpperCase())?.short || peer[1].toUpperCase();
    return current.hubs.get(group)?.name || group;
  };

  // Why a cell is in no network, in the words of what its machine sent.
  const cellPanel = (s) => {
    const m = current;
    const u = s.usage || {};
    const facts = [
      ["Repository", s.repo],
      ["Started", s.since ? atWords(s.since) : ""],
      ["Last turn", u.last_turn ? agoWords(u.last_turn) : ""],
    ].filter(([, v]) => v);
    // Its machine as drawn, a basin for a basin's machine; and the machine
    // itself, which says its lease.
    const box = current.machines.find((x) => x.name === s.machine);
    const runs = everyMachine().find((x) => x.name === s.host);
    const named = box?.members ? `${s.host}, in the ${box.basin} basin` : s.machine || "no machine";
    const on = `${named}${isSandbox(runs) ? `, ${sandboxWords(runs)}` : ""}`;
    const home =
      s.kind === "receptor" ? `at the network's edge, on ${on}, where ${s.purpose || "the outside"} comes in` : s.hub ? `in the ${s.hub.name} network, on ${on}` : `working alone on ${on}`;
    const receptor = s.kind === "receptor";
    const purpose = receptor ? `Brings ${s.purpose || "what it hears"} in from outside, for the cells that read it.` : s.purpose || NO_PURPOSE;
    // What a receptor fed it, or what it fed, in the last ten minutes.
    const fedFact = (m.fed || [])
      .filter((f) => f.cell === s.address || f.receptor === s.address)
      .map((f) => (f.cell === s.address ? `${fedWords(f.kind)}, by ${m.byAddress.get(f.receptor)?.short || f.receptor}` : `${m.byAddress.get(f.cell)?.short || f.cell}, ${fedWords(f.kind)}`) + `, at ${clock(f.at, false)}`)
      .join("; ");
    if (fedFact) facts.push(["Fed", fedFact]);
    const networks = s.networks.map((name) => {
      const h = m.hubs.get(name);
      if (h) return `<li><button type="button" class="row" data-go="net=${esc(h.key)}">${glyph(h.shape, 10)}<span class="what"><span class="name">${esc(h.name)}</span><span class="fact">${h === s.hub ? "its own" : "also on it"}${s.leads.includes(h.key) ? ", leads it" : ""}</span></span></button></li>`;
      const n = m.shared.find((x) => x.key === name);
      if (n) return `<li><button type="button" class="row" data-go="net=${esc(n.key)}"><svg class="glyph" viewBox="-12 -12 24 24" aria-hidden="true"><path class="m-cross m-${n.shape}" d="M-11,4Q0,-8 11,4"/></svg><span class="what"><span class="name mono">${esc(n.short)}</span><span class="fact">${SHARED_WORD[n.shape]}, ${plural(n.cells.length, "cell")}</span></span></button></li>`;
      return "";
    });
    // The networks its own sits within, the nearest first, so where it
    // sits reads up to the top without the whole tree under each.
    const above = [];
    for (let h = s.hub?.within && m.hubs.get(s.hub.within); h; h = h.within && m.hubs.get(h.within)) above.push(hubRow(h));
    // The cells it talks with, the most first, as the map's arcs draw them.
    const own = `c:${s.address}`;
    const talksWith = [...talk.entries()]
      .map(([e, n]) => [e.split("|"), n])
      .filter(([ends]) => ends.includes(own))
      .map(([ends, n]) => [ends.find((end) => end !== own), n])
      .filter(([end]) => end?.[0] === "c" && m.byAddress.has(end.slice(2)))
      .sort((a, b) => b[1] - a[1])
      .slice(0, TALKS_SHOWN)
      .map(([end]) => cellRow(m.byAddress.get(end.slice(2))));
    const where =
      `<details class="fold where" data-keep="where"><summary>${TURN}<span>Where It Sits</span>` +
      `<span class="sum mono">${esc(plural(networks.filter(Boolean).length, "network") + (talksWith.length ? `\u00a0\u00b7 talks with ${talksWith.length}` : ""))}</span></summary>` +
      (box ? `<h3>Its Machine</h3><ul class="rows">${machineRow(box)}</ul>` : "") +
      `<h3>Its Networks</h3><ul class="rows">${networks.join("")}${above.join("")}</ul>` +
      `<h3>Talks With</h3>` +
      (talksWith.length ? `<ul class="rows">${talksWith.join("")}</ul>` : `<p class="hint">No cell it has talked with lately.</p>`) +
      `</details>`;
    const peer = `peer.${s.address.toLowerCase()}`;
    const lately = m.messages
      .filter((x) => x.from === s.address || (x.groups || []).includes(peer))
      .sort((a, b) => b.at.localeCompare(a.at))
      .slice(0, LATELY)
      .map((x) => {
        const sent = x.from === s.address;
        const whom = sent ? `to ${(x.groups || []).map(who).join(", ") || "nobody named"}` : `from ${m.byAddress.get(x.from)?.short || x.from}`;
        const said = `<span class="when mono">${esc(clock(x.at, false))}</span><span>${esc(`${x.intent || "message"} ${whom}`)}</span>`;
        // A message with one other end opens the talk between the two, so
        // the keys reach every talk the map draws.
        const ends = sent ? receiversOf(x) : spots.has(`c:${x.from}`) ? [`c:${x.from}`] : [];
        const pair = ends.length === 1 ? pairKey(`c:${s.address}`, ends[0]) : null;
        return pair && talkEndsKnown(pair) ? `<li><button type="button" class="said" data-go="talk=${esc(pair)}">${said}</button></li>` : `<li>${said}</li>`;
      });
    return (
      backButton(netOf(s) ? netOf(s).name : "Now") +
      `<h2 id="panel-title">${esc(s.short)}</h2>` +
      `<p class="carries">${esc(`A cell ${home}.`)}</p>` +
      glance(esc(purpose), `<span class="doing">${stateGlyph(s)}<span>${esc(cellDoing(s))}</span></span>`, cellHealth(s)) +
      (receptor ? "" : launcherLink("cell", s.address, "Talk to It in the Launcher")) +
      factList(facts) +
      where +
      `<h3>Lately</h3>` +
      (lately.length ? `<ul class="lately">${lately.join("")}</ul>` : `<p class="hint">No message to or from it among the latest the node keeps.</p>`)
    );
  };

  // A talk arc opened: the two it joins, and what passed between them in
  // the last ten minutes, by kind and time, never the words.
  const endName = (end) => (end[0] === "h" ? current.hubs.get(end.slice(2))?.name : current.byAddress.get(end.slice(2))?.short) || end.slice(2);
  const talkWords = (id) => `talk between ${id.split("|").map(endName).join(" and ")}`;
  // What a click or a key opened, in the words #where says it in.
  const openedWords = (c) => {
    if (c.kind === "talk") return tieOf.has(c.id) ? `${talkWords(c.id)}: ${plural(tieSum(tieOf.get(c.id).today), "message")} today` : `${talkWords(c.id)}: ${plural((talkSaid.get(c.id) || []).length, "message")} in ten minutes`;
    const key = keyOf(c);
    return key ? describe(key) : placeless(c);
  };
  const endRow = (end) => {
    if (end[0] === "h") return current.hubs.has(end.slice(2)) ? hubRow(current.hubs.get(end.slice(2))) : "";
    const s = current.byAddress.get(end.slice(2));
    return s ? cellRow(s) : "";
  };
  // How much a tie's two ends said, as the node counts each end's own.
  const tieSum = (pair) => (Array.isArray(pair) ? pair.reduce((sum, n) => sum + (Number(n) || 0), 0) : 0);
  // A tie said in words: how much passed today and in the last hour, who
  // sent how much, and of what kinds, as the mesh names them.
  const tieWords = (t) => {
    const today = tieSum(t.today);
    const hour = tieSum(t.hour);
    const kinds = (Array.isArray(t.intents) ? t.intents : []).map(String);
    const each = Array.isArray(t.today) ? t.ends.map((end, i) => `${endName(`c:${end}`)} sent ${Number(t.today[i]) || 0}`).join(", ") : "";
    return (
      `<p class="counts mono">${esc(`${plural(today, "message")} today\u00a0\u00b7 ${hour} in the last hour`)}</p>` +
      `<p>${esc(`${each ? `${each}. ` : ""}${kinds.length ? `Of the kinds ${kinds.join(", ")}. ` : ""}Last at ${clock(t.at, false)}.`)}</p>`
    );
  };
  const talkPanel = (id) => {
    const ends = id.split("|");
    const said = talkSaid.get(id) || [];
    const tied = tieOf.get(id);
    const asked = said.filter(({ msg }) => SIGNAL.has(msg.intent)).length;
    const rows = [...said]
      .reverse()
      .map(({ msg, to }) => {
        const from = `c:${msg.from}`;
        return `<li><span class="when mono">${esc(clock(msg.at, false))}</span><span>${esc(`${msg.intent || "message"} from ${endName(from)} to ${endName(to)}`)}</span></li>`;
      });
    const from = upFrom({ kind: "talk", id });
    // The two ends are set apart, each on its own line, so two long
    // titles never read as one.
    return (
      backButton(from ? cut(endName(`c:${from.id}`), 32) : "Now") +
      `<h2 id="panel-title" class="ends"><span>${esc(endName(ends[0]))}</span><span class="and">and</span><span>${esc(endName(ends[1]))}</span></h2>` +
      (tied
        ? `<p class="carries">Talk on the mesh today, as the node ties the two: how much each said and of what kinds, never the words.</p>` + tieWords(tied)
        : `<p class="carries">Talk on the mesh in the last ten minutes: who said what kind of thing to whom, never the words.</p>` +
          `<p class="counts mono">${esc(`${plural(said.length, "message")}\u00a0\u00b7 ${asked} asked or found something`)}</p>`) +
      `<h3>Between Them</h3><ul class="rows">${ends.map(endRow).join("")}</ul>` +
      `<h3>What Passed</h3>` +
      (rows.length ? `<ul class="lately">${rows.join("")}</ul>` : `<p class="hint">Nothing has passed between them in the last ten minutes: the page lists ten minutes of what it heard, from the latest messages the node keeps.</p>`)
    );
  };
  // What has no place of its own on the map, named in words.
  const placeless = (c) => (c.kind === "talk" ? talkWords(c.id) : sharedWords(c.id));

  // A network is said as a network first: how many cells it joins, where
  // they run and what it branches into, and only then the work it carries,
  // so its drawer never reads as a project's page.
  const machinesOf = (cells) => {
    const names = [...new Set(cells.map((s) => s.machine).filter(Boolean))].sort(collate);
    return names.length ? (names.length === 1 ? names[0] : `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`) : "no machine";
  };
  const networkWords = (h) => {
    const all = allCells(h);
    const more = all.length - h.cells.length;
    const branches = h.children.length ? `, with ${more ? `${NUMBER.format(more)} more` : "none"} in ${plural(h.children.length, "branch", "branches")}` : "";
    if (h.alone) return `${plural(all.length, "cell")} working alone on ${machinesOf(all)}, in no network.`;
    if (h.edge) return `${plural(all.length, "receptor")} at the network's edge, on ${machinesOf(all)}: cells that hear the outside and work in no network.`;
    // Fed by, never on: a network belongs to no machine, and its drawer
    // lists every one that feeds it.
    const feeders = h.feeds?.length || 0;
    const fed = feeders > 1 ? `, fed by ${plural(feeders, "machine")}` : feeders ? `, fed by ${h.feeds[0][0]}` : "";
    return `A network of ${plural(h.cells.length, "cell")}${branches}${fed}.`;
  };
  const networkPurpose = (h) => {
    if (h.alone) return "Each its own: these cells work alone, and each one's drawer says what it is for.";
    const work = (h.carries || "").replace(/[.\s]+$/, "");
    if (!work) return NO_PURPOSE;
    return `${work[0].toUpperCase()}${work.slice(1)}.`;
  };
  // A network's health by level: its own cells, each branch, the whole.
  const networkHealth = (h) => {
    if (!h.children.length) return `<ul class="rows health">${healthRow(`Its ${plural(h.cells.length, kindWord(h.cells))}`, h.cells)}</ul>`;
    const all = allCells(h);
    return (
      `<ul class="rows health">${healthRow(`Its own ${plural(h.cells.length, "cell")}`, h.cells)}` +
      h.children.map((c) => healthRow(`${c.name}, ${plural(allCells(c).length, "cell")}`, allCells(c), `net=${c.key}`)).join("") +
      `${healthRow(`All ${NUMBER.format(all.length)} together`, all)}</ul>`
    );
  };
  // A cell's machine, and what kind of machine it is.
  const TALKS_SHOWN = 8;
  const machineRow = (x) =>
    `<li><button type="button" class="row" data-go="machine=${esc(x.name)}">${MACHINE_GLYPH}<span class="what"><span class="name mono">${esc(x.name)}</span><span class="fact">${esc(x.members ? `a basin of ${plural(x.members.length, "machine")}` : isSandbox(x) ? sandboxWords(x) : x.state || "")}</span></span></button></li>`;
  // A machine that feeds a network, and how many of its cells run there.
  const feederRow = ([x, n]) =>
    `<li><button type="button" class="row" data-go="machine=${esc(x)}">${MACHINE_GLYPH}<span class="what"><span class="name mono">${esc(x)}</span><span class="fact">${esc(`feeds it ${plural(n, "cell")}`)}</span></span></button></li>`;
  const netPanel = (key) => {
    const m = current;
    const h = m.hubs.get(key);
    if (h) {
      const up = h.within ? m.hubs.get(h.within) : null;
      return (
        backButton(up ? up.name : "Now") +
        `<h2 id="panel-title" class="with-glyph">${glyph(h.shape, 12)}<span>${esc(h.name)}</span></h2>` +
        `<p class="carries">${esc(networkWords(h))}</p>` +
        glance(esc(networkPurpose(h)), esc(h.edge ? sensesDoing(h.cells) : groupDoing(allCells(h))), networkHealth(h)) +
        (h.lead ? `<p class="led">Led by <button type="button" class="inline" data-go="cell=${esc(h.lead.address)}">${esc(h.lead.short)}</button></p>` : "") +
        (h.alone || h.edge ? "" : launcherLink("network", h.key, "Open It in the Launcher")) +
        heldUp(h.cells) +
        talkLine(touching(h.cells, h.key), true) +
        spendBlock(allCells(h), [["branch", ownSpendRow(h).concat(hubSpendRows(h.children))]]) +
        (h.alone || h.within ? "" : `<h3>Its Machines</h3><ul class="rows">${h.feeds.map(feederRow).join("")}</ul>`) +
        (h.children.length ? `<h3>Its Branches</h3><ul class="rows">${h.children.map((c) => hubRow(c)).join("")}</ul>` : "") +
        `<h3>Its ${h.edge ? "Receptors" : "Cells"}</h3><ul class="rows">${[...h.cells].sort(worstFirst).map(cellRow).join("")}</ul>`
      );
    }
    const n = m.shared.find((x) => x.key === key);
    if (!n) return overview();
    return (
      backButton("Now") +
      `<h2 id="panel-title" class="mono">${esc(n.short)}</h2>` +
      `<p class="carries">${esc(`A network of ${plural(n.cells.length, "cell")} sharing the ${n.short} ${SHARED_WORD[n.shape]}, on ${machinesOf(n.cells)}; the dotted cross-hyphae on the map join them.`)}</p>` +
      glance(
        esc(n.title ? `${n.title.replace(/[.\s]+$/, "")}.` : `Work on the ${n.short} ${SHARED_WORD[n.shape]}, from whichever networks its cells belong to.`),
        esc(groupDoing(n.cells)),
        `<ul class="rows health">${healthRow(`Its ${plural(n.cells.length, "cell")}`, n.cells)}</ul>`,
      ) +
      launcherLink("network", n.key, "Open It in the Launcher") +
      spendBlock(n.cells) +
      `<h3>Its Cells</h3><ul class="rows">${[...n.cells].sort(worstFirst).map(cellRow).join("")}</ul>`
    );
  };

  // ---- Finding: a line that narrows the map as he types. ----

  // Every cell and network whose names hold all the words typed, networks
  // first, then cells with those that need him first: the order n steps.
  const matchesOf = (q) => {
    const m = current;
    const words = q.toLowerCase().split(/\s+/).filter(Boolean);
    if (!m || !words.length) return [];
    const hits = (text) => words.every((w) => text.toLowerCase().includes(w));
    const nets = [...m.hubs.values()].filter((h) => hits(`${h.name} ${h.carries} ${h.key}`)).map((h) => ({ key: `h:${h.key}`, go: { kind: "net", id: h.key } }));
    const shared = m.shared.filter((n) => hits(n.key)).map((n) => ({ key: null, go: { kind: "net", id: n.key } }));
    const cells = m.cells
      .filter((s) => hits(`${s.short} ${s.purpose || ""} ${s.address} ${s.repo || ""}`))
      .sort(worstFirst)
      .map((s) => ({ key: `c:${s.address}`, go: { kind: "cell", id: s.address } }));
    return [...nets, ...shared, ...cells];
  };
  const finding = () => !findLine.hidden && find.value.trim() !== "";
  // What matches keeps its full ink and the rest recedes, names included.
  const markFound = () => {
    const on = finding();
    world.classList.toggle("finding", on);
    labelsEl.classList.toggle("finding", on);
    paintGround();
    for (const g of world.querySelectorAll("[data-key]")) g.classList.toggle("match", on && foundSet.has(g.dataset.key));
  };
  const foundWords = () => {
    const q = find.value.trim();
    if (!q) return "";
    if (!found.length) return `Nothing is called \u201c${q}\u201d`;
    return foundAt >= 0 ? `${foundAt + 1} of ${found.length}` : plural(found.length, "match", "matches");
  };
  // A fresh read keeps the match the keys were taken to, where it still is.
  const refind = ({ keep = false } = {}) => {
    const was = keep && foundAt >= 0 ? found[foundAt] : null;
    found = matchesOf(find.value.trim());
    foundSet = new Set(found.map((f) => f.key).filter(Boolean));
    foundAt = was ? found.findIndex((f) => f.go.kind === was.go.kind && f.go.id === was.go.id) : -1;
    findCount.textContent = foundWords();
    if (!current) return;
    markFound();
    labelsFor();
    placeLabels();
  };
  // Return takes the keys to the first match, opened; n and N to the next
  // and the one before, round and round.
  const stepFound = (by) => {
    const to = stepped(foundAt, by, found.length);
    if (to === null) return;
    foundAt = to;
    const f = found[foundAt];
    visit(f.go, { replace: findVisited });
    findVisited = true;
    view.focus({ preventScroll: true });
    if (f.key) point(f.key, { follow: false });
    // The count is said by its own line, once; this says what was found.
    findCount.textContent = foundWords();
    where.textContent = `Found ${f.key ? describe(f.key) : sharedWords(f.go.id)}. ${levelWords(pointed)} n goes to the next.`;
  };
  const openFind = () => {
    findVisited = false;
    trail.hidden = true;
    findLine.hidden = false;
    find.focus();
    find.select();
  };
  const closeFind = () => {
    if (findLine.hidden) return;
    const inFind = findLine.contains(document.activeElement);
    find.value = "";
    findLine.hidden = true;
    trail.hidden = false;
    refind();
    if (inFind) view.focus({ preventScroll: true });
    if (view.contains(document.activeElement)) where.textContent = "Find closed; the whole map is back in its full ink.";
  };
  find.addEventListener("input", () => refind());
  find.addEventListener("keydown", (e) => {
    const asked = findKey(e.key);
    if (!asked) return;
    e.preventDefault();
    if (asked === "first") {
      foundAt = -1;
      stepFound(1);
    } else closeFind();
  });

  // ---- The heading line: where the keys are, as a trail back up. ----

  // A trail step's name, cut at a whole word and marked as cut with three
  // full stops; a narrow view leaves less room, so nothing is clipped.
  const NARROW = matchMedia("(max-width: 30rem)");
  const trailCut = (text, room) => {
    const r = NARROW.matches ? Math.round(room * 0.6) : room;
    const kept = cut(text, r);
    return kept.length < text.trim().length ? `${kept.replace(/[\s.]+$/, "")}...` : kept;
  };

  const trailWords = (key) => {
    const id = key.slice(2);
    if (key[0] === "c") {
      const s = current.byAddress.get(id);
      return `${s ? stateGlyph(s) : ""}<span>${esc(trailCut(s?.short || id, 32))}</span>`;
    }
    if (key[0] === "h") {
      const h = current.hubs.get(id);
      return `${h ? glyph(h.shape, 10) : ""}<span>${esc(trailCut(h?.name || id, 32))}</span>`;
    }
    return `${MACHINE_GLYPH}<span class="mono">${esc(id)}</span>`;
  };
  // The trail says where the view is, and the ring where the keys are:
  // what's open, else the network whose cells the keys walk, else the
  // whole map. Redrawn only when it changes, so a step he's on keeps its
  // focus.
  const viewKey = () => {
    const open = keyOf(chosen);
    if (open && spots.has(open)) return open;
    if (!chosen && pointed?.[0] === "c") return levelOf(pointed).hub ? `h:${levelOf(pointed).hub.key}` : null;
    return null;
  };
  const drawTrail = () => {
    if (!current) return;
    const chain = [];
    // A network belongs to no machine, so its trail starts at Networks; a
    // machine's, and the cells working alone on it, at Machines.
    const trailUp = (key) => (key[0] === "h" && !current.hubs.get(key.slice(2))?.alone && !current.hubs.get(key.slice(2))?.within ? null : treeUp(key));
    for (let key = viewKey(); key && current && (key[0] !== "h" || current.hubs.has(key.slice(2))); key = trailUp(key)) chain.unshift(key);
    const step = (key, words, here) => `<li><button type="button" class="step" data-trail="${esc(key)}"${here ? ' aria-current="location"' : ""}>${words}</button></li>`;
    // A shared network has no place of its own on the map, so it is named
    // at the trail's end without a step to take.
    // A talk names both its ends, each cut on its own, so neither is lost.
    const words = chosen?.kind === "talk" ? `talk between ${chosen.id.split("|").map((end) => trailCut(endName(end), 24)).join(" and ")}` : chosen && trailCut(placeless(chosen), 32);
    const shared = chosen && !keyOf(chosen) ? `<li><span class="step" aria-current="location"><span>${esc(words)}</span></span></li>` : "";
    const root = chain[0]?.[0] === "h" ? "Networks" : chain[0]?.[0] === "m" ? "Machines" : "The whole map";
    const html = [step("", `<span>${root}</span>`, !chain.length && !shared), ...chain.map((key, i) => step(key, trailWords(key), i === chain.length - 1)), shared].join("");
    if (html === trailShown) return;
    const had = trail.contains(document.activeElement) ? document.activeElement.dataset.trail : null;
    trailShown = html;
    trailSteps.innerHTML = html;
    if (had == null) return;
    const again = trailSteps.querySelector(`[data-trail="${CSS.escape(had)}"]`);
    (again || view).focus({ preventScroll: true });
  };
  trail.addEventListener("click", (e) => {
    const b = e.target.closest("[data-trail]");
    if (!b) return;
    const key = b.dataset.trail;
    if (!key) {
      if (chosen) letGo();
      fit();
      view.focus({ preventScroll: true });
      point(firstKey(), { follow: false, say: true });
      return;
    }
    visit(choiceOf(key));
    view.focus({ preventScroll: true });
    point(key, { follow: false, say: true });
  });

  // The drawer is open while something is chosen, or while Now is asked
  // for; otherwise the map's gutter is empty and quiet.
  let nowOpen = false;
  const nowControl = document.getElementById("now-control");
  const drawer = () => {
    const open = !!chosen || nowOpen;
    treeBox.classList.toggle("open", open);
    nowControl.setAttribute("aria-expanded", String(!chosen && nowOpen));
    // Focus never stays in a drawer going out of reach: it goes to the map,
    // on the mark the keys were on.
    if (!open && treeBox.contains(document.activeElement)) view.focus({ preventScroll: true });
  };
  // Now opens over whatever is chosen, letting it go, and keeps the mark
  // the keys are on clear of the drawer.
  const openNow = () => {
    nowOpen = true;
    if (chosen) {
      history.replaceState(null, "", location.pathname);
      apply(false);
    } else drawer();
    if (pointed) keepInView(pointed);
    where.textContent = "Now is open in the drawer.";
  };
  const closeNow = () => {
    nowOpen = false;
    drawer();
    where.textContent = "Now is closed.";
  };
  const toggleNow = () => (nowOpen && !chosen ? closeNow() : openNow());

  let shownKey = "";
  const panel = () => timed("mycelium-panel", drawPanel);
  // Where the column stands, kept as it scrolls, so a new panel need not
  // ask: a read or write of it straight after the map is redrawn lays the
  // whole page out before the first frame.
  let columnTop = 0;
  panelBody.parentElement.addEventListener("scroll", (e) => (columnTop = e.currentTarget.scrollTop), { passive: true });
  // A new panel's tree waits for the frame that shows the panel: it sits
  // below it, and at ten thousand cells it is a frame of work of its own.
  let treeSoon = 0;
  const afterPaint = (fn) => {
    const ticket = ++treeSoon;
    requestAnimationFrame(() => setTimeout(() => ticket === treeSoon && fn(), 0));
  };
  const drawPanel = () => {
    if (!current) return;
    const html = !chosen
      ? overview()
      : chosen.kind === "cell"
        ? cellPanel(current.byAddress.get(chosen.id))
        : chosen.kind === "machine"
          ? machinePanel(chosen.id)
          : chosen.kind === "talk"
            ? talkPanel(chosen.id)
            : netPanel(chosen.id);
    const key = hashOf(chosen);
    // A redraw of the same panel keeps his place in it: the same row keeps
    // focus and the list keeps its scroll.
    const held = document.activeElement?.closest?.("#panel-body [data-go]")?.dataset.go;
    // What he opened stays open, the legend everywhere and the rest within
    // the same panel, and a summary he was on keeps the keys.
    const opened = [...panelBody.querySelectorAll("details[data-keep][open]")].map((d) => d.dataset.keep);
    const onSummary = document.activeElement?.closest?.("#panel-body details[data-keep] > summary")?.parentElement.dataset.keep;
    // The panel scrolls as one column, with the tree in it, so its place
    // is the column's. It is read only for a redraw of the same panel and
    // written once at the end, since each read after the map has changed
    // lays the whole page out again.
    const column = panelBody.parentElement;
    const same = key === shownKey;
    const scroll = same ? column.scrollTop : 0;
    const hadFocus = panelBody.contains(document.activeElement);
    panelBody.innerHTML = html;
    drawer();
    for (const d of panelBody.querySelectorAll("details[data-keep]")) if (opened.includes(d.dataset.keep) && (same || d.dataset.keep === "legend")) d.open = true;
    shownKey = key;
    // A cell is looked at alone: the whole network as a tree under it
    // draws the eye away, so its drawer says only where it sits.
    const alone = chosen?.kind === "cell";
    if (alone && tree.contains(document.activeElement)) view.focus({ preventScroll: true });
    tree.parentElement.hidden = alone;
    if (same) {
      treeSoon++;
      if (!alone) renderTree({ keepScroll: false });
      column.scrollTop = scroll;
    } else {
      if (alone) treeSoon++;
      else afterPaint(() => renderTree({ keepScroll: false }));
      if (columnTop) column.scrollTop = columnTop = 0;
    }
    if (same && held) panelBody.querySelector(`[data-go="${CSS.escape(held)}"]`)?.focus({ preventScroll: true });
    if (same && onSummary) panelBody.querySelector(`details[data-keep="${onSummary}"] > summary`)?.focus({ preventScroll: true });
    // Focus is never dropped with the panel it was in: it goes to the map,
    // whose pointer is still on the thing just left.
    if (hadFocus && !panelBody.contains(document.activeElement)) view.focus({ preventScroll: true });
  };

  panelBody.addEventListener("click", (e) => {
    const go = e.target.closest("[data-go]");
    if (go) {
      const [kind, id] = go.dataset.go.split(/=(.*)/s);
      // A talk opened from a cell's details steps back to that cell.
      talkFrom = kind === "talk" && chosen?.kind === "cell" ? `c:${chosen.id}` : null;
      visit({ kind, id });
      where.textContent = `Opened ${openedWords({ kind, id })}; its details are in the drawer.`;
      requestAnimationFrame(() => panelBody.querySelector("h2")?.setAttribute("tabindex", "-1"));
      requestAnimationFrame(() => panelBody.querySelector("[data-back]")?.focus({ preventScroll: true }));
      return;
    }
    // The way back to the whole map is the way to Now, as it says.
    if (e.target.closest("[data-back]")) {
      if (chosen && !upFrom(chosen)) nowOpen = true;
      back();
    }
  });
  // Pointing at a row names its cell on the map.
  panelBody.addEventListener("pointerover", (e) => {
    const go = e.target.closest('[data-go^="cell="]');
    const id = go ? current?.byAddress.get(go.dataset.go.slice(5))?.id : null;
    if (id !== hover) {
      hover = id;
      labelsFor();
      placeLabels();
      paintGround();
    }
  });

  // ---- The network as a tree, for keys and screen readers. ----

  // Machines hold the networks their cords carry, networks hold their
  // branches and then their cells, worst first. Only open branches are
  // drawn, so the tree stays small however large the network grows. The
  // item with focus is where the map's pointer is, and the other way round.
  const expanded = new Set();
  let treeActive = null;
  let treeSeeded = false;
  const byName = (a, b) => collate(a.name, b.name);
  // Each branch's children are sorted once for each reading of the data,
  // and only once it opens: a closed branch needs only to know it has any.
  const kidsCache = new Map();
  let kidsOf = null;
  const treeKids = (key) => {
    const m = current;
    if (kidsOf !== m) {
      kidsCache.clear();
      kidsOf = m;
    }
    if (kidsCache.has(key)) return kidsCache.get(key);
    const id = key.slice(2);
    let kids = [];
    if (key[0] === "m") kids = [...m.hubs.values()].filter((h) => !h.within && h.machine === id).sort(byName).map((h) => `h:${h.key}`);
    else if (key[0] === "h" && m.hubs.has(id)) {
      const h = m.hubs.get(id);
      kids = [...[...h.children].sort(byName).map((c) => `h:${c.key}`), ...[...h.cells].sort(worstFirst).map((s) => `c:${s.address}`)];
    }
    kidsCache.set(key, kids);
    return kids;
  };
  const hasKids = (key) => {
    if (kidsCache.has(key) && kidsOf === current) return kidsCache.get(key).length > 0;
    if (key[0] === "h") {
      const h = current.hubs.get(key.slice(2));
      return !!h && h.children.length + h.cells.length > 0;
    }
    return key[0] === "m" && treeKids(key).length > 0;
  };
  const treeUp = (key) => {
    const m = current;
    const id = key.slice(2);
    if (key[0] === "h") {
      const h = m.hubs.get(id);
      return h ? (h.within ? `h:${h.within}` : h.machine && machinesShown ? `m:${h.machine}` : null) : null;
    }
    if (key[0] === "c") {
      const s = m.byAddress.get(id);
      return s ? `h:${(s.hub || s.loose).key}` : null;
    }
    return null;
  };
  const openTo = (key) => {
    for (let up = treeUp(key); up; up = treeUp(up)) expanded.add(up);
  };
  // A machine's mark in small: its box, a cord leaving each side.
  const MACHINE_GLYPH = `<svg class="glyph" viewBox="-12 -12 24 24" aria-hidden="true"><path class="m-cord" d="M-12,0H-7M7,0H12" style="--w:1.5px"/><rect class="m-box" x="-7" y="-5" width="14" height="10"/></svg>`;
  const RECEPTOR_GLYPH = (hearing) =>
    `<svg class="glyph" viewBox="-12 -12 24 24" aria-hidden="true"><path class="m-receptor" d="M-9,-5.4Q0,14.4 9,-5.4"/>${hearing ? '<circle class="m-nucleus" r="4.5"/>' : '<circle class="m-quiet" r="4"/>'}</svg>`;
  const TWISTY = `<svg class="twisty" viewBox="0 0 10 10" aria-hidden="true"><path d="M3 1.5L7 5L3 8.5"/></svg>`;
  const treeRow = perReading((key) => {
    const m = current;
    const id = key.slice(2);
    if (key[0] === "c") {
      const s = m.byAddress.get(id);
      return [stateGlyph(s), s.short, cellWords(s), ""];
    }
    if (key[0] === "h") {
      const h = m.hubs.get(id);
      return [glyph(h.shape, 10), h.name, hubWords(h), ""];
    }
    return [MACHINE_GLYPH, id, machineWords(id), " mono"];
  });
  const renderTree = (how) => timed("mycelium-tree", () => drawTree(how));
  // Drawn alone, the tree keeps the column's place itself; drawn with the
  // panel, the panel keeps it.
  const drawTree = ({ keepScroll = true } = {}) => {
    if (!current) return;
    if (!treeSeeded) {
      for (const x of current.machines) expanded.add(`m:${x.name}`);
      treeSeeded = true;
    }
    const held = document.activeElement?.closest?.("#tree [role=treeitem]")?.dataset.key;
    const scroll = keepScroll ? treeBox.scrollTop : 0;
    const opened = keyOf(chosen);
    const item = (key, level, i, n) => {
      const any = hasKids(key);
      const open = any && expanded.has(key);
      const kids = open ? treeKids(key) : [];
      const [mark, name, fact, mono] = treeRow(key);
      return (
        `<li role="treeitem" data-key="${esc(key)}" aria-level="${level}" aria-setsize="${n}" aria-posinset="${i + 1}"` +
        `${any ? ` aria-expanded="${open}"` : ""}${key === opened ? ' aria-current="true"' : ""} aria-label="${esc(describe(key))}" tabindex="-1">` +
        `<div class="ti${key === opened ? " opened" : ""}" style="--level:${level - 1}" aria-hidden="true">${any ? TWISTY : '<span class="twisty"></span>'}${mark}` +
        `<span class="what"><span class="name${mono}">${esc(name)}</span><span class="fact">${esc(fact)}</span></span></div>` +
        (open ? `<ul role="group">${kids.map((c, j) => item(c, level + 1, j, kids.length)).join("")}</ul>` : "") +
        `</li>`
      );
    };
    const roots = shownMachines().length ? shownMachines().map((x) => `m:${x.name}`) : [...current.hubs.values()].filter((h) => !h.within).map((h) => `h:${h.key}`);
    tree.innerHTML = roots.map((key, i) => item(key, 1, i, roots.length)).join("");
    const find1 = (key) => key && tree.querySelector(`[data-key="${CSS.escape(key)}"]`);
    const active = find1(held) || find1(treeActive) || find1(pointed) || tree.querySelector("[role=treeitem]");
    if (active) {
      active.tabIndex = 0;
      treeActive = active.dataset.key;
    }
    if (keepScroll) treeBox.scrollTop = scroll;
    if (held && active) active.focus({ preventScroll: true });
  };
  const treeItems = () => [...tree.querySelectorAll("[role=treeitem]")];
  const focusItem = (li) => {
    if (!li) return;
    for (const x of tree.querySelectorAll('[tabindex="0"]')) x.tabIndex = -1;
    li.tabIndex = 0;
    treeActive = li.dataset.key;
    li.focus({ preventScroll: true });
    li.firstElementChild.scrollIntoView({ block: "nearest" });
  };
  // What each of the tree's keys does to the item it's on, once keys.ts
  // has said which.
  const TREE_DOES = {
    next: ({ list, at }) => focusItem(list[at + 1]),
    prev: ({ list, at }) => focusItem(list[at - 1]),
    first: ({ list }) => focusItem(list[0]),
    last: ({ list }) => focusItem(list[list.length - 1]),
    in: ({ li, key, open, shut }) => {
      if (shut) {
        expanded.add(key);
        renderTree();
      } else if (open) focusItem(li.querySelector("[role=treeitem]"));
    },
    out: ({ li, key, open }) => {
      if (open) {
        expanded.delete(key);
        renderTree();
      } else focusItem(li.parentElement.closest("[role=treeitem]"));
    },
    open: ({ key }) => visit(choiceOf(key)),
  };
  tree.addEventListener("keydown", (e) => {
    const li = e.target.closest("[role=treeitem]");
    const act = treeKey(e);
    if (!li || !act) return;
    e.preventDefault();
    const list = treeItems();
    TREE_DOES[act]({
      li,
      key: li.dataset.key,
      list,
      at: list.indexOf(li),
      open: li.getAttribute("aria-expanded") === "true",
      shut: li.getAttribute("aria-expanded") === "false",
    });
  });
  // Focus in the tree moves the map's pointer to the same thing.
  tree.addEventListener("focusin", (e) => {
    const key = e.target.closest?.("[role=treeitem]")?.dataset.key;
    if (!key || key === pointed || !spots.has(key)) return;
    pointed = key;
    treeActive = key;
    placePointer();
    labelsFor();
    placeLabels();
    keepInView(key);
  });
  tree.addEventListener("click", (e) => {
    const li = e.target.closest("[role=treeitem]");
    if (!li) return;
    const key = li.dataset.key;
    focusItem(li);
    if (e.target.closest(".twisty") && li.hasAttribute("aria-expanded")) {
      if (expanded.has(key)) expanded.delete(key);
      else expanded.add(key);
      renderTree();
      return;
    }
    visit(choiceOf(key));
  });

  // ---- Messages: one dot for each message, and arcs for who has talked. ----

  // A message on the mesh goes straight from the cell that sent it to
  // whoever it was for, and passes through no one on the way. So it is
  // drawn that way: one dot on one arc from the sender to the receiver,
  // bowed off the hyphae so it never seems to run along them, the
  // sender's ring opening as it leaves and the receiver's as it lands.
  // Each is replayed at the pace it was sent, one reading late, and the
  // arc between two that talked stays drawn for ten minutes, thicker the
  // more they said. The node keeps no words, and the map shows none: only
  // who sent it and where it went.
  const SPEED = 520; // world units a second
  const FLIGHT_MIN_MS = 700;
  const FLIGHT_MAX_MS = 2600;
  const LANDING_MS = 420;
  const PULSE_PX = 10;
  const RIPPLE_PX = 16;
  const TRAFFIC_MS = 10 * 60_000;
  const REPLAY_MS = 2 * EVERY_MS; // older than this, a message only warms
  const MAX_FANOUT = 6; // receivers drawn for a message to a shared network
  const MAX_FLIGHTS = 48;
  const WARM_AT = [1, 3, 8]; // messages over a link for each step warmer
  // A tie's strength is a message's worth for each one sent, halving every
  // two hours, so one message just now is 1, and one four hours ago a
  // quarter: below that a tie is too faint to draw.
  const TIE_WARM_AT = [0.25, 2, 6];
  const MAX_TIES = 400; // the strongest drawn, so a busy day stays a map
  const WARMS = true;
  const PULSES = true;
  const ARC_BOW = 0.24; // how far an arc bows off the straight, over its length
  const ARC_GAP = 3; // between an arc's end and the ring it leaves or reaches

  const pairKey = (a, b) => (a < b ? `${a}|${b}` : `${b}|${a}`);

  // The arc between two marks, leaving the sender's ring and ending at the
  // receiver's. Both ways round share one curve, so a reply retraces it;
  // a bow of the other sign curves to the other side of the same two.
  const arcOf = (from, to, bow = ARC_BOW) => {
    const [lo, hi] = from < to ? [from, to] : [to, from];
    const a = spots.get(lo);
    const b = spots.get(hi);
    if (!a || !b) return null;
    const c = control(a, b, bow);
    const edge = (at, key) => {
      const vx = c[0] - at[0];
      const vy = c[1] - at[1];
      const d = Math.hypot(vx, vy) || 1;
      const r = (sizes.get(key) || R) + ARC_GAP;
      return [at[0] + (vx / d) * r, at[1] + (vy / d) * r];
    };
    const [p, q] = [edge(a, lo), edge(b, hi)];
    const length = (Math.hypot(c[0] - p[0], c[1] - p[1]) + Math.hypot(q[0] - c[0], q[1] - c[1]) + Math.hypot(q[0] - p[0], q[1] - p[1])) / 2;
    return lo === from ? { p, c, q, length } : { p: q, c, q: p, length };
  };

  // Where a message goes: a peer's cell, an orchestration's hub, or the
  // cells of a shared network, a few of them.
  const receiversOf = (msg) => {
    const from = `c:${msg.from}`;
    const out = new Set();
    for (const g of msg.groups || []) {
      if (g.startsWith("peer.")) out.add(`c:${g.slice(5).toUpperCase()}`);
      else if (current.hubs.has(g)) out.add(`h:${g}`);
      else {
        const n = current.shared.find((x) => x.key === g);
        for (const s of (n?.cells || []).slice(0, MAX_FANOUT + 1)) out.add(`c:${s.address}`);
      }
    }
    out.delete(from);
    return [...out].filter((k2) => spots.has(k2)).slice(0, MAX_FANOUT);
  };

  const heard = new Map(); // a message's key: when it was sent, in ms
  // A pair's key: how much they talked, as a strength where the node ties
  // its cells, and otherwise as the page's count of ten minutes' messages.
  let talk = new Map();
  let tieOf = new Map(); // a pair's key: the node's tie between them, where it ties
  let talkSaid = new Map(); // a pair's key: those messages, oldest first
  const msgKey = (msg) => `${msg.at}|${msg.from}|${(msg.groups || []).join(",")}`;

  // What the node said since the last reading: kept for ten minutes, and
  // what's new is sent on its way.
  const hear = (fresh) => {
    const now = Date.parse(current.at);
    const arrived = [];
    for (const msg of current.messages) {
      const key = msgKey(msg);
      const at = Date.parse(msg.at);
      if (heard.has(key) || !(now - at < TRAFFIC_MS)) continue;
      heard.set(key, { at, msg });
      if (fresh && now - at < REPLAY_MS) arrived.push({ at, msg });
    }
    for (const [key, h] of heard) if (now - h.at >= TRAFFIC_MS) heard.delete(key);
    talk = new Map();
    talkSaid = new Map();
    for (const { msg } of [...heard.values()].sort((a, b) => a.at - b.at)) {
      for (const to of receiversOf(msg)) {
        const e = pairKey(`c:${msg.from}`, to);
        talk.set(e, (talk.get(e) || 0) + 1);
        if (!talkSaid.has(e)) talkSaid.set(e, []);
        talkSaid.get(e).push({ msg, to });
      }
    }
    // The node's ties, where it serves them, say a day's talk between
    // cells in place of the page's ten minutes, so two that talked this
    // morning are still joined, fainter as they fall quiet.
    tieOf = new Map();
    if (current.ties) {
      talk = new Map();
      for (const t of current.ties) {
        if (tieOf.size >= MAX_TIES || t.strength < TIE_WARM_AT[0]) break;
        const [a, b] = t.ends.map((end) => `c:${end}`);
        if (!current.byAddress.has(a.slice(2)) || !current.byAddress.has(b.slice(2))) continue;
        const e = pairKey(a, b);
        tieOf.set(e, t);
        talk.set(e, t.strength);
      }
    }
    paintTalk();
    for (const { at, msg } of arrived.sort((a, b) => a.at - b.at)) {
      const delay = Math.min(Math.max(at - (now - EVERY_MS), 0), EVERY_MS);
      later(() => send(msg), delay);
    }
  };

  const warmLevel = (n) => (current.ties ? TIE_WARM_AT : WARM_AT).filter((t) => n >= t).length;
  // A redraw that frames the map straight after builds the layer and
  // leaves the painting to its frame, which would only paint it again.
  const paintTalk = ({ paint = true } = {}) => {
    if (!WARMS) return;
    talkShapes = [];
    talkLayer = [];
    // All talk is drawn, as thick as it is warm, but faint until the choice
    // touches it, so the cells read first. A cell open lifts its own talk;
    // a network lifts the talk within it, since talk with one of its cells
    // reaches most of the map.
    const own = chosen?.kind === "cell" ? `c:${chosen.id}` : null;
    const touches = (end) => (end[0] === "h" ? near.hubs.has(end.slice(2)) : near.ids.has(current.byAddress.get(end.slice(2))?.id));
    // A cell folded into its network talks through the network's hub, so
    // talk is never lost to a fold: the talk between two folded networks
    // is one line, as warm as the warmest pair it stands for, and talk
    // inside one folded network waits in its ground.
    const endOf = (end) => {
      if (spots.has(end)) return end;
      const s = end[0] === "c" && current.byAddress.get(end.slice(2));
      return s && isFolded(s) ? `h:${homeOf(s)}` : null;
    };
    const drawn = new Map();
    for (const [e, n] of talk) {
      const [a, b] = e.split("|").map(endOf);
      if (!a || !b || a === b) continue;
      const key = a === e.split("|")[0] && b === e.split("|")[1] ? e : pairKey(a, b);
      const was = drawn.get(key);
      if (!was || was.n < n) drawn.set(key, { e, n, a, b });
    }
    talkBetween = drawn.size;
    for (const [key, { e, n, a, b }] of drawn) {
      const faint = chosen?.kind === "talk" ? "" : !near || !(own ? key.split("|").includes(own) : key.split("|").every(touches)) ? " faint" : "";
      const arc = arcOf(a, b);
      if (!arc) continue;
      const line = opens([arc.p, arc.c, arc.q], "talk", e);
      talkShapes.push(line);
      // An opened talk is drawn alone; the rest step back with the map.
      const open = chosen?.kind === "talk" ? (chosen.id === e ? " lit" : " far") : "";
      talkLayer.push({ tag: "path", cls: `m-talk warm${warmLevel(n)}${open}${faint}`, line });
    }
    // A receptor's feeding bows off the hyphae as talk does, so it never
    // lies over the marks between, but dot-and-dash and to the other side, so
    // the two read apart even between the same two. It is drawn whole
    // until a choice that doesn't touch it, being rare and gone in ten
    // minutes, and opens the cell it fed.
    fedShapes = [];
    fedLayer = [];
    for (const f of current.fed || []) {
      const [a, b] = [endOf(`c:${f.receptor}`), endOf(`c:${f.cell}`)];
      const arc = a && b && a !== b ? arcOf(a, b, -ARC_BOW) : null;
      if (!arc) continue;
      const line = [arc.p, arc.c, arc.q];
      fedShapes.push(opens(line, "cell", f.cell));
      const ends = [`c:${f.receptor}`, `c:${f.cell}`];
      const faint = near && !(own ? ends.includes(own) : ends.every(touches)) ? " faint" : "";
      fedLayer.push({ tag: "path", cls: `m-fed${faint}`, line });
    }
    if (!paint) return;
    // The names keep off talk too, so they move when it does.
    placeLabels();
    paintGround();
  };

  // Pulses are timed, and the frames run only while one is in flight.
  const waiting = new Set();
  const later = (fn, ms) => {
    const t = setTimeout(() => {
      waiting.delete(t);
      fn();
    }, ms);
    waiting.add(t);
  };
  const flights = [];
  let flying = 0;
  const send = (msg) => {
    if (!PULSES || still.matches || document.hidden || flights.length >= MAX_FLIGHTS) return;
    const from = `c:${msg.from}`;
    for (const to of receiversOf(msg)) {
      const arc = arcOf(from, to);
      if (!arc?.length) continue;
      const ms = Math.min(Math.max((arc.length / SPEED) * 1000, FLIGHT_MIN_MS), FLIGHT_MAX_MS);
      flights.push({ arc, ms, from, to, start: performance.now() });
    }
    if (flights.length && !flying) flying = requestAnimationFrame(fly);
  };
  const along = ({ p, c, q }, t) => {
    const u = 1 - t;
    return [u * u * p[0] + 2 * u * t * c[0] + t * t * q[0], u * u * p[1] + 2 * u * t * c[1] + t * t * q[1]];
  };
  // What flies is painted on a canvas of its own above the ground, so a
  // frame costs the flights alone however many cells lie under them, and
  // a page with nothing in flight draws nothing at all.
  const airEl = document.getElementById("map-air");
  const air = airEl.getContext("2d");
  const clearAir = () => {
    const [w, h] = size();
    const dpr = window.devicePixelRatio || 1;
    const [pw, ph] = [Math.round(w * dpr), Math.round(h * dpr)];
    if (airEl.width !== pw || airEl.height !== ph) {
      airEl.width = pw;
      airEl.height = ph;
    }
    air.setTransform(dpr, 0, 0, dpr, 0, 0);
    air.clearRect(0, 0, w, h);
  };
  const X = (x) => x * k + tx;
  const Y = (y) => y * k + ty;
  // A ring opening round a mark, `l` of the way from its edge outward.
  const ring = (key, l) => {
    const at = spots.get(key);
    if (!at) return;
    const ink = inkOf("circle", "ripple");
    air.globalAlpha = (1 - l) * ink.alpha;
    air.strokeStyle = ink.stroke;
    air.lineWidth = ink.width;
    air.beginPath();
    air.arc(X(at[0]), Y(at[1]), (sizes.get(key) || R) * k + l * RIPPLE_PX, 0, 2 * Math.PI);
    air.stroke();
  };
  const fly = () => {
    flying = 0;
    const now = performance.now();
    clearAir();
    const trailInk = inkOf("path", "trail");
    const dotInk = inkOf("circle", "pulse");
    for (let i = flights.length - 1; i >= 0; i--) {
      const f = flights[i];
      const t = Math.max(0, (now - f.start) / f.ms);
      const landing = LANDING_MS / f.ms;
      if (t >= 1 + landing) {
        flights.splice(i, 1);
        continue;
      }
      // Eased in and out over the whole way, so it leaves and arrives.
      const e = t >= 1 ? 1 : t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
      const { p, c } = f.arc;
      const [x, y] = along(f.arc, e);
      // The way it has come draws itself behind it, so who spoke to whom
      // stays in view after the bead has gone by: the arc's first `e`,
      // which is a curve of its own with its control `e` along p to c.
      air.globalAlpha = (t >= 1 ? 1 - Math.min((t - 1) / landing, 1) : 1) * trailInk.alpha;
      air.strokeStyle = trailInk.stroke;
      air.lineWidth = trailInk.width;
      air.lineCap = trailInk.cap || "round";
      air.beginPath();
      air.moveTo(X(p[0]), Y(p[1]));
      air.quadraticCurveTo(X(p[0] + (c[0] - p[0]) * e), Y(p[1] + (c[1] - p[1]) * e), X(x), Y(y));
      air.stroke();
      // Leaving: a ring opens round the sender as the dot sets off.
      const out = (now - f.start) / LANDING_MS;
      if (out < 1) ring(f.from, Math.max(0, out));
      if (t < 1) {
        air.globalAlpha = dotInk.alpha;
        air.beginPath();
        air.arc(X(x), Y(y), PULSE_PX, 0, 2 * Math.PI);
        air.fillStyle = dotInk.fill;
        air.fill();
        if (dotInk.stroke) {
          air.strokeStyle = dotInk.stroke;
          air.lineWidth = dotInk.width;
          air.stroke();
        }
      } else {
        // Landing: a ring opens round the receiver and fades.
        ring(f.to, Math.min((t - 1) / landing, 1));
      }
    }
    lifeFrame(now);
    air.globalAlpha = 1;
    if (flights.length || lives.length) flying = requestAnimationFrame(fly);
    else clearAir();
  };

  // ---- Birth and death. ----
  // A cell the page hadn't heard of grows out of its hub along a hypha,
  // and a ring opens round it as it arrives; one that has gone leaves a
  // ghost of its membrane that shrinks back toward where it hung from.
  // Both are said in words in Now for ten minutes, so nothing rests on
  // seeing them move, and nothing moves at all for who asks for less.
  const LIFE_KEEP_MS = 10 * 60 * 1000;
  const BIRTH_MS = 900;
  const DEATH_MS = 1200;
  // However far out the map is, a ghost starts big enough to be seen go.
  const GHOST_MIN_PX = 10;
  const MAX_LIVES = 24;
  // How far a ghost drifts toward its hub before it is gone.
  const GHOST_DRIFT = 0.6;
  const born = new Map(); // id: when the page learnt of it
  const ended = new Map(); // id: { short, at }
  const lives = [];
  let heardCells = null; // the last reading's cells: id to { address, short, hub, machine }
  const heardOf = (m) => new Map(m.cells.map((s) => [s.id, { address: s.address, short: s.short, hub: s.hub?.key, machine: s.machine }]));
  const homeKey = (hub, machine) => (hub ? `h:${hub}` : `m:${machine}`);
  // The first reading has nothing to compare with, so a cell counts as
  // born then only if it says it started within the last ten minutes.
  const firstLives = () => {
    const now = Date.now();
    for (const s of current.cells) {
      const t = Date.parse(s.since || "");
      if (now - t < LIFE_KEEP_MS) born.set(s.id, t);
    }
    heardCells = heardOf(current);
  };
  // Between two readings: who is new and who has gone. `was` is where
  // everything stood before this reading drew, for the ghosts.
  const noteLives = (was) => {
    const now = Date.now();
    const fresh = [];
    const gone = [];
    for (const s of current.cells) {
      if (heardCells.has(s.id)) continue;
      born.set(s.id, now);
      ended.delete(s.id);
      fresh.push(s);
    }
    for (const [id, c] of heardCells) {
      if (current.byId.has(id)) continue;
      ended.set(id, { short: c.short, at: now });
      born.delete(id);
      gone.push(c);
    }
    for (const [id, t] of born) if (now - t > LIFE_KEEP_MS || !current.byId.has(id)) born.delete(id);
    for (const [id, e] of ended) if (now - e.at > LIFE_KEEP_MS) ended.delete(id);
    heardCells = heardOf(current);
    if (!PULSES || still.matches || document.hidden) return;
    const start = performance.now();
    for (const s of fresh) {
      const key = `c:${s.address}`;
      const to = spots.get(key);
      const from = spots.get(homeKey(s.hub?.key, s.machine));
      if (to && from && lives.length < MAX_LIVES) lives.push({ birth: true, key, from, to, start });
    }
    for (const c of gone) {
      const at = was.get(`c:${c.address}`);
      if (at && lives.length < MAX_LIVES) lives.push({ birth: false, at, home: was.get(homeKey(c.hub, c.machine)) || at, start });
    }
    if (lives.length && !flying) flying = requestAnimationFrame(fly);
  };
  // One frame of whatever is being born or ending, over the flights.
  const lifeFrame = (now) => {
    if (!lives.length) return;
    const birthInk = inkOf("path", "birth");
    const ghostInk = inkOf("circle", "ghost");
    for (let i = lives.length - 1; i >= 0; i--) {
      const v = lives[i];
      if (v.birth) {
        const t = (now - v.start) / BIRTH_MS;
        const landing = LANDING_MS / BIRTH_MS;
        if (t >= 1 + landing) {
          lives.splice(i, 1);
          continue;
        }
        // Out from the hub's edge to the cell's, easing as it arrives.
        const [dx, dy] = [v.to[0] - v.from[0], v.to[1] - v.from[1]];
        const d = Math.hypot(dx, dy) || 1;
        const [ux, uy] = [dx / d, dy / d];
        const reach = Math.max(0, d - HUB_R - R);
        const e = 1 - (1 - Math.min(t, 1)) ** 3;
        const [sx, sy] = [v.from[0] + ux * HUB_R, v.from[1] + uy * HUB_R];
        air.globalAlpha = (t >= 1 ? 1 - Math.min((t - 1) / landing, 1) : 1) * birthInk.alpha;
        air.strokeStyle = birthInk.stroke;
        air.lineWidth = birthInk.width;
        air.lineCap = birthInk.cap || "round";
        air.beginPath();
        air.moveTo(X(sx), Y(sy));
        air.lineTo(X(sx + ux * reach * e), Y(sy + uy * reach * e));
        air.stroke();
        if (t >= 1) ring(v.key, Math.min((t - 1) / landing, 1));
      } else {
        const t = (now - v.start) / DEATH_MS;
        if (t >= 1) {
          lives.splice(i, 1);
          continue;
        }
        // Slow to leave, quicker as it goes.
        const e = t * t;
        const [x, y] = [v.at[0] + (v.home[0] - v.at[0]) * e * GHOST_DRIFT, v.at[1] + (v.home[1] - v.at[1]) * e * GHOST_DRIFT];
        air.globalAlpha = (1 - t) * ghostInk.alpha;
        air.strokeStyle = ghostInk.stroke;
        air.lineWidth = ghostInk.width;
        air.setLineDash(ghostInk.dash);
        air.beginPath();
        air.arc(X(x), Y(y), Math.max(0.5, Math.max(R * k, GHOST_MIN_PX) * (1 - e)), 0, 2 * Math.PI);
        air.stroke();
        air.setLineDash([]);
      }
    }
  };
  // Hidden, or asked to keep still: nothing waits to fly and nothing flies.
  const ground = () => {
    for (const t of waiting) clearTimeout(t);
    waiting.clear();
    flights.length = 0;
    lives.length = 0;
    if (flying) cancelAnimationFrame(flying);
    flying = 0;
    clearAir();
  };
  still.addEventListener("change", () => still.matches && ground());

  // ---- Reading. ----

  // Each reading comes from site.ts, which says over the map how the feed
  // stands; the drawing only draws.
  let first = true;
  const take = (now) => {
    try {
      // Each redraw is measured, so its cost at any size can be read from
      // outside (performance.getEntriesByName("mycelium-redraw")).
      const started = performance.now();
      // model.ts has written every cut title's ellipsis as three full stops.
      current = model(now);
      sow(current);
      if (!findLine.hidden) refind({ keep: true });
      if (first) {
        judgeChanged();
        firstLives();
        first = false;
        apply(false);
        hear(false);
        // A talk opened from the address was written before the ties were
        // heard, so it is written again now that they are.
        if (tieOf.size) panel();
      } else {
        if (chosen && !knows(chosen)) chosen = null;
        const was = spots;
        draw();
        noteLives(was);
        panel();
        hear(true);
      }
      drawStatus();
      performance.clearMeasures("mycelium-redraw");
      performance.measure("mycelium-redraw", { start: started });
      // What was made of the reading, once drawn whole, for reach.ts to
      // judge beside it; a reading that fails to draw says nothing.
      document.dispatchEvent(new CustomEvent("mycelium:modelled", { detail: { now, model: current } }));
    } catch (err) {
      // A reading that fails to draw says why in the console, so a page
      // stuck at "reading" can be told from a node that isn't answering.
      console.error("mycelium: a reading failed to draw", err?.stack || err);
      // The next change brings the next reading, which is drawn whole.
      asof.textContent = `The map couldn't draw: ${err?.message || err}. It tries again with the next change.`;
    }
  };
  document.addEventListener("mycelium:now", (e) => take(e.detail));

  // How things stand, as one sentence over the work, its last clause the
  // italic turn: who needs him, else what is stuck or moving, else that
  // all is quiet. A count Now lists is the way to that list: the cells to
  // the network as a tree, who needs him to the first of them. The
  // sentence is rewritten only when its words change, so a count he is on
  // keeps its focus. What is spoken is coarser, so the polite live region
  // speaks only when the network changes, never on each reading.
  // How the network stands and who needs him are two spans of the one
  // sentence, so #status-words holds the counts alone, as the page's test
  // reads them.
  const statusWords = document.getElementById("status-words");
  const asksWords = document.getElementById("asks-words");
  const sentenceLine = statusWords.closest("h1");
  const statusSaid = document.getElementById("status-said");
  let statusShown = "";
  let sentenceShown = "";
  const atWork = (moving, all) => (!moving ? "all quiet" : moving * 4 < all ? "a few at work" : moving * 2 < all ? "many at work" : "most at work");
  const COUNTED = {
    cells: () => focusItem(tree.querySelector('[role="treeitem"]')),
    asks: () => panelBody.querySelector(".asks-line + .rows button")?.focus({ preventScroll: true }),
  };
  const drawStatus = () => {
    if (!current) return;
    const n = countsOf(current.cells);
    const all = current.cells.length;
    const stuck = n.stuck ? `, ${NUMBER.format(n.stuck)} stuck` : "";
    const asks = n.asks === 1 ? "one needs you" : n.asks ? `${NUMBER.format(n.asks)} need you` : "";
    const clauses = [["cells", plural(all, "cell")]];
    if (n.moving) clauses.push(["", `${NUMBER.format(n.moving)} moving`]);
    if (n.stuck) clauses.push(["", `${NUMBER.format(n.stuck)} stuck`]);
    if (clauses.length === 1 && !asks) clauses.push(["", "all quiet"]);
    // A clause is kept whole on its line, so a count never parts from what
    // it counts, and the last clause of the sentence takes its turn.
    const clause = ([what, words], last) => {
      const shown = what ? `<button type="button" class="count" data-count="${what}" aria-controls="panel">${esc(words)}</button>` : `<span class="clause">${esc(words)}</span>`;
      return last ? `<em>${shown}</em>` : shown;
    };
    const counted = clauses.map((c, i) => clause(c, !asks && i === clauses.length - 1)).join(", ");
    const asked = asks ? `, ${clause(["asks", asks], true)}` : "";
    if (counted + asked !== sentenceShown) {
      const had = sentenceLine.contains(document.activeElement) ? document.activeElement.dataset.count : null;
      sentenceShown = counted + asked;
      statusWords.innerHTML = counted;
      asksWords.innerHTML = asked;
      if (had) (sentenceLine.querySelector(`[data-count="${had}"]`) || view).focus({ preventScroll: true });
    }
    const said = `${atWork(n.moving, all)}${stuck}${asks ? `; ${asks}` : ""}`;
    if (said === statusShown) return;
    statusShown = said;
    statusSaid.textContent = said;
  };
  // A count opens Now, with the keys on the first row of what it counts.
  sentenceLine.addEventListener("click", (e) => {
    const b = e.target.closest("[data-count]");
    if (!b) return;
    openNow();
    COUNTED[b.dataset.count]?.();
  });

  // Idle means zero: site.ts hands nothing over while the tab is hidden,
  // and what he saw is kept as he leaves, so the next look rings what
  // changed since.
  const watch = () => {
    if (document.hidden) {
      keepSeen();
      ground();
      return;
    }
    if (current) judgeChanged();
  };
  document.addEventListener("visibilitychange", watch);
  window.addEventListener("pagehide", keepSeen);

  // A new size keeps what's in the middle in the middle, or the whole map
  // whole if he hasn't moved it.
  let was = null;
  new ResizeObserver(() => {
    const [w, h] = seen();
    if (!current || !L) {
      was = [w, h];
      return;
    }
    if (auto) fit(false);
    else if (was) {
      tx += (w - was[0]) / 2;
      ty += (h - was[1]) / 2;
      frame();
    }
    was = [w, h];
  }).observe(svg);
  document.fonts?.ready.then(() => {
    if (!current || !L) return;
    measureLabels();
    placeLabels();
  });

  // ---- Keys and the switch, as on every one of his pages. ----

  const toggleKeys = () => (keys.open ? keys.close() : keys.showModal());
  const toggleMachines = () => {
    machinesShown = !machinesShown;
    if (!machinesShown && chosen?.kind === "machine") letGo();
    if (pointed?.[0] === "m" && !machinesShown) pointed = null;
    draw();
    if (!tree.parentElement.hidden) renderTree();
    where.textContent = machinesShown ? "The machines are shown, with each network's cord to the machine it runs on; m hides them." : "The machines are hidden; m shows them.";
  };
  // Out, as Enter is in: out of what's open, else up from where the walk
  // has reached to what it hangs from.
  const goOut = () => {
    // Out of an open network the keys come back up to it too, so the
    // level follows the view.
    const up = chosen && chosen.kind !== "cell" && pointed?.[0] === "c" ? treeUp(pointed) : null;
    if (up && spots.has(up)) point(up, { say: view.contains(document.activeElement), follow: false });
    if (chosen) back();
    else if (pointed && treeUp(pointed)) {
      const up = treeUp(pointed);
      const inTree = tree.contains(document.activeElement);
      point(up, { say: view.contains(document.activeElement) });
      if (inTree) focusItem(tree.querySelector(`[data-key="${CSS.escape(up)}"]`));
    }
  };
  // What each of the page's letters does, once keys.ts has said which.
  const PAGE_DOES = {
    keys: () => toggleKeys(),
    find: () => openFind(),
    "close-find": () => closeFind(),
    "next-found": () => stepFound(1),
    "found-before": () => stepFound(-1),
    "let-go": () => letGo(),
    "close-now": () => closeNow(),
    now: () => toggleNow(),
    machines: () => toggleMachines(),
    fit: () => fit(),
    "zoom-in": () => zoomBy(1.25),
    "zoom-out": () => zoomBy(0.8),
    out: () => goOut(),
  };
  document.addEventListener("keydown", (e) => {
    const asked = pageKey(e, {
      typing: !!e.target.closest?.("input, textarea, select"),
      keysOpen: keys.open,
      onMap: !!e.target.closest?.("#main"),
      findOpen: !findLine.hidden,
      finding: finding(),
      chosen: !!chosen,
      nowOpen: !!nowOpen,
    });
    if (!asked) return;
    e.preventDefault();
    PAGE_DOES[asked]();
  });
  // keys.ts lists every act a key can ask, and no typechecker reads this
  // file, so a table here missing one would swallow its key unseen. The
  // page says so as it loads instead, where every page test hears it.
  for (const [does, every, what] of [
    [TREE_DOES, EVERY_TREE_ACT, "tree"],
    [PAGE_DOES, EVERY_PAGE_ACT, "page"],
  ]) {
    const undone = every.filter((act) => typeof does[act] !== "function");
    if (undone.length) console.error(`The map's ${what} keys do nothing for ${undone.join(", ")}.`);
  }
  // The skip link moves focus without writing the map's place into the
  // address, which back would then have to walk through.
  document.querySelector(".skip").addEventListener("click", (e) => {
    e.preventDefault();
    view.focus();
  });
  const taps = { "/": openFind, ".": toggleNow, "?": toggleKeys };
  for (const kbd of document.querySelectorAll(".head .keys kbd")) {
    const act = taps[kbd.textContent];
    if (!act) continue;
    kbd.setAttribute("role", "button");
    kbd.tabIndex = 0;
    kbd.setAttribute("aria-label", { "/": "Find a cell", ".": "Now", "?": "Keys" }[kbd.textContent]);
    kbd.addEventListener("click", act);
    kbd.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        act();
      }
    });
  }

  const store = {
    set: (k2, v) => {
      try {
        localStorage.setItem(k2, v);
      } catch {
        // Without storage the choice holds for this visit only.
      }
    },
    del: (k2) => {
      try {
        localStorage.removeItem(k2);
      } catch {
        // Nothing was kept, so there's nothing to remove.
      }
    },
  };
  const themeSwitch = document.querySelector(".switch");
  const showTheme = () => {
    const choice = document.documentElement.dataset.theme || "system";
    for (const b of themeSwitch.querySelectorAll("button")) {
      const on = b.dataset.choice === choice;
      b.setAttribute("aria-checked", String(on));
      b.tabIndex = on ? 0 : -1;
    }
  };
  const chooseTheme = (choice) => {
    if (choice === "system") {
      delete document.documentElement.dataset.theme;
      store.del("theme");
    } else {
      document.documentElement.dataset.theme = choice;
      store.set("theme", choice);
    }
    showTheme();
  };
  showTheme();
  themeSwitch.addEventListener("click", (e) => {
    const b = e.target.closest("button[data-choice]");
    if (b) chooseTheme(b.dataset.choice);
  });
  themeSwitch.addEventListener("keydown", (e) => {
    const buttons = [...themeSwitch.querySelectorAll("button")];
    const to = switchedTo(e.key, buttons.indexOf(document.activeElement), buttons.length);
    if (to === null) return;
    e.preventDefault();
    const next = buttons[to];
    next.focus();
    chooseTheme(next.dataset.choice);
  });

  watch();
})();
