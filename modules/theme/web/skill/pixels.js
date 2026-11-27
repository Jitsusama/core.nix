// The gate's probe for every pixel (checks.md), beside gate.js and layout.js
// and run the same way: whole, as one expression in the page, at each width.
// It answers with where the page doesn't hold at that width, each in a line
// saying what to do, and an empty answer passes:
//
// - Nothing scrolls sideways: the page is no wider than its window (WCAG
//   1.4.10), so a phone at 390 px never pans.
// - Nothing moves as it arrives: the page's layout shift, counted as the
//   browser's Cumulative Layout Shift counts it, stays under 0.1. Run it
//   first on a page freshly loaded, before layout.js opens its folds.
//
// Each word's size is floor.js's, its contrast words.js's, and a control's
// name names.js's, each run beside it.
//
// It ends without a semicolon, since an eval asked for one expression
// answers nothing for a statement.
(async () => {
  const missing = [];
  const page = document;
  const root = page.documentElement;
  const shown = (element) =>
    element?.isConnected === true &&
    element.checkVisibility({ visibilityProperty: true, opacityProperty: true });

  // Sideways: the page's own width against its window's, then the widest
  // thing past the window's edge that no scroller of its own holds, so the
  // line names what to fix.
  const wide = root.clientWidth;
  if (root.scrollWidth > wide + 1) {
    const scrolls = (element) => {
      for (let at = element.parentElement; at !== null && at !== root; at = at.parentElement) {
        if (getComputedStyle(at).overflowX !== "visible") {
          return true;
        }
      }
      return false;
    };
    let widest = null;
    for (const element of page.body.querySelectorAll("*")) {
      const box = element.getBoundingClientRect();
      if (box.right > wide + 1 && !scrolls(element) && shown(element)) {
        if (widest === null || box.right > widest.right) {
          widest = { element, right: box.right };
        }
      }
    }
    const reach = widest === null ? 0 : Math.round(widest.right);
    const what = widest === null ? "" : ` (${widest.element.localName} reaches ${reach} px)`;
    missing.push(
      `the page is ${root.scrollWidth} px wide in a ${wide} px window${what}: ` +
        "let it wrap, or give it a scroller of its own",
    );
  }

  // Shift, by the browser's own count: shifts a second or less apart make a
  // window of at most five seconds, and the largest window is the page's.
  // One just after a press is the press's, so it's left out, as the browser
  // leaves it out. The count runs from the page's load, so the probe goes
  // first on a page freshly loaded and drawn, before anything opens a fold:
  // layout.js's openings would count as the page's own. It waits for half a
  // second with no shift, at most about three, so one arriving as it's
  // called is counted rather than missed. It waits on timers alone, never a
  // frame: a tab out of sight draws no frames, but its timers still run, so
  // the wait ends there too.
  const quiet = 500;
  const longest = 3000;
  // The browser keeps no more than this many shifts from before the probe
  // began to watch (the Layout Instability spec's buffer).
  const kept = 150;
  const watched = await new Promise((resolve) => {
    const heard = [];
    const began = performance.now();
    let lastHeard = began;
    const hear = (entries) => {
      heard.push(...entries);
      for (const entry of entries) {
        lastHeard = Math.max(lastHeard, entry.startTime);
      }
    };
    const watch = new PerformanceObserver((list) => hear(list.getEntries()));
    try {
      watch.observe({ type: "layout-shift", buffered: true });
    } catch {
      resolve(null);
      return;
    }
    const settled = () => {
      const now = performance.now();
      if (now - lastHeard < quiet && now - began < longest) {
        setTimeout(settled, Math.min(quiet - (now - lastHeard), longest - (now - began)));
        return;
      }
      hear(watch.takeRecords());
      watch.disconnect();
      resolve({ heard, began });
    };
    setTimeout(settled, quiet);
  });
  if (watched === null) {
    missing.push(
      "this browser can't observe layout shifts, so the shift went unmeasured: run it in Chromium",
    );
  } else if (watched.heard.filter((shift) => shift.startTime < watched.began).length >= kept) {
    missing.push(
      `the page shifted ${kept} times before the probe ran, all the browser keeps, so its ` +
        "shift may be more than counted: stop what moves it as it draws",
    );
  }
  const shifts = watched?.heard ?? [];
  let largest = 0;
  let session = 0;
  let first = 0;
  let last = 0;
  for (const shift of shifts.filter((one) => !one.hadRecentInput)) {
    if (session > 0 && shift.startTime - last < 1000 && shift.startTime - first < 5000) {
      session += shift.value;
    } else {
      session = shift.value;
      first = shift.startTime;
    }
    last = shift.startTime;
    largest = Math.max(largest, session);
  }
  if (largest >= 0.1) {
    missing.push(
      `the page shifts ${largest.toFixed(2)} as it arrives, over 0.1: ` +
        "reserve each part's box (aspect-ratio, a fixed height) before it's filled",
    );
  }

  return missing;
})()
