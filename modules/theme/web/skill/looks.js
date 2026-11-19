// The gate's probe for checks.md's "A deploy keeps its looks", beside
// gate.js, layout.js, map.js and homes.js and run the same way: whole, as
// one expression in the page. It lists what the page draws beyond its
// words, measured from the render rather than read from the markup, since a
// stylesheet that answers 404 or a face that never loads is still in the
// markup: each stylesheet that loaded with rules in it, each face loaded,
// the type the body and the title are set in, the page's colours, the
// masthead and its mark, each drawing, canvas and image drawn, each picture
// a stylesheet paints (a background, a border, a ::before), and each
// script. It ends without a semicolon, since an eval asked for one
// expression answers nothing for a statement.
//
// Run on the live page before a deploy, it answers { looks, broken }: keep
// that answer whole. Run on the live page after it, with that answer set
// first as `globalThis.looksBefore`, it answers { lost, meant, broken,
// still, now }: each look the page had and has no more, each it was told
// was meant to go, by `globalThis.looksMeant`, an object from a look to why
// it went, each asset that answered an error and didn't before, each that
// answered one before too, and the answer the page gives now, the next
// deploy's before. An empty `lost` and `broken` passes; an answer with no
// `lost` in it is a before run, never a pass, and a `looksBefore` set to
// anything but a before run's answer is refused rather than read as none.
// On 18 November 2026 a deploy of a tracker lost its guide's masthead,
// heading art and stylesheets while every word check passed, and Joel found
// it himself: this is the check that would have stopped it.
//
// A look is a "kind value" string, so two runs compare as sets. An asset is
// named by its path from the page's folder, or from the site's root when
// it's above it, or by its address without a query when it's on another
// origin, and a part of its name that's a build's content hash reads as *,
// so a rebuilt asset keeps its name. A drawing is named by its data-drawing
// or id, else by its place under the nearest element with an id, never by
// its words, since a change of words isn't a lost look. Every face the page
// declares is loaded, since one is fetched only once something is drawn in
// it, which can come after the page's load. Folds are opened to measure
// what's in them, and closed again, so run it on a page nobody is reading.
// What a browser keeps from another origin's answers, it keeps from the
// page too: an asset from another origin that failed may read as loaded,
// so the shots beside this are what see it. A page with no stylesheet,
// face, masthead or picture of its own is a fault, never a quiet pass.
(async () => {
  // Past fifteen seconds, whatever hasn't loaded is taken as it stands.
  const patience = 15_000;
  const wait = (promise) =>
    Promise.race([promise, new Promise((late) => setTimeout(late, patience))]);
  const before = globalThis.looksBefore;
  if (before !== undefined && !(Array.isArray(before?.looks) && Array.isArray(before?.broken))) {
    throw new Error("looksBefore is set, but not to the answer of a run before the deploy");
  }
  if (document.readyState !== "complete") {
    await wait(new Promise((loaded) => addEventListener("load", loaded, { once: true })));
  }

  const folder = new URL("./", location.href).href;
  const digits = "0123456789";
  const hashLength = 8;
  // A run of letters and digits long enough, all hexadecimal and holding a
  // digit, is a build's content hash, as app.3f9a2c1b.js and site-8c1e04d2.css
  // are named, and reads as *.
  const hashed = (run) =>
    run.length >= hashLength &&
    [...run].every((letter) => !Number.isNaN(Number.parseInt(letter, 16))) &&
    [...run].some((letter) => digits.includes(letter));
  const steady = (path) => {
    const cut = path.lastIndexOf("/") + 1;
    let file = "";
    let run = "";
    for (const letter of `${path.slice(cut)}/`) {
      const word = letter.toLowerCase() !== letter.toUpperCase() || digits.includes(letter);
      if (word) {
        run += letter;
      } else {
        file += (hashed(run) ? "*" : run) + letter;
        run = "";
      }
    }
    return path.slice(0, cut) + file.slice(0, -1);
  };
  const named = (address) => {
    const url = new URL(address, location.href);
    const path = `${url.origin}${url.pathname}`;
    if (url.origin !== location.origin) {
      return steady(path);
    }
    return steady(path.startsWith(folder) ? path.slice(folder.length) : url.pathname);
  };
  const unquoted = (name) => name.replaceAll('"', "").replaceAll("'", "").trim();
  const drawn = (element) => {
    const box = element.getBoundingClientRect();
    return box.width > 0 && box.height > 0;
  };
  // Each address a computed value names in url(...), as a background or a
  // border image is written.
  const addresses = (value) =>
    value
      .split("url(")
      .slice(1)
      .map((rest) => unquoted(rest.slice(0, rest.indexOf(")"))))
      .filter((address) => address !== "" && !address.startsWith("data:"));

  // A fold in an exclusive group shuts the others as it opens, so the
  // group's name is set aside while every fold is open, and given back.
  const shut = [...document.querySelectorAll("details:not([open])")];
  const grouped = [...document.querySelectorAll("details[name]")].map((fold) => [
    fold,
    fold.getAttribute("name"),
  ]);
  for (const [fold] of grouped) {
    fold.removeAttribute("name");
  }
  for (const fold of shut) {
    fold.open = true;
  }
  const images = [...document.images];
  for (const image of images) {
    image.loading = "eager";
  }
  await wait(Promise.allSettled(images.map((image) => image.decode())));
  // One face that can't be loaded is lost, whenever its first use comes.
  await wait(Promise.allSettled([...document.fonts].map((face) => face.load())));

  const looks = new Set();
  const broken = [];
  const failing = 400;
  for (const entry of performance.getEntriesByType("resource")) {
    if (entry.responseStatus >= failing) {
      broken.push(`${named(entry.name)} answered ${entry.responseStatus}`);
    }
  }
  for (const script of [...document.scripts].filter((each) => each.src !== "")) {
    looks.add(`script ${named(script.src)}`);
  }
  for (const sheet of document.styleSheets) {
    let rules = 0;
    try {
      rules = sheet.cssRules.length;
    } catch {
      // A sheet from another origin keeps its rules to itself; that it
      // loaded at all is what can be said of it.
      rules = 1;
    }
    if (rules > 0) {
      looks.add(sheet.href === null ? "stylesheet inline" : `stylesheet ${named(sheet.href)}`);
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
  const family = (element) => unquoted(getComputedStyle(element).fontFamily.split(",")[0]);
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
    if ([...masthead.querySelectorAll("svg, img, canvas")].some(drawn)) {
      looks.add("mark");
    }
  }
  const pictures = "svg, canvas";
  const topmost = (picture) => picture.parentElement?.closest("svg") === null;
  const nameOf = (picture) => {
    const own = picture.dataset.drawing || picture.id;
    if (own) {
      return own;
    }
    const place = picture.parentElement?.closest("[id]");
    const among = [...(place ?? document.body).querySelectorAll(pictures)].filter(topmost);
    return `${place === null ? "page" : `#${place.id}`} ${among.indexOf(picture) + 1}`;
  };
  for (const picture of document.querySelectorAll(pictures)) {
    if (topmost(picture) && drawn(picture)) {
      looks.add(`drawing ${nameOf(picture)}`);
    }
  }
  for (const image of images) {
    if (image.complete && image.naturalWidth > 0) {
      looks.add(`image ${named(image.src)}`);
    }
  }
  const painted = ["backgroundImage", "borderImageSource", "listStyleImage", "maskImage"];
  for (const element of [...document.body.querySelectorAll("*")].filter(drawn)) {
    for (const part of [null, "::before", "::after"]) {
      const style = getComputedStyle(element, part);
      for (const address of painted.flatMap((property) => addresses(style[property]))) {
        looks.add(`art ${named(address)}`);
      }
    }
  }
  for (const fold of shut) {
    fold.open = false;
  }
  for (const [fold, group] of grouped) {
    fold.setAttribute("name", group);
  }
  // The type and colours are always there, the browser's own when nothing
  // else sets them, so a page with nothing else draws nothing of its own.
  const own = ["stylesheet ", "face ", "drawing ", "image ", "art "];
  if (![...looks].some((look) => look === "masthead" || own.some((kind) => look.startsWith(kind)))) {
    broken.push("the page draws nothing of its own: no stylesheet, face, masthead or picture");
  }

  const now = { looks: [...looks].sort(), broken };
  if (before === undefined) {
    return now;
  }
  const reasons = globalThis.looksMeant ?? {};
  const gone = before.looks.filter((look) => !looks.has(look));
  return {
    lost: gone.filter((look) => !(look in reasons)),
    meant: gone.filter((look) => look in reasons).map((look) => `${look}: ${reasons[look]}`),
    broken: broken.filter((fault) => !before.broken.includes(fault)),
    still: broken.filter((fault) => before.broken.includes(fault)),
    now,
  };
})()
