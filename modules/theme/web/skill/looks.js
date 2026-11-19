// The gate's probe for checks.md's "A deploy keeps its looks", beside
// gate.js, layout.js, map.js and homes.js and run the same way: whole, as
// one expression in the page. It lists what the page draws beyond its
// words, measured from the render rather than read from the markup, since a
// stylesheet that answers 404 or a face that never loads is still in the
// markup: each stylesheet that loaded with rules in it, each face loaded,
// the type the body and the title are set in, the page's colours, the
// masthead and its mark, each drawing and image drawn, and each script that
// loaded. It ends without a semicolon, since an eval asked for one
// expression answers nothing for a statement.
//
// Run on the live page before a deploy, it answers { looks, broken }: keep
// `looks`. Run on the live page after it, with that list set first as
// `globalThis.looksBefore`, it answers { lost, meant, broken, looks }: each
// look the page had and has no more, each it was told was meant to go, by
// `globalThis.looksMeant`, an object from a look to why it went, and the
// looks it has now, the next deploy's before. Both runs
// name each asset the page asked for that answered an error, in `broken`.
// An empty `lost` and `broken` passes. On 18 November 2026 a deploy of a
// tracker lost its guide's masthead, heading art and stylesheets while
// every word check passed, and Joel found it himself: this is the check
// that would have stopped it.
//
// A look is a "kind value" string, so two runs compare as sets. An asset is
// named by its path from the page's folder, or from the site's root when
// it's above it, so a site moved to another address keeps its names. A
// drawing is named by its data-drawing, id or label, else by its place
// under the nearest element with an id. Every face the page declares is
// loaded, since one is fetched only once something is drawn in it, which
// can come after the page's load. Folds are opened to measure what's
// in them, and closed again, so run it on a page nobody is reading. A page
// with no stylesheet, face, masthead or picture of its own is a fault,
// never a quiet pass.
(async () => {
  if (document.readyState !== "complete") {
    await new Promise((loaded) => addEventListener("load", loaded, { once: true }));
  }
  const folder = new URL("./", location.href).href;
  const named = (address) => {
    const url = new URL(address, location.href);
    if (url.origin !== location.origin) {
      return url.href;
    }
    const path = `${url.origin}${url.pathname}`;
    return path.startsWith(folder) ? path.slice(folder.length) : url.pathname;
  };
  const unquoted = (name) => name.replaceAll('"', "").replaceAll("'", "").trim();
  const drawn = (element) => {
    const box = element.getBoundingClientRect();
    return box.width > 0 && box.height > 0;
  };
  const shut = [...document.querySelectorAll("details:not([open])")];
  for (const fold of shut) {
    fold.open = true;
  }
  const images = [...document.images];
  for (const image of images) {
    image.loading = "eager";
  }
  // Past five seconds, whatever hasn't loaded is taken as it stands.
  const patience = 5000;
  const wait = (promise) =>
    Promise.race([promise, new Promise((late) => setTimeout(late, patience))]);
  await wait(Promise.allSettled(images.map((image) => image.decode())));
  // One face that can't be loaded is lost, whenever its first use comes.
  await wait(Promise.allSettled([...document.fonts].map((face) => face.load())));

  const looks = new Set();
  const broken = [];
  const failing = 400;
  const answered = performance.getEntriesByType("resource");
  for (const entry of answered) {
    if (entry.responseStatus >= failing) {
      broken.push(`${named(entry.name)} answered ${entry.responseStatus}`);
    } else if ([".js", ".mjs"].some((end) => new URL(entry.name).pathname.endsWith(end))) {
      looks.add(`script ${named(entry.name)}`);
    }
  }
  for (const sheet of [...document.styleSheets].filter((each) => each.href !== null)) {
    let rules = 0;
    try {
      rules = sheet.cssRules.length;
    } catch {
      // A sheet from another origin keeps its rules to itself; that it
      // loaded at all is what can be said of it.
      rules = 1;
    }
    if (rules > 0) {
      looks.add(`stylesheet ${named(sheet.href)}`);
    }
  }
  for (const face of document.fonts) {
    // A face declared with no weight answers "normal", and one declared
    // 400 answers "400": both are the same face to a reader.
    const weight = { normal: "400", bold: "700" }[face.weight] ?? face.weight;
    const name = `face ${unquoted(face.family)} ${face.style} ${weight}`;
    if (face.status === "loaded") {
      looks.add(name);
    } else {
      broken.push(`${name} is ${face.status === "error" ? "declared and won't load" : face.status}`);
    }
  }
  const family = (element) =>
    unquoted(getComputedStyle(element).fontFamily.split(",")[0]);
  looks.add(`type body ${family(document.body)}`);
  const title = document.querySelector("h1");
  if (title !== null) {
    looks.add(`type title ${family(title)}`);
  }
  const ground = [document.body, document.documentElement]
    .map((element) => getComputedStyle(element).backgroundColor)
    .find((colour) => !["rgba(0, 0, 0, 0)", "transparent"].includes(colour));
  looks.add(`colour ground ${ground ?? "none"}`);
  looks.add(`colour ink ${getComputedStyle(document.body).color}`);
  const masthead = [
    ...document.querySelectorAll(".masthead, [role='banner'], body > header, main > header"),
  ].find(drawn);
  if (masthead !== undefined) {
    looks.add("masthead");
    if ([...masthead.querySelectorAll("svg, img")].some(drawn)) {
      looks.add("mark");
    }
  }
  const nameOf = (picture) => {
    const own =
      picture.dataset.drawing ||
      picture.id ||
      picture.getAttribute("aria-label") ||
      picture.querySelector(":scope > title")?.textContent.trim();
    if (own) {
      return own;
    }
    const place = picture.parentElement?.closest("[id]");
    const among = [...(place ?? document.body).querySelectorAll("svg")].filter(
      (each) => each.parentElement?.closest("svg") === null,
    );
    return `${place === null ? "page" : `#${place.id}`} ${among.indexOf(picture) + 1}`;
  };
  for (const picture of document.querySelectorAll("svg")) {
    if (picture.parentElement?.closest("svg") === null && drawn(picture)) {
      looks.add(`drawing ${nameOf(picture)}`);
    }
  }
  for (const image of images) {
    if (image.complete && image.naturalWidth > 0) {
      looks.add(`image ${named(image.currentSrc || image.src)}`);
    }
  }
  for (const fold of shut) {
    fold.open = false;
  }
  // The type and colours are always there, the browser's own when nothing
  // else sets them, so a page with nothing else draws nothing of its own.
  const own = ["stylesheet ", "face ", "drawing ", "image "];
  if (![...looks].some((look) => look === "masthead" || own.some((kind) => look.startsWith(kind)))) {
    broken.push("the page draws nothing of its own: no stylesheet, face, masthead or picture");
  }

  const before = globalThis.looksBefore;
  if (!Array.isArray(before)) {
    return { looks: [...looks].sort(), broken };
  }
  const reasons = globalThis.looksMeant ?? {};
  const gone = before.filter((look) => !looks.has(look));
  return {
    lost: gone.filter((look) => !(look in reasons)),
    meant: gone.filter((look) => look in reasons).map((look) => `${look}: ${reasons[look]}`),
    broken,
    looks: [...looks].sort(),
  };
})()
