// The mycelium map's own checks, the misses Joel found himself turned into
// code: a fit that leaves part of what was opened off screen, a drawer that
// doesn't say what kind of thing it is, names on screen that aren't what he
// is engaged with, and drawers that don't open the same way. Run whole as
// one expression in the page, as the web skill's gate.js and layout.js are,
// on a map that has drawn; it answers with a line for each fault, saying
// what to fix, and an empty answer passes. It opens things through the hash,
// so run it on a page nobody is reading. A check that finds nothing to test
// fails rather than passing. It counts no cells: whoever feeds the page
// knows how many it holds, and says so of #status-words itself.
//
// What each check reads:
// - drawn: the standing line says the map is live (or stale, still drawn),
//   and at least one network is on the map;
// - fit: once a network is opened, its hub, its branches' hubs and every
//   cell of its own lie inside the map, left of the drawer;
// - words: the first sentence of a network's drawer says "network", a
//   machine's says "machine" and a cell's says "cell";
// - layer: the whole map draws no machine until "m" shows them, and "m"
//   then draws at least one;
// - reach: until the map folds, every cell the sentence counts has a
//   mark, and no group of cells working alone lists a receptor;
// - screen: at the whole map and one and two zooms in, at most
//   NAMES_ON_SCREEN names past the ones he's engaged with, no name over a
//   cell or a network's mark, and every cell drawn at CELL_PX across or
//   more, since smaller ones fold into their network; no network named
//   while the cap leaves off a cell that needs him; some cell drawn unless
//   the map says it folded; and each Zoom In moves the view;
// - lines: the whole map paints lines across networks where knots stand,
//   and talk wherever talk has two ends on the map;
// - names: on the whole map, no more names than networks, branches
//   included, past the one the keys rest on and the cells that need him;
//   once something is open, no faded name at all and at most NAMES_OPEN
//   names;
// - parity: every drawer, Now, network, branch, machine and cell, opens
//   with Purpose, Doing and Health, in that order, in a dl.glance, with no
//   words squeezed into less than half their column.
(async () => {
  const faults = [];
  const NETS_TRIED = 10;
  const NAMES_OPEN = 12;
  const SETTLE_MS = 900;
  // The map folds networks past this many cells, as view.js's FOLD_OVER.
  const FOLD_OVER = 400;
  const EDGE_PX = 1;
  // Less map beside the drawer than one target: the drawer stands in for it.
  const NO_ROOM_PX = 44;
  const CRUSH_SHARE = 0.5;
  // About as many things as a person tracks at a glance, as view.js's.
  const NAMES_ON_SCREEN = 7;
  // The least a cell is drawn across before its network folds, twice view.js's FOLD_PX.
  const CELL_PX = 5;
  const SLACK_PX = 0.25;
  // A cell's mark is its hit circle, R + 4 round its membrane's R, as
  // view.js draws them; the membrane is what he sees.
  const MEMBRANE_SHARE = 11 / 15;
  const settle = () => new Promise((done) => setTimeout(done, SETTLE_MS));
  const open = async (hash) => {
    history.pushState(null, "", hash || location.pathname);
    dispatchEvent(new PopStateEvent("popstate"));
    await settle();
  };
  const shown = (el) => el.isConnected && el.checkVisibility({ opacityProperty: true, visibilityProperty: true }) && getComputedStyle(el).display !== "none";
  const names = () => [...document.querySelectorAll("#labels .tag")].filter(shown);
  const marks = (sel) => [...document.querySelectorAll(`#world ${sel}`)];
  const view = document.getElementById("map-view");
  const panel = document.getElementById("panel");
  const body = document.getElementById("panel-body");
  // A sentence ends at a stop, a colon or a semicolon followed by a space.
  const firstSentence = () => {
    const words = body.querySelector(".carries")?.textContent || "";
    const ends = [". ", ": ", "; "].map((end) => words.indexOf(end)).filter((at) => at >= 0);
    return ends.length > 0 ? words.slice(0, Math.min(...ends) + 1) : words;
  };

  // Nothing else can be judged on a map that never drew, so this one
  // answers alone. A draw that failed says so as broken, until a drawing
  // after it succeeds.
  const standingLine = document.getElementById("asof");
  const asof = standingLine?.textContent.trim() || "";
  const state = standingLine?.dataset.state;
  const drew = ["live", "stale"].includes(state);
  if (!drew) {
    return [`drawn: the standing line reads "${asof}"${state ? ` (${state})` : ""}; the map didn't draw its reading (see the console)`];
  }
  if (marks(".hub").length === 0) {
    return ["drawn: no network is on the map"];
  }

  const glanceOf = (what) => {
    const heads = [...body.querySelectorAll("dl.glance dt")].map((dt) => dt.textContent.trim());
    if (heads.join("|") !== "Purpose|Doing|Health") {
      faults.push(`parity: ${what}'s drawer heads read "${heads.join(", ") || "nothing"}"; it should open with Purpose, Doing and Health in a dl.glance`);
    }
    // Words crushed into a narrow column wrap without overflowing, so the
    // layout check never sees them: every block of words in the glance
    // takes at least half the width its answer column has.
    for (const dd of body.querySelectorAll("dl.glance dd")) {
      const room = dd.getBoundingClientRect().width;
      for (const el of dd.querySelectorAll(".what, .name, .fact, p, span.doing")) {
        const w = el.getBoundingClientRect().width;
        if (el.textContent.trim() && w < room * CRUSH_SHARE) {
          faults.push(`parity: in ${what}'s drawer, "${el.textContent.trim().slice(0, 40)}" is squeezed into ${Math.round(w)} of ${Math.round(room)} px`);
          return;
        }
      }
    }
  };
  const wordsOf = (what, kind) => {
    const first = firstSentence();
    if (!new RegExp(`\\b${kind}\\b`, "iu").test(first)) {
      faults.push(`words: ${what}'s drawer begins "${first || "with nothing"}"; its first sentence should say "${kind}"`);
    }
  };
  const fitOf = (what, key) => {
    const v = view.getBoundingClientRect();
    const right = panel.checkVisibility() ? Math.min(v.right, panel.getBoundingClientRect().left) : v.right;
    // Too narrow for a gutter, the drawer takes the map's whole width and
    // stands in for it until Escape, so there is no map left to fit into.
    if (right - v.left < NO_ROOM_PX) {
      return;
    }
    const owned = [
      ...marks(`[data-key="h:${CSS.escape(key)}"]`),
      ...marks(`[data-branch-of="${CSS.escape(key)}"]`),
      ...marks(`.m-hit[data-within="${CSS.escape(key)}"]`),
    ];
    const out = owned.filter((el) => {
      const b = el.getBoundingClientRect();
      if (!(b.width || b.height)) {
        return false;
      }
      const [x, y] = [b.left + b.width / 2, b.top + b.height / 2];
      return x < v.left - EDGE_PX || x > right + EDGE_PX || y < v.top - EDGE_PX || y > v.bottom + EDGE_PX;
    });
    if (out.length > 0) {
      faults.push(`fit: ${what} opened with ${out.length} of its ${owned.length} marks off the map or under the drawer, the first ${out[0].dataset.key}`);
    }
  };
  const namesOpen = (what) => {
    const on = names();
    const far = on.filter((t) => t.classList.contains("far"));
    if (far.length > 0) {
      faults.push(`names: ${what} open shows ${far.length} faded names, the first ${far[0].dataset.name}; a name not engaged hides`);
    }
    if (on.length > NAMES_OPEN) {
      faults.push(`names: ${what} open shows ${on.length} names, past ${NAMES_OPEN}`);
    }
  };

  const pressM = async () => {
    view.focus();
    view.dispatchEvent(new KeyboardEvent("keydown", { key: "m", bubbles: true }));
    await settle();
  };
  // The whole map as he first sees it: no machine, and a name per network.
  const wholeMap = () => {
    if (marks('[data-key^="m:"]').length > 0) {
      faults.push("layer: the whole map draws machines before m asks for them; they are a layer of their own");
    }
    // A knot, a network with no leader, is a network too.
    const hubs = marks('[data-key^="h:"], [data-key^="n:"]').length;
    // The name the keys rest on is what he's engaged with, which keeps its
    // name wherever it is, so it counts against no budget; nor does a cell
    // that needs him, which is named before any network.
    const askingKeys = new Set(marks("[data-asks]").map((el) => el.dataset.key));
    const whole = names().filter((el) => !(el.classList.contains("strong") || askingKeys.has(el.dataset.name))).length;
    if (whole > hubs) {
      faults.push(`names: the whole map shows ${whole} names, past one per network (${hubs})`);
    }
    // Lines are quieted, never taken away: a network shared across others
    // keeps its lines at the whole map, and so does the talk between cells.
    const painted = document.getElementById("map-ground")?.painted?.() ?? { across: 0, talk: 0, between: 0 };
    if (marks('[data-key^="n:"]').length > 0 && painted.across === 0) {
      faults.push("lines: the whole map paints no line across networks, though knots stand on it");
    }
    // Talk inside one folded network waits in its ground, so only talk
    // with two ends on the map must paint.
    if (painted.between > 0 && painted.talk === 0) {
      faults.push(`lines: the whole map paints none of the ${painted.between} talk arcs it holds between two ends`);
    }
  };
  // Every cell he can reach: each one the sentence counts has its mark,
  // until the map folds; and a receptor stands at the edge, never among
  // the cells working alone.
  // What the map shows at one zoom, judged on screen: a handful of names,
  // none over a cell or a network's mark, and every cell large enough to
  // read as its kind.
  const screen = (what) => {
    const box = view.getBoundingClientRect();
    const inView = (r) => r.right > box.left && r.left < box.right && r.bottom > box.top && r.top < box.bottom;
    const on = names().filter((el) => !el.classList.contains("engaged") && !el.classList.contains("t-mark"));
    if (on.length > NAMES_ON_SCREEN) {
      faults.push(`screen: ${what} shows ${on.length} names, past ${NAMES_ON_SCREEN}`);
    }
    const cells = marks('circle.cell, [data-key^="c:"]').map((el) => el.getBoundingClientRect()).filter(inView);
    // Cells that need him take the seven names before any network does, so
    // none of them is left off by the cap while a network holds a name. A
    // name with no clear place near its mark waits for a closer view, which
    // is no fault; the map lists the names the cap left off.
    const capped = new Set((document.getElementById("labels")?.dataset.capped || "").split(" "));
    const askingCapped = marks("[data-asks]").filter((el) => capped.has(el.dataset.key)).length;
    // The network he's engaged with is named beyond the seven, so it doesn't count.
    const netsNamed = on.filter((el) => ["h:", "n:"].some((kind) => (el.dataset.name || "").startsWith(kind))).length;
    if (askingCapped > 0 && netsNamed > 0) {
      faults.push(`screen: ${what} names ${netsNamed} networks while the cap leaves off ${askingCapped} cells that need him`);
    }
    // No cell drawn is a pass only when the map says it folded them.
    if (cells.length === 0 && !document.getElementById("map-ground")?.painted?.().folded) {
      faults.push(`screen: ${what} draws no cell, and nothing is folded`);
    }
    const small = cells.filter((r) => r.width * MEMBRANE_SHARE < CELL_PX - SLACK_PX);
    if (small.length > 0) {
      faults.push(`screen: ${what} draws ${small.length} cells under ${CELL_PX} px across, the smallest ${Math.min(...small.map((r) => r.width * MEMBRANE_SHARE)).toFixed(1)}; their networks fold`);
    }
    const nets = marks('[data-key^="h:"] .m-hub, [data-key^="n:"] .m-hub').map((el) => el.getBoundingClientRect()).filter(inView);
    const hit = (a, b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
    for (const el of on) {
      const r = el.getBoundingClientRect();
      const under = [...cells, ...nets].filter((c) => hit(r, c)).length;
      if (under > 0) {
        faults.push(`screen: ${what} names ${el.dataset.name || el.textContent.trim().slice(0, 30)} over ${under} marks`);
      }
    }
  };
  const zoomIn = async () => {
    const was = document.getElementById("world")?.getAttribute("transform");
    document.querySelector('button[data-zoom="in"]')?.click();
    await settle();
    if (document.getElementById("world")?.getAttribute("transform") === was) {
      faults.push("screen: Zoom In left the view where it was");
    }
  };
  const reach = async () => {
    const counted = Number((document.querySelector('[data-count="cells"]')?.textContent || "").replace(/\D/g, ""));
    const drawn = marks('[data-key^="c:"]').length;
    const folds = document.getElementById("map-ground")?.painted?.().folded ?? 0;
    if (counted <= FOLD_OVER && folds === 0 && drawn !== counted) {
      faults.push(`reach: the sentence counts ${counted} cells and the map marks ${drawn}`);
    }
    for (const hub of marks('[data-key^="h:#loose"]')) {
      await open(`#net=${encodeURIComponent(hub.dataset.key.slice(2))}`);
      if (document.querySelector("#panel .rows .m-receptor")) {
        faults.push("reach: a receptor is listed among the cells working alone");
      }
    }
    await open("");
  };
  // The machines' layer: "m" draws them, and each drawer reads as a
  // machine's and opens as every drawer does; then they go away again.
  const machineLayer = async () => {
    await pressM();
    const machines = marks('[data-key^="m:"]');
    if (machines.length === 0) {
      faults.push("layer: m drew no machine");
    }
    for (const m of machines) {
      const id = m.dataset.key.slice(2);
      await open(`#machine=${encodeURIComponent(id)}`);
      wordsOf(`the machine ${id}`, "machine");
      glanceOf(`the machine ${id}`);
      namesOpen(`the machine ${id}`);
    }
    // Letting go of a machine keeps the layer on, so one more "m" hides it.
    await open("");
    await pressM();
  };

  await open("");
  const tops = marks('[data-key^="h:"]:not([data-branch-of])');
  if (tops.length === 0) {
    return ["nothing to check: the map drew no networks"];
  }
  wholeMap();
  screen("the whole map");
  await reach();
  await zoomIn();
  screen("one zoom in");
  await zoomIn();
  screen("two zooms in");
  document.querySelector('button[data-zoom="fit"]')?.click();
  await settle();

  // Now: parity with the rest.
  const nowControl = document.getElementById("now-control");
  nowControl?.click();
  await settle();
  glanceOf("Now");
  nowControl?.click();
  await settle();

  await machineLayer();

  // The largest networks, a branch among them when there is one.
  const sized = tops
    .map((h) => [h, marks(`.m-hit[data-within="${CSS.escape(h.dataset.key.slice(2))}"]`).length])
    .sort((a, b) => b[1] - a[1])
    .slice(0, NETS_TRIED)
    .map(([h]) => h);
  const branch = marks("[data-branch-of]")[0];
  let cellTried = false;
  for (const h of branch ? [...sized, branch] : sized) {
    const key = h.dataset.key.slice(2);
    const what = `the network ${key}${h.dataset.branchOf ? " (a branch)" : ""}`;
    await open(`#net=${encodeURIComponent(key)}`);
    fitOf(what, key);
    wordsOf(what, "network");
    glanceOf(what);
    namesOpen(what);
    const cell = marks(`.m-hit[data-within="${CSS.escape(key)}"]`)[0];
    if (!cellTried && cell) {
      cellTried = true;
      const id = cell.dataset.key.slice(2);
      await open(`#cell=${encodeURIComponent(id)}`);
      wordsOf(`a cell of ${key}`, "cell");
      glanceOf(`a cell of ${key}`);
      namesOpen(`a cell of ${key}`);
    }
  }
  if (!cellTried) {
    faults.push("nothing to check: no network had a cell to open");
  }
  await open("");
  return faults;
})()
