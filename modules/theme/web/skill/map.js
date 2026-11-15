// The gate's probe for a walked map or graph (checks.md), beside gate.js and
// layout.js and run the same way: whole, as one expression in the page as
// published, at each width and in each look. It answers with where the map
// doesn't walk or read the way SKILL.md's "A picture is walked" says, each
// in a line saying what to do, and an empty answer passes. It presses keys
// on the map, so run it on a page nobody is reading. It ends without a
// semicolon, since an eval asked for one expression answers nothing for a
// statement.
//
// A map tells the probe what it is with a few attributes, none of them
// read by anyone else:
// - data-map on the element that takes focus and the keys, with data-level
//   naming the level the arrows walk now, and data-within the key of what
//   it has opened, when it has;
// - data-key on each mark the arrows can reach, with data-level and, below
//   the top, data-within, the key of what holds it;
// - data-here on the one mark the keys rest on;
// - data-name on each drawn name, the whole of its ink, halo or plate
//   included, and data-line on each drawn link;
// - data-lines on an element, a canvas most often, that draws links
//   without an element for each, with a lines() method answering them as
//   polylines of {x, y} in CSS pixels of the viewport, the space
//   getBoundingClientRect answers in, never canvas or device pixels.
//
// A check that finds nothing to test fails rather than passing, since a
// map that hides what it draws would otherwise read as one that draws it
// well.
(async () => {
  const missing = [];
  const page = document;
  const map = page.querySelector("[data-map]");
  if (map === null) {
    return ["a data-map on the element that takes the map's keys, for its walk to be measured"];
  }
  if (map.tabIndex < 0) {
    missing.push("the map takes no focus: give its data-map element tabindex 0");
  }
  const settle = () => new Promise((done) => setTimeout(done, 450));
  const shown = (element) =>
    element.isConnected &&
    element.checkVisibility({ opacityProperty: true, visibilityProperty: true });
  const box = (element) => element.getBoundingClientRect();
  const centre = (element) => {
    const edges = box(element);
    return [edges.left + edges.width / 2, edges.top + edges.height / 2];
  };
  const marks = () => [...map.querySelectorAll("[data-key]")].filter(shown);
  const here = () => map.querySelector("[data-here]")?.dataset.key ?? null;
  const said = () =>
    [...page.querySelectorAll("[aria-live], [role=status], [role=alert], [role=log]")]
      .map((region) => region.textContent)
      .join(" ");
  // A key is pressed as the page hears one, on what holds focus, and the
  // answer says whether the page kept the browser from acting on it.
  const press = async (key, shiftKey = false) => {
    const event = new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true });
    const kept = !(page.activeElement ?? map).dispatchEvent(event);
    await settle();
    return kept;
  };

  map.focus();
  await settle();
  if (marks().length === 0) {
    return ["no mark the arrows can reach carries data-key, for the walk to be measured"];
  }
  if (marks().some((mark) => mark.dataset.level === undefined)) {
    missing.push(
      "a mark carries no data-level: name each mark's level, for the walk to keep to one",
    );
  }
  if (here() === null) {
    await press("ArrowRight");
  }
  if (here() === null) {
    return ["focus on the map rests on no mark: mark the one the keys rest on with data-here"];
  }

  // The rule the arrows keep: the nearest mark on the level being walked,
  // inside what's open, whose centre lies within 45 degrees of the arrow,
  // by distance along it. When none does, focus stays and the page says so,
  // and the opposite arrow comes straight back.
  const ways = { ArrowRight: [1, 0], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowUp: [0, -1] };
  const back = {
    ArrowRight: "ArrowLeft",
    ArrowLeft: "ArrowRight",
    ArrowDown: "ArrowUp",
    ArrowUp: "ArrowDown",
  };
  const cone = Math.cos(Math.PI / 4) - 1e-6;
  const expected = (from, [dx, dy]) => {
    const start = map.querySelector(`[data-key="${CSS.escape(from)}"]`);
    const level = map.dataset.level ?? start.dataset.level;
    const within = map.dataset.within;
    const [px, py] = centre(start);
    let pick = null;
    let along = Number.POSITIVE_INFINITY;
    for (const mark of marks()) {
      const key = mark.dataset.key;
      const walked = mark.dataset.level === level;
      const held = within === undefined || mark.dataset.within === within;
      const [qx, qy] = centre(mark);
      const [vx, vy] = [qx - px, qy - py];
      const reach = Math.hypot(vx, vy);
      const distance = vx * dx + vy * dy;
      const ahead = reach > 0 && distance / reach >= cone;
      if (key !== from && walked && held && ahead && distance < along) {
        along = distance;
        pick = key;
      }
    }
    return pick;
  };
  const strayed = new Set();
  // The last step that moved, so an opposite arrow pressed next is judged
  // as the retrace it promises rather than by the cone, which a step back
  // to a mark outside it would otherwise fail.
  let last = null;
  const order = ["ArrowRight", "ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft", "ArrowDown"];
  for (let step = 0; step < 24; step += 1) {
    const arrow = order[step % order.length];
    const from = here();
    if (from === null || map.querySelector(`[data-key="${CSS.escape(from)}"]`) === null) {
      strayed.add(
        `after ${step} arrow presses the keys rest on no mark (${from}): ` +
          "mark the one they rest on with data-here, on a mark with data-key",
      );
      break;
    }
    const retrace = last !== null && arrow === back[last.arrow] && from === last.to;
    const want = retrace ? last.from : expected(from, ways[arrow]);
    const before = said();
    await press(arrow);
    const landed = here();
    if (want === null && landed !== from) {
      strayed.add(
        `${arrow} from ${from} moved to ${landed} where nothing on its level lies within 45 ` +
          "degrees: keep focus where it is and say so",
      );
    } else if (want === null && said() === before) {
      strayed.add(
        `${arrow} from ${from} found nothing that way and said nothing new: ` +
          "say so in the live region, naming the way, so a repeat is heard too",
      );
    } else if (retrace && landed !== want) {
      strayed.add(
        `${arrow} after ${back[arrow]} from ${want} went to ${landed}: ` +
          "the opposite arrow retraces the step",
      );
    } else if (want !== null && landed !== want) {
      strayed.add(
        `${arrow} from ${from} moved to ${landed}, not ${want}, the nearest mark on its level ` +
          "within 45 degrees: walk by that rule alone",
      );
    }
    if (landed !== from) {
      last = { arrow, from, to: landed };
      await press(back[arrow]);
      if (here() !== from) {
        strayed.add(
          `${back[arrow]} after ${arrow} came back to ${here()}, not ${from}: ` +
            "the opposite arrow retraces the step",
        );
      }
      await press(arrow);
    }
  }
  missing.push(...[...strayed].slice(0, 6));

  // Escape closes only what a mode opened: the view and the level stay, and
  // only - steps back out.
  const view = () =>
    marks()
      .slice(0, 8)
      .map((mark) => centre(mark).map(Math.round).join())
      .join(" ");
  // A map still gliding, or redrawn from new data, moves on its own; it's
  // judged only once it holds still for a settle.
  await settle();
  const placed = view();
  await settle();
  const still = view() === placed;
  const level = [map.dataset.level, map.dataset.within].join();
  await press("Escape");
  if (!still) {
    missing.push(
      "the map never held still for a settle, so Escape went unjudged: " +
        "let nothing but motion's own marks move once it has drawn",
    );
  } else if (view() !== placed) {
    missing.push("Escape moved the map: let it close only what a mode opened, never move the view");
  }
  if ([map.dataset.level, map.dataset.within].join() !== level) {
    missing.push("Escape changed the level the arrows walk: only - steps back out");
  }
  map.focus();
  const outside = map.dataset.within;
  await press("Enter");
  if (map.dataset.within !== undefined && map.dataset.within !== outside) {
    const opened = map.dataset.within;
    await press("-");
    if (map.dataset.within === opened) {
      missing.push(`- inside ${opened} didn't step back out: let it go up one level`);
    }
  }

  // Tab and Shift+Tab leave the map from any depth, for the next thing on
  // the page and the one before, so focus is never trapped.
  map.focus();
  for (const shiftKey of [false, true]) {
    if (await press("Tab", shiftKey)) {
      missing.push(`the map keeps ${shiftKey ? "Shift+Tab" : "Tab"} from leaving it: let it go on`);
    }
  }

  // A drawn name reads whole: its ink, halo included, crosses no other
  // name and no line, and stays inside the map's own box.
  map.focus();
  await settle();
  const frame = box(map);
  const names = [...map.querySelectorAll("[data-name]")].filter(shown).map((name) => ({
    name,
    edges: box(name),
    words: name.textContent.trim().slice(0, 40),
  }));
  if (names.length === 0 && marks().length > 1) {
    missing.push(
      "nothing says where the map's names are drawn: " +
        "mark each with data-name, halo included, to be measured",
    );
  }
  const crosses = (one, other) =>
    Math.min(one.right, other.right) - Math.max(one.left, other.left) > 1 &&
    Math.min(one.bottom, other.bottom) - Math.max(one.top, other.top) > 1;
  const clashes = new Set();
  for (const [at, one] of names.entries()) {
    if (
      one.edges.left < frame.left - 1 ||
      one.edges.right > frame.right + 1 ||
      one.edges.top < frame.top - 1 ||
      one.edges.bottom > frame.bottom + 1
    ) {
      clashes.add(`"${one.words}" runs past the map's edge: draw it whole inside or not at all`);
    }
    for (const other of names.slice(at + 1)) {
      if (crosses(one.edges, other.edges)) {
        clashes.add(
          `"${one.words}" lies over "${other.words}": ` +
            "move one, or leave one out at this depth",
        );
      }
    }
  }
  const inside = (edges, [x, y]) =>
    x > edges.left + 1 && x < edges.right - 1 && y > edges.top + 1 && y < edges.bottom - 1;
  // Every link as points 4 px or less apart on screen, read afresh now,
  // after the settle, so a re-layout or a resize is never judged stale.
  const drawn = [...map.querySelectorAll("[data-line]")].filter(shown).map((line) => {
    const length = typeof line.getTotalLength === "function" ? line.getTotalLength() : 0;
    const toScreen = line.getScreenCTM();
    const steps = Math.min(400, Math.ceil(length / 4));
    const points = [];
    for (let at = 0; toScreen !== null && length > 0 && at <= steps; at += 1) {
      points.push(line.getPointAtLength((length * at) / steps).matrixTransform(toScreen));
    }
    return points;
  });
  const sayers = [...map.querySelectorAll("[data-lines]")].filter(shown);
  for (const sayer of sayers) {
    const polylines = typeof sayer.lines === "function" ? sayer.lines() : null;
    if (!Array.isArray(polylines)) {
      missing.push("a data-lines element has no lines(): answer its links as polylines");
      continue;
    }
    for (const polyline of polylines) {
      const points = [];
      for (const [at, end] of polyline.slice(1).entries()) {
        const start = polyline[at];
        const steps = Math.min(400, Math.ceil(Math.hypot(end.x - start.x, end.y - start.y) / 4));
        for (let step = 0; step <= steps; step += 1) {
          const along = steps === 0 ? 0 : step / steps;
          points.push({
            x: start.x + (end.x - start.x) * along,
            y: start.y + (end.y - start.y) * along,
          });
        }
      }
      drawn.push(points);
    }
  }
  if (drawn.every((points) => points.length === 0) && marks().length > 1) {
    const canvas = [...map.querySelectorAll("canvas")].some(shown);
    missing.push(
      canvas && sayers.length === 0
        ? "a canvas draws the links, but nothing says where they run: " +
            "give it data-lines and lines()"
        : "nothing says where the links run: give each link data-line, or what draws them " +
            "data-lines and lines()",
    );
  }
  for (const points of drawn) {
    const under = points
      .map((point) => names.find((one) => inside(one.edges, [point.x, point.y])))
      .find((one) => one !== undefined);
    if (under !== undefined) {
      clashes.add(`a line runs under "${under.words}": no name sits over a line`);
    }
  }
  missing.push(...[...clashes].slice(0, 8));
  return missing;
})()
