// The home of Joel's local sites: j and k move through the list, Return opens,
// gg and G go to the ends, and ? shows the keys.

// The keys the threshold names are buttons too, for a reader who taps or clicks.
const tapKeys = (taps) => {
  for (const kbd of document.querySelectorAll(".threshold .keys kbd")) {
    const act = taps.get(kbd.textContent);
    if (act) {
      kbd.setAttribute("role", "button");
      kbd.tabIndex = 0;
      kbd.addEventListener("click", act);
      kbd.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          act();
        }
      });
    }
  }
};

(() => {
  const links = [...document.querySelectorAll(".sites a")];
  const keys = document.querySelector("#keys");
  // The second g of gg comes within this many milliseconds of the first.
  const ggWithin = 600;
  let at = 0;
  let lastG = 0;
  const show = (i) => {
    at = Math.max(0, Math.min(links.length - 1, i));
    for (const [n, a] of links.entries()) {
      a.setAttribute("aria-current", String(n === at));
    }
    links[at].focus();
  };
  const moves = new Map([
    ["j", () => show(at + 1)],
    ["ArrowDown", () => show(at + 1)],
    ["k", () => show(at - 1)],
    ["ArrowUp", () => show(at - 1)],
    ["G", () => show(links.length - 1)],
  ]);
  const g = () => {
    const second = Date.now() - lastG < ggWithin;
    lastG = second ? 0 : Date.now();
    if (second) {
      show(0);
    }
  };
  document.addEventListener("focusin", (e) => {
    const n = links.indexOf(e.target.closest?.(".sites a"));
    if (n >= 0 && n !== at) {
      show(n);
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) {
      return;
    }
    if (e.key === "?") {
      e.preventDefault();
      if (keys.open) {
        keys.close();
      } else {
        keys.showModal();
      }
    } else if (!keys.open && e.key === "g") {
      g();
    } else if (!keys.open && moves.has(e.key)) {
      e.preventDefault();
      moves.get(e.key)();
    }
  });
  tapKeys(
    new Map([
      ["j", moves.get("j")],
      ["k", moves.get("k")],
      ["?", () => keys.showModal()],
    ]),
  );
  if (links.length > 0) {
    links[0].setAttribute("aria-current", "true");
  }
})();
