// What keyed lists (keyed.ts) read to know a focused control again in its
// row's new drawing: what it is, what it does and what its form sends,
// read from its markup, and whether it can take a press at all. Where the
// markup can't say, it fails safe, so focus goes to the row rather than to
// another control that would take the next press.

/**
 * As much of an element as keeping focus reads: where it sits, what it is
 * and says, and focusing it.
 */
interface Placed {
  readonly parentElement: Placed | null;
  readonly children: ArrayLike<Placed>;
  readonly localName?: string;
  readonly textContent?: string | null;
  getAttribute?(name: string): string | null;
  getAttributeNames?(): string[];
  focus?(options?: FocusOptions): void;
  blur?(): void;
  checkVisibility?(options?: { readonly visibilityProperty?: boolean }): boolean;
}

/** Whether `node` is `row` or inside it. */
function inside(node: Placed | null, row: object): boolean {
  for (let at = node; at !== null; at = at.parentElement) {
    if (at === row) {
      return true;
    }
  }
  return false;
}

/** A link whose href runs a script, which says nothing of where it goes. */
const SCRIPTED = /^javascript:/iu;

/** What a browser drops from an address before reading it. */
const DROPPED = /[\t\n\r]/gu;

/** A fragment naming no element, as "#", "#!", "#0" and "#/" do, which many links share. */
const PLACEHOLDER = /^#[^a-z]*$/iu;

/**
 * What a control does, where its markup says it of this control alone:
 * where it goes, unless that's a placeholder or a script, which many
 * links share, read as a browser reads an address; else the page's own
 * act for it; else its name with its value, since a form's buttons and a
 * group's radios share a name and differ by value. Null where it says none
 * of them.
 */
function does(control: Placed): string | null {
  const href = (control.getAttribute?.("href") ?? "").replace(DROPPED, "").trim();
  if (href !== "" && !PLACEHOLDER.test(href) && !SCRIPTED.test(href)) {
    return `href ${href}`;
  }
  const act = control.getAttribute?.("data-act")?.trim() ?? "";
  if (act !== "") {
    return `act ${act}`;
  }
  const name = control.getAttribute?.("name") ?? "";
  const value = control.getAttribute?.("value") ?? "";
  return name !== "" && value !== "" ? `name ${name} ${value}` : null;
}

/**
 * The attributes saying a control's state, not which control it is: a
 * toggle pressed or ticked, a tab chosen or a button disabled while it
 * works is the same control as before. A class or an id isn't among them,
 * since a page may key a control's act on one; a page drawing new ids each
 * time sends focus to the row, which fails safe.
 */
const LOOKS = new Set([
  "aria-busy",
  "aria-checked",
  "aria-current",
  "aria-describedby",
  "aria-disabled",
  "aria-expanded",
  "aria-invalid",
  "aria-pressed",
  "aria-selected",
  "checked",
  "disabled",
  "style",
  "tabindex",
]);

/** The form a control sends, the nearest holding it. */
function formOf(control: Placed): Placed | null {
  for (let at = control.parentElement; at !== null; at = at.parentElement) {
    if (at.localName === "form") {
      return at;
    }
  }
  return null;
}

/** An element's attributes but its state, each a name and its value, in order. */
function attributesOf(element: Placed): [string, string][] {
  return (element.getAttributeNames?.() ?? [])
    .filter((name) => !LOOKS.has(name))
    .sort()
    .map((name) => [name, element.getAttribute?.(name) ?? ""]);
}

/** Whether `element` carries the attribute `name`, whatever its value. */
function carries(element: Placed, name: string): boolean {
  return (element.getAttribute?.(name) ?? null) !== null;
}

/** Fields whose value a person sets, which no markup says. */
const TYPED = new Set(["select", "textarea"]);

/** Inputs whose value is what their markup says. */
const WRITTEN = new Set(["button", "hidden", "image", "reset", "submit"]);

/**
 * Whether what `field` sends is what its markup says, not a person's
 * choice; a custom element's is what its script sets.
 */
function written(field: Placed): boolean {
  const tag = field.localName ?? "";
  if (TYPED.has(tag) || tag.includes("-")) {
    return false;
  }
  const type = (field.getAttribute?.("type") ?? "text").trim().toLowerCase();
  return field.localName !== "input" || WRITTEN.has(type);
}

/**
 * A field a form sends: its tag, name, value, type and the form it names,
 * and whether it's disabled, so not sent.
 */
type Field = [string, string, string, string | null, string | null, boolean];

/**
 * What a form sends beside `control`: each other named element in it, by
 * its tag, name and value as written, in order, since a hidden input may
 * be what carries a form's act. Of the list, whether the form holds it or
 * is it, only the control's own row counts: a row coming, going or
 * changing beside it changes nothing the control's press says. Null where
 * a field sends what a person chose, a box ticked or words typed, which no
 * markup says.
 */
