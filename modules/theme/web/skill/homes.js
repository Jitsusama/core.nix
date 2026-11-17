// The gate's probe for sites.md's "One Home for Every Thing" (checks.md),
// beside gate.js, layout.js and map.js and run the same way: whole, as one
// expression in the page as published. It reads every home site's
// homes.json through sites.json, then answers with each place the page names
// a thing that has a home without linking it there, each in a line saying
// what to do. An empty answer passes. It ends without a semicolon, since an
// eval asked for one expression answers nothing for a statement.
//
// A mention counts once per row, item, paragraph or heading: one link to
// the thing's home there covers every other mention beside it. Names drawn
// on a walked map ([data-map]) are the map's marks, reached by its keys, and
// a person's own words ([data-said-by]) are shown as they wrote them, so
// neither is read. A view that is a thing's home marks itself with
// data-home, its address as homes.json gives it, and the thing's own name
// needn't link inside it.
//
// A name that reads as an ordinary word, lower-case letters alone such as a
// repository called "documents" or "notes", is read only where the page sets
// it as a name: in code, kbd or samp, or in a link. In running prose it is the
// word, and "No documents yet" names no repository (2026-11-16).
//
// A check that finds nothing to test fails rather than passing, so a
// sites.json no home site answers, or a homes.json that lists nothing, is
// a fault, never a quiet pass.
(async () => {
  const missing = [];
  const page = document;
  const root = new URL("/", location.href);
  const read = async (address) => {
    try {
      const answer = await fetch(new URL(address, root), { cache: "no-store" });
      return answer.ok ? await answer.json() : null;
    } catch {
      return null;
    }
  };
  // An address is compared by its path and its fragment, each decoded, so
  // "code/#repo=Jitsusama%2Fcore.nix" and its decoded twin are one home.
  const place = (address) => {
    const url = new URL(address, root);
    return `${decodeURIComponent(url.pathname)}${decodeURIComponent(url.hash)}`;
  };

  const sites = await read("sites.json");
  if (sites === null || !Array.isArray(sites.sites)) {
    return ["sites.json at the sites' root, for the homes to be read from"];
  }
  // A row's homes are kinds, each read from its homes.json, or patterns,
  // {"kind", "address"}, giving every thing of a kind a home at the address
  // with its name, percent-encoded, for {name}. A pattern's address is a
  // fragment on the row's page, "#machine={name}", and its row's path plain
  // steps, as the system reader takes them; it reads no homes.json.
  const slot = "{name}";
  // A letter an address keeps as it is: a letter, a digit or one of -._~.
  const unreserved = (letter) =>
    (letter >= "a" && letter <= "z")
    || (letter >= "A" && letter <= "Z")
    || (letter >= "0" && letter <= "9")
    || "-._~".includes(letter);
  const plainPath = (path) =>
    typeof path === "string"
    && path !== ""
    && !path.startsWith("/")
    && path
      .split("/")
      .every((step) => step !== "." && step !== ".." && [...step].every(unreserved));
  // Visible ASCII with no quote, backslash, angle bracket or backtick, so an
  // address can't break out of a page's markup or a terminal's link.
  const plainText = (address) =>
    typeof address === "string"
    && [...address].every(
      (letter) => letter > " " && letter <= "~" && !"\\\"'<>`".includes(letter),
    );
  const plainAddress = (address) =>
    plainText(address) && address.startsWith("#") && address.includes(slot);
  const claimed = new Map();
  const things = [];
  const patterns = [];
  for (const site of sites.sites) {
    const kinds = [];
    for (const claim of site.homes ?? []) {
      const pattern = typeof claim === "object" && claim !== null;
      const kind = pattern ? claim.kind : claim;
      if (typeof kind !== "string") {
        missing.push(
          `${site.path}'s homes in sites.json hold ${JSON.stringify(claim)}: `
            + `a kind, or a pattern {"kind", "address"}`,
        );
      } else if (pattern && !(plainPath(site.path) && plainAddress(claim.address))) {
        missing.push(
          `${site.path}'s pattern for ${kind}, ${JSON.stringify(claim.address)}, `
            + `must be a fragment (#) holding ${slot} on a path of plain steps`,
        );
      } else {
        if (claimed.has(kind)) {
          missing.push(
            `${kind} has two homes, ${claimed.get(kind)} and ${site.path}: keep one in sites.json`,
          );
        }
        claimed.set(kind, site.path);
        if (pattern) {
          patterns.push({ kind, address: `${site.path}${claim.address}` });
        } else {
          kinds.push(kind);
        }
      }
    }
    if (kinds.length === 0) {
      continue;
    }
    const homes = await read(`${site.path}homes.json`);
    if (homes === null || !Array.isArray(homes.things) || homes.things.length === 0) {
      missing.push(`${site.path}homes.json, listing every thing ${site.path} is home to`);
      continue;
    }
    for (const thing of homes.things) {
      if (!kinds.includes(thing.kind)) {
        missing.push(
          `${site.path}homes.json lists a ${thing.kind}, which sites.json doesn't give it by name`,
        );
      }
      // The system reader keeps only an address on the row's own site.
      if (!(plainText(thing.address) && thing.address.startsWith(site.path))) {
        missing.push(
          `${site.path}homes.json gives ${thing.name} ${JSON.stringify(thing.address)}: `
            + `an address starts with ${site.path} and holds plain visible letters`,
        );
      }
      things.push(thing);
    }
  }

  // A pattern lists no things, so its names are learned from the page: each
  // link or data-home whose address the pattern makes names one. A mention
  // of that name elsewhere on the page is then measured like any other.
  const literal = (text) => text.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const hex = 16;
  const encode = (name) =>
    [...new TextEncoder().encode(name)]
      .map((byte) => {
        const letter = String.fromCharCode(byte);
        const coded = `%${byte.toString(hex).toUpperCase().padStart(2, "0")}`;
        return unreserved(letter) ? letter : coded;
      })
      .join("");
  const addresses = [
    ...[...page.querySelectorAll("a[href]")].map((link) => link.href),
    ...[...page.querySelectorAll("[data-home]")].map((own) => own.dataset.home),
  ];
  for (const { kind, address } of patterns) {
    const [head, ...rest] = literal(place(address.replaceAll(slot, "\u0000"))).split("\u0000");
    const made = new RegExp(`^${head}(.+?)${rest.join("\\1")}$`, "u");
    const learned = new Set();
    for (const each of addresses) {
      let name;
      try {
        name = made.exec(place(each))?.[1];
      } catch {
        // An address that won't decode names nothing a pattern makes.
      }
      if (name !== undefined && !learned.has(name)) {
        learned.add(name);
        things.push({ kind, name, names: [], address: address.replaceAll(slot, encode(name)) });
      }
    }
  }
  if (things.length === 0 && patterns.length === 0) {
    missing.push("a home site in sites.json, for any mention to be measured");
    return missing;
  }

  // Every name a page may call a thing by, longest first, so a repository's
  // whole name is read before its short name inside it.
  const byName = new Map();
  for (const thing of things) {
    for (const name of [thing.name, ...(thing.names ?? [])]) {
      if (typeof name === "string" && name.length >= 3) {
        byName.set(name, [...(byName.get(name) ?? []), thing]);
      }
    }
  }
  const names = [...byName.keys()].sort((one, other) => other.length - one.length);
  const escaped = names.map((name) => name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&"));
  // A name is part of a longer one when a letter, a slash or a dash joins it
  // on either side, a dot before it, or a dot and a letter after it. So is a
  // name just after an ellipsis (U+2026): it's the tail of a long name a page
  // shortened in its middle. One before an ellipsis still counts, since
  // prose may trail off after a whole name.
  const before = "(?<![\\p{L}\\p{N}_/.\u2026-])";
  const after = "(?![\\p{L}\\p{N}_/-]|\\.[\\p{L}\\p{N}])";
  const mention = new RegExp(`${before}(?:${escaped.join("|")})${after}`, "gu");

  const blocks = "p, li, tr, dt, dd, h1, h2, h3, h4, h5, h6, figcaption, caption, summary, "
    + "blockquote, [role='row'], [role='treeitem'], [role='listitem']";
  const skipped = "script, style, pre, title, [aria-hidden='true'], [data-map], [data-said-by], "
    + "nav.sites-strip";
  // Lower-case letters alone: each one changes when raised to a capital.
  const isWord = (name) => [...name].every((letter) => letter !== letter.toUpperCase());
  const setAsName = "code, kbd, samp, a[href]";
  const homeOf = (thing) => place(thing.address);
  const linksTo = (element, home) => {
    const link = element?.closest("a[href]");
    return link !== null && link !== undefined && place(link.href) === home;
  };

  // Each block's mentions, by name, with whether any of them links home.
  const seen = new Map();
  const walker = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const text = walker.currentNode;
    const holder = text.parentElement;
    if (holder === null || holder.closest(skipped) !== null || !holder.checkVisibility()) {
      continue;
    }
    for (const found of text.textContent.matchAll(mention)) {
      const name = found[0];
      if (isWord(name) && holder.closest(setAsName) === null) {
        continue;
      }
      const homes = byName.get(name).map(homeOf);
      const block = holder.closest(blocks) ?? holder;
      const own = holder.closest("[data-home]");
      const atHome = own !== null && homes.includes(place(own.dataset.home));
      const key = `${name}\u0000`;
      const counted = seen.get(block) ?? new Map();
      const so = counted.get(key) ?? { name, homes, linked: false, said: block };
      so.linked ||= atHome || homes.some((home) => linksTo(holder, home));
      counted.set(key, so);
      seen.set(block, counted);
    }
  }
  for (const counted of seen.values()) {
    for (const so of counted.values()) {
      if (!so.linked) {
        const where = so.said.textContent.replace(/\s+/gu, " ").trim().slice(0, 60);
        const kinds = byName.get(so.name).map((thing) => `${thing.kind} at ${thing.address}`);
        missing.push(`"${so.name}" in "${where}" names a ${kinds.join(" or ")}: link it there`);
      }
    }
  }

  // A thing that is two kinds links once from each of its homes to the other.
  for (const own of page.querySelectorAll("[data-home]")) {
    if (!own.checkVisibility()) {
      continue;
    }
    const here = place(own.dataset.home);
    const thing = things.find((each) => homeOf(each) === here);
    if (thing === undefined) {
      missing.push(
        `data-home="${own.dataset.home}" is no address any homes.json or pattern gives`,
      );
      continue;
    }
    const twins = (byName.get(thing.name) ?? []).filter((each) => homeOf(each) !== here);
    for (const twin of twins) {
      const reached = [...own.querySelectorAll("a[href]")].some(
        (link) => place(link.href) === homeOf(twin),
      );
      if (!reached) {
        missing.push(`${thing.name}'s home is also a ${twin.kind}: link once to ${twin.address}`);
      }
    }
  }
  return missing;
})()
