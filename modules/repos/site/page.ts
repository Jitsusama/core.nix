// The page's own parts, acted on with only what each is given: finding an
// element the markup holds, writing into one, marking where the cursor is,
// and copying a folder. What the cursor is, and where, stays with site.ts.

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which Code draws into`);
  }
  return found;
}

const written = new WeakMap<Element, string>();

/**
 * Writes `html` into `into` only when it differs from what was last written there, so a record
 * that changes nothing on this part of the page leaves focus where it stood.
 */
function write(into: Element, html: string): void {
  if (written.get(into) !== html) {
    written.set(into, html);
    into.innerHTML = html;
  }
}

/** Marks the repository keyed `cursor` in the grove and the list, and brings it into view. */
function markRows(cursor: string | null, scroll: boolean): void {
  for (const each of document.querySelectorAll<HTMLElement>(
    "#grove [role=option], #list [role=option]",
  )) {
    const on = each.dataset["key"] === cursor;
    each.setAttribute("aria-selected", String(on));
    if (on) {
      each.parentElement?.setAttribute("aria-activedescendant", each.id);
      if (scroll) {
        each.scrollIntoView({ block: "nearest", inline: "nearest" });
      }
    }
  }
}

/** Marks the tree at `inner` inside `opened`, focusing it when asked. */
function markTree(opened: HTMLElement, inner: string | null, focus: boolean): void {
  for (const each of opened.querySelectorAll<HTMLElement>(".tree")) {
    const on = each.dataset["path"] === inner;
    each.setAttribute("aria-current", String(on));
    if (on && focus) {
      each.focus();
    }
  }
}

/** Copies `path` and says so on `note`, or says it whole when it can't be copied. */
function copyFolder(path: string, note: HTMLElement): void {
  navigator.clipboard.writeText(path).then(
    () => {
      note.textContent = `Copied ${path}.`;
    },
    (error: unknown) => {
      note.textContent = `Couldn't copy (${String(error)}); the folder is ${path}.`;
    },
  );
}

export { copyFolder, element, markRows, markTree, write };
