// What every one of Joel's pages holds, asked of a page as published: run
// whole as one expression in the page (the browser tool's eval, or the
// console), and it answers with what the page lacks, each in a line saying
// what to add. An empty answer is a page with all its parts; a page that
// lacks any is unfinished, and isn't published. SKILL.md says why each part
// is there. This file only finds them, so a page built by any model, from
// any first draft, is held to the same parts. It ends without a semicolon,
// since an eval asked for one expression answers nothing for a statement.
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

  // The site's mark, drawn in the head: an inline drawing that isn't one of
  // the switch's icons, the same drawing as the favicon.
  const head = page.querySelector("header");
  const marks = [...(head?.querySelectorAll("svg") ?? [])].filter(
    (drawing) => drawing.closest('[role="radiogroup"]') === null,
  );
  if (marks.length === 0) {
    missing.push("the site's mark, drawn as inline SVG in the first <header>");
  }

  if (page.querySelector('[role="radiogroup"][aria-label="Theme"]') === null) {
    missing.push('the switch in the head: role="radiogroup" aria-label="Theme"');
  }

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
  // with its imagery gone.
  if (toRead) {
    const pictures = "svg[role='img'], table";
    const headings = [...page.querySelectorAll("h2")].filter(
      (heading) => heading.closest("dialog, [role='dialog'], aside") === null,
    );
    for (const heading of new Set(headings)) {
      const part = heading.closest("section") ?? heading.parentElement;
      const why = part.closest("[data-figure]")?.dataset.figure?.trim() ?? "";
      if (part.querySelector(pictures) === null && why === "") {
        const name = heading.innerText.trim().replaceAll("\n", " ");
        missing.push(`a figure for "${name}", or data-figure="why" on its section`);
      }
    }
    if (page.querySelector("svg[role='img']") === null) {
      missing.push('a drawing: no SVG with role="img", so nothing is pictured');
    }
  }

  return missing;
})()
