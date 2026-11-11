// The first step of the gate every one of Joel's pages passes before it's
// published (checks.md), asked of the page as published: run whole as one
// expression in the page (the browser tool's eval, or the console), and it
// answers with what the page lacks or gets wrong, each in a line saying what
// to do. An empty answer passes this step; anything else and the page isn't
// published. It finds the parts every page has, the switch and keys of its
// head, and the faults a guard on the writing catches. SKILL.md says why
// each is there; this file only finds them, so a page built by any model,
// from any first draft, is held to the same bar. It ends without a
// semicolon, since an eval asked for one expression answers nothing for a
// statement.
(async () => {
  const missing = [];
  const page = document;
  const root = page.documentElement;
  const toRead = root.dataset.kind !== "interface";

  const meta = (selector) =>
    page.querySelector(selector)?.getAttribute("content")?.trim() ?? "";
  const loads = async (address, drawing) => {
    try {
      const answer = await fetch(address, { cache: "no-store" });
      const text = await answer.text();
      return answer.ok && (drawing ? text.includes("<svg") : text.trim() !== "");
    } catch {
      return false;
    }
  };
  // A heading's name is its text nodes each trimmed and joined by a space,
  // since textContent runs one element's words into the next's ("yes3"). It
  // leaves out what's aria-hidden, as a screen reader does, so a heading drawn
  // once for the eye and once for the ear is named once ("Keys", not "Keys
  // Keys").
  const named = (element) => {
    const walker = page.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const words = [];
    while (walker.nextNode()) {
      if (walker.currentNode.parentElement.closest('[aria-hidden="true"]') === null) {
        words.push(walker.currentNode.textContent.trim());
      }
    }
    return words
      .filter((word) => word !== "")
      .join(" ")
      .replace(/ ([,.;:!?])/gu, "$1");
  };

  // The head: what names the page in a tab, a shared link and a reader.
  if (page.characterSet !== "UTF-8") {
    missing.push('<meta charset="utf-8"> first in the head, or every accent reads wrong');
  }
  if (root.lang !== "en-CA") {
    missing.push('lang="en-CA" on <html>, for Canadian English');
  }
  if (page.title.trim() === "") {
    missing.push("a <title>, the page's pithy title");
  }
  const icon = page.querySelector('link[rel~="icon"]');
  if (icon === null || !new URL(icon.href).pathname.endsWith(".svg")) {
    missing.push('a favicon of the site\'s mark: <link rel="icon" href="favicon.svg">');
  } else if (!(await loads(icon.href, true))) {
    missing.push(`a favicon that loads: ${icon.href} doesn't answer with an SVG`);
  }
  for (const [selector, part] of [
    ['meta[name="description"]', '<meta name="description">, the lede'],
    ['meta[property="og:title"]', '<meta property="og:title">, for a shared link'],
    ['meta[property="og:description"]', '<meta property="og:description">, for a shared link'],
    ['meta[property="og:type"]', '<meta property="og:type" content="website">'],
    ['meta[name="theme-color"]', '<meta name="theme-color">, the ground round the page'],
  ]) {
    if (meta(selector) === "") {
      missing.push(part);
    }
  }

  // The masthead: the site's mark, drawn inline (not one of the switch's
  // icons), one title, the key line, the switch and a skip link first.
  const head = page.querySelector("header");
  const marks = [...(head?.querySelectorAll("svg") ?? [])].filter(
    (drawing) => drawing.closest('[role="radiogroup"]') === null,
  );
  if (marks.length === 0) {
    missing.push("the site's mark, drawn as inline SVG in the first <header>");
  }
  // Only what's drawn counts, since an interface keeps its other views
  // mounted under [hidden], each with its own h1, while it shows one.
  const titles = [...page.querySelectorAll("h1")].filter((title) => title.checkVisibility()).length;
  if (titles !== 1) {
    missing.push(`one <h1>, the page's title: it has ${titles}`);
  }
  if (page.querySelector('[role="radiogroup"][aria-label="Theme"]') === null) {
    missing.push('the switch in the head: role="radiogroup" aria-label="Theme"');
  }
  // The Keys list is on the page or opens on ?, and Escape closes it with
  // focus back where it was. An interface often draws it only once ? asks,
  // so a list not shown at load is asked for the way a reader asks.
  // A page may keep a shut fold with a Keys group of its own mounted, ahead
  // of the list ? opens, so the heading shown wins over the first in the page.
  const keysHeading = () => {
    const headed = [...page.querySelectorAll("h2, h3")].filter(
      (heading) => named(heading) === "Keys",
    );
    return headed.find(shown) ?? headed[0];
  };
  const shown = (element) => element?.isConnected === true && element.checkVisibility();
  const within = (holds) =>
    new Promise((answer) => {
      const until = performance.now() + 2000;
      const look = () => {
        if (holds() || performance.now() >= until) {
          answer(holds());
        } else {
          requestAnimationFrame(look);
        }
      };
      look();
    });
  const press = (key, code) =>
    (page.activeElement ?? page.body).dispatchEvent(
      new KeyboardEvent("keydown", {
        key,
        code,
        shiftKey: key === "?",
        bubbles: true,
        cancelable: true,
        composed: true,
      }),
    );
  let keysList = keysHeading();
  if (!shown(keysList)) {
    // Asked from the first control, as a reader on the page asks, since
    // focus that starts on the body has nowhere to come back to.
    const start = page.activeElement;
    page.querySelector("a[href], button, input, select, textarea, [tabindex]")?.focus();
    const before = page.activeElement;
    press("?", "Slash");
    if (await within(() => shown(keysHeading()))) {
      keysList = keysHeading();
      // A modal <dialog> closes on a real Escape by the browser's own hand,
      // which a made-up key never triggers, so it's asked to close the way
      // that Escape asks it.
      const dialog = keysList.closest("dialog");
      if (dialog?.matches(":modal") && typeof dialog.requestClose === "function") {
        dialog.requestClose();
      } else {
        press("Escape", "Escape");
      }
      if (!(await within(() => !shown(keysList)))) {
        missing.push("Escape didn't close the Keys list");
      } else if (page.activeElement !== before) {
        missing.push("focus didn't come back after Escape");
      }
    } else {
      missing.push("a Keys list, headed Keys, that ? opens");
    }
    if (start === page.body) {
      page.activeElement?.blur();
    }
  }
  const keyLine = [...page.querySelectorAll("kbd")].find(
    (key) =>
      key.textContent.trim() === "?" &&
      key.checkVisibility() &&
      !keysList?.parentElement?.contains(key),
  );
  // It's in the head, where a reader finds it as the page opens: inside the
  // first <header>, or anywhere before <main>, never in a footer the length
  // of the page away. A key outside <main> and before it finds <main>
  // following it, and nothing more, since neither holds the other.
  const main = page.querySelector("main");
  const following = Node.DOCUMENT_POSITION_FOLLOWING;
  const upTop = (key) =>
    head?.contains(key) === true ||
    (main !== null && key.compareDocumentPosition(main) === following);
  if (keyLine === undefined) {
    missing.push("the key line under the head, naming ? for the keys");
  } else if (!upTop(keyLine)) {
    missing.push("the key line in the head, not further down the page");
  }
  const first = page.querySelector("a[href], button, input, select, textarea, [tabindex]");
  // The target is found by its id rather than as a selector, since a link to
  // a route such as "#/" is no selector at all.
  const skipsTo = first?.getAttribute("href")?.startsWith("#") ? first.getAttribute("href") : "";
  if (skipsTo.length < 2 || page.getElementById(decodeURIComponent(skipsTo.slice(1))) === null) {
    missing.push("a skip link, the first thing focus reaches, to the page's main part");
  }

  // The twin for agents.
  const twin = page.querySelector('link[rel="alternate"][type="text/markdown"]');
  if (twin === null || !(await loads(twin.href, false))) {
    missing.push('a Markdown twin that loads: <link rel="alternate" type="text/markdown">');
  }
  if (!(await loads(new URL("/llms.txt", location.href).href, false))) {
    missing.push("an llms.txt at the site's root, saying what it holds and where");
  }

  // A page to read explains with figures. Each part under a heading holds
  // a drawing (an inline SVG with role="img", as craft.md draws one) or a
  // table, or says in data-figure why it has none, as a suppression says
  // why. A page with no drawing at all, only words and tables, is a page
  // with its imagery gone, unless the page says why as a whole: a dated
  // record kept as written, its main or its article marked data-figure.
  if (toRead) {
    const pictures = "svg[role='img'], table";
    const said = (element) => element?.closest("[data-figure]")?.dataset.figure?.trim() ?? "";
    const headings = [...page.querySelectorAll("h2")].filter(
      (heading) => heading.closest("dialog, [role='dialog'], aside") === null,
    );
    for (const heading of new Set(headings)) {
      const part = heading.closest("section") ?? heading.parentElement;
      if (part.querySelector(pictures) === null && said(part) === "") {
        missing.push(`a figure for "${named(heading)}", or data-figure="why" on its section`);
      }
    }
    const whole = said(main) !== "" || said(main?.querySelector("article")) !== "";
    if (page.querySelector("svg[role='img']") === null && !whole) {
      missing.push('a drawing: no SVG with role="img", so nothing is pictured');
    }
  }

  // A drawing's labels never shrink under 12 px on screen, however narrow
  // the page (craft.md draws it again narrow, or sets its labels as HTML).
  // A label's size on screen is its font size scaled by every transform
  // above it, so a wide drawing squeezed onto a phone shows here when the
  // probe runs at 320 px; a drawing hidden at this width is skipped. It's
  // in CSS pixels rather than rem, since a page may set its root's size.
  const floor = 12;
  const onScreen = (label) => {
    const scale = label.getScreenCTM();
    const size = Number.parseFloat(getComputedStyle(label).fontSize);
    return scale === null ? size : size * Math.hypot(scale.a, scale.b);
  };
  for (const picture of [...page.querySelectorAll("svg[role='img']")].filter(shown)) {
    const smallest = Math.min(...[...picture.querySelectorAll("text")].filter(shown).map(onScreen));
    if (smallest < floor - 0.5) {
      const caption = picture.querySelector("title")?.textContent.trim() ?? "";
      const what = caption.length > 60 ? `${caption.slice(0, 57)}...` : caption || "a drawing";
      missing.push(
        `labels drawn at ${smallest.toFixed(1)} px in "${what}", under ` +
          `${floor.toFixed(1)} px: draw it again narrow, or set its labels as HTML`,
      );
    }
  }

  // A page to read sets no prose under 15 px on screen, the kit's
  // smallest step at its narrowest: a card's aside or a strip's labels
  // shrunk to fit is the same fault as a drawing's. It's measured through
  // any transform (a deck scaled to fit). Mono runs (code, keys, figures)
  // sit at 0.85 em by the kit's own optical match, and a superscript is a
  // mark, so neither counts.
  if (toRead && main !== null) {
    const least = 15;
    const probe = document.createElement("span");
    probe.style.fontFamily = "var(--mono, monospace)";
    main.append(probe);
    const mono = getComputedStyle(probe).fontFamily;
    probe.remove();
    const isMono = (element) =>
      getComputedStyle(element).fontFamily === mono ||
      element.closest("code, kbd, samp, pre, .mono, .figure, sup, sub, svg") !== null;
    const set = new Set();
    const prose = document.createTreeWalker(main, NodeFilter.SHOW_TEXT);
    for (let text = prose.nextNode(); text !== null; text = prose.nextNode()) {
      if (text.textContent.trim() !== "" && text.parentElement !== null) {
        set.add(text.parentElement);
      }
    }
    let small = null;
    for (const element of set) {
      if (shown(element) && !isMono(element) && element.offsetWidth > 0) {
        const box = element.getBoundingClientRect();
        const size =
          Number.parseFloat(getComputedStyle(element).fontSize) * (box.width / element.offsetWidth);
        if (size < least - 0.5 && (small === null || size < small.size)) {
          small = { element, size };
        }
      }
    }
    if (small !== null) {
      const words = small.element.textContent.trim().replace(/\s+/g, " ");
      missing.push(
        `prose set at ${small.size.toFixed(1)} px ("${words.slice(0, 40)}"), under ` +
          `${least.toFixed(1)} px: set it in --step-small or larger`,
      );
    }
  }

  // The writing's guard, over every word a reader reads (code, keys and
  // samples are names, not prose): no emdash or Unicode ellipsis, Canadian
  // spelling, and nothing private that an agent's work left behind. It reads
  // views kept hidden too, since each is a page the reader opens. A proper
  // name is marked translate="no", the standard's own way of saying it stays
  // as written, so a surname such as Gray is never asked to be respelled.
  // A person's own words, typed into a site that shows them as written, sit
  // in an element marked data-said-by with who said them; they're theirs,
  // never rewritten, so only the marker counts, never a bare quote, which
  // could hide a site's own prose.
  const names = '[translate="no"], [data-said-by]';
  const prose = [];
  const walker = page.createTreeWalker(page.body, NodeFilter.SHOW_TEXT);
  for (let text = walker.nextNode(); text !== null; text = walker.nextNode()) {
    const kept = `code, kbd, pre, samp, script, style, template, ${names}`;
    if (text.parentElement.closest(kept) === null) {
      prose.push(text.textContent);
    }
  }
  // A tab's title can't carry a marker, so it's read without whatever the
  // page's own marked names and words hold: an issue titled by the person
  // who filed it is drawn on the page with data-said-by, and its tab reads
  // only the site's parts around it.
  const tabTitle = [...page.querySelectorAll(names)]
    .map((element) => element.textContent.trim())
    .filter((marked) => marked !== "")
    .reduce((left, marked) => left.replaceAll(marked, " "), page.title);
  const words = `${tabTitle} ${meta('meta[name="description"]')} ${prose.join(" ")}`;
  const faults = [
    ["\u2014", "an emdash: a comma, colon or full stop instead"],
    ["\u2026", "a Unicode ellipsis: three full stops, or say it whole"],
  ];
  for (const [mark, fault] of faults) {
    if (words.includes(mark)) {
      missing.push(fault);
    }
  }
  const american = new RegExp(
    [
      "colors?",
      "colored",
      "behaviors?",
      "favorites?",
      "honors?",
      "labors?",
      "neighbors?",
      "centers?",
      "centered",
      "gray",
      "canceled",
      "canceling",
      "traveled",
      "traveling",
      "modeling",
      "labeled",
      "labeling",
      "catalogs?",
      "defense",
      "theaters?",
    ]
      .map((word) => `\\b${word}\\b`)
      .join("|"),
    "giu",
  );
  const spelled = [...new Set((words.match(american) ?? []).map((word) => word.toLowerCase()))];
  if (spelled.length > 0) {
    missing.push(`Canadian spelling for: ${spelled.join(", ")}`);
  }
  const traces = [
    ["QEST-", "a quest's id"],
    ["PLAN-", "a plan's id"],
    ["/home/", "a path on a machine"],
    ["~/.", "a path on a machine"],
  ];
  for (const [trace, what] of traces) {
    if (words.includes(trace)) {
      missing.push(`nothing private: the page names ${what} ("${trace}")`);
    }
  }

  return missing;
})()