function fieldsOf(form: Placed, control: Placed, list: object): Field[] | null {
  const held = Array.from(form.children);
  const fields: Field[] = [];
  for (const each of form === list ? held.filter((row) => inside(control, row)) : held) {
    const name = each.getAttribute?.("name") ?? "";
    if (each !== control && name !== "") {
      if (!written(each)) {
        return null;
      }
      const value = each.getAttribute?.("value") ?? "";
      const [type, owner] = [
        each.getAttribute?.("type") ?? null,
        each.getAttribute?.("form") ?? null,
      ];
      fields.push([each.localName ?? "", name, value, type, owner, disabled(each)]);
    }
    const inner = fieldsOf(each, control, list);
    if (inner === null) {
      return null;
    }
    fields.push(...inner);
  }
  return fields;
}

/**
 * A control's markup but its state, which says what it does and how it's
 * heard: where it goes, a target, a download, a page's own data-act and
 * class, its label and title, and the form it sends, by that form's own
 * markup but its state and what else it sends, each told apart, so no
 * value can pass for another's.
 */
function markup(control: Placed, list: object): string {
  const form = formOf(control);
  return JSON.stringify({
    form: form === null ? null : attributesOf(form),
    sends: form === null ? null : fieldsOf(form, control, list),
    control: attributesOf(control),
  });
}

/** A control as it's known again in a new drawing, read while it's in the page. */
interface Known {
  readonly localName: string | undefined;
  readonly markup: string;
  readonly does: string | null;
  readonly words: string;
  readonly apart: boolean;
}

/**
 * What `control`, in the list `list`, is, to know it again by. One labelled
 * by another element, or sending a form named by its form attribute, stands
 * apart from what it holds and what holds it, which this doesn't read; so
 * does one whose form has an id, which fields elsewhere may join it by, or
 * sends what a person chose.
 */
function knownAs(control: Placed, list: object): Known {
  const form = formOf(control);
  const named = (element: Placed, name: string) => (element.getAttribute?.(name) ?? "") !== "";
  return {
    localName: control.localName,
    markup: markup(control, list),
    does: does(control),
    words: control.textContent?.trim() ?? "",
    apart:
      named(control, "aria-labelledby") ||
      named(control, "form") ||
      (form !== null && (named(form, "id") || fieldsOf(form, control, list) === null)),
  };
}

/**
 * Whether `one` is the control `other` was: the same element with the
 * same markup but its state, which then says it does the same, so where
 * the markup says what it does (where it goes, the page's act, its name
 * and value) its words may show the record that just changed. One whose
 * markup says none of those is judged by its words too, and one with no
 * words is no control `one` can be known as. One standing apart is never
 * known again, since what it says or sends isn't its own to read here. A
 * listener no markup shows isn't seen, so a page drawing by key marks
 * each act in its controls' markup.
 */
function alike(one: Placed, other: Known | null, list: object): boolean {
  if (
    other === null ||
    one.localName !== other.localName ||
    markup(one, list) !== other.markup ||
    other.apart
  ) {
    return false;
  }
  if (other.does !== null) {
    return true;
  }
  return other.words !== "" && one.textContent?.trim() === other.words;
}

/** The form controls a disabled fieldset disables; a link or a tabindex in one isn't. */
const CONTROLS = new Set(["button", "input", "select", "textarea"]);

/**
 * Whether `field` is disabled, itself or, as a form control, by a fieldset
 * above it, so no form sends it. Marked disabled, any element counts, and
 * a fieldset's first legend isn't told apart, which both fail safe.
 */
function disabled(field: Placed): boolean {
  const tag = field.localName ?? "";
  if (carries(field, "disabled")) {
    return true;
  }
  if (!(CONTROLS.has(tag) || tag.includes("-"))) {
    return false;
  }
  for (let at = field.parentElement; at !== null; at = at.parentElement) {
    if (at.localName === "fieldset" && carries(at, "disabled")) {
      return true;
    }
  }
  return false;
}

/**
 * Whether a page lets `control` take focus by its markup alone: a link or
 * an area going somewhere, a form control but a hidden input, a summary, a
 * frame, or anything with a tabindex or editable. Anything else fails safe.
 */
function focusable(control: Placed): boolean {
  const tag = control.localName ?? "";
  const editable = (control.getAttribute?.("contenteditable") ?? "false").toLowerCase();
  if (carries(control, "tabindex") || editable !== "false") {
    return true;
  }
  if (tag === "a" || tag === "area") {
    return carries(control, "href");
  }
  if (tag === "input") {
    return (control.getAttribute?.("type") ?? "").trim().toLowerCase() !== "hidden";
  }
  return CONTROLS.has(tag) || tag === "summary" || tag === "iframe";
}

/**
 * Whether `control` takes no press though a browser may still name it
 * focused until it next draws: disabled, inert, no longer one a page lets
 * take focus, or hidden, by itself or what holds it.
 */
function refuses(control: Placed): boolean {
  // A browser's checkVisibility counts visibility: hidden only when asked,
  // and a control so hidden takes no focus as surely as one with no box.
  if (
    disabled(control) ||
    !focusable(control) ||
    control.checkVisibility?.({ visibilityProperty: true }) === false
  ) {
    return true;
  }
  for (let at: Placed | null = control; at !== null; at = at.parentElement) {
    if (carries(at, "inert")) {
      return true;
    }
  }
  return false;
}

export type { Known, Placed };
export { alike, inside, knownAs, refuses };
