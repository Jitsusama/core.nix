// The keys a live page answers, said once for every page. None answer while
// a modifier is held, since those are the browser's and the desktop's, or
// where Joel types, whose keys are the field's. `?` and F1 open and close the
// page's list of its keys wherever focus is. The rest are the page's own,
// looked up in its table, and give way on a control with keys of its own.

/**
 * What a page does for a key, by its `KeyboardEvent.key`. Returning `false`
 * declines, leaving the key to the browser: Return on a link the page's
 * cursor is on follows it.
 */
type Action = () => unknown;

/** Where Joel types: any field taking words, but not a radio or a checkbox. */
const TYPED = 'input:not([type="radio"], [type="checkbox"]), textarea, [contenteditable]';

/** What a page does for each key it answers. */
type Actions = { readonly [key: string]: Action };

/** A key pressed, as far as choosing what it does goes. */
interface Press {
  readonly key: string;
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  /** Whether it was pressed where Joel types. */
  readonly typing: boolean;
}

/** What a press asks for: the page's keys, one of its actions, or nothing. */
type Chosen = "help" | Action | undefined;

/** The keys that open the page's list of its keys, as `less` and a desktop do. */
const HELP_KEYS: ReadonlySet<string> = new Set(["?", "F1"]);

/**
 * What `press` asks of a page whose keys are `actions`. A key the page
 * doesn't name is nothing, even one an object answers to, such as
 * `constructor`.
 */
function chosen(press: Press, actions: Actions): Chosen {
  if (press.altKey || press.ctrlKey || press.metaKey || press.typing) {
    return undefined;
  }
  if (HELP_KEYS.has(press.key)) {
    return "help";
  }
  return Object.hasOwn(actions, press.key) ? actions[press.key] : undefined;
}

/**
 * Whether `target` is somewhere Joel types. A radio or a checkbox takes no
 * letters, so the page's keys still answer there.
 */
function typing(target: Element): boolean {
  return target.closest(TYPED) !== null;
}

/** Whether `target` is a control whose own keys come before the page's. */
function hasOwnKeys(target: Element): boolean {
  return target.closest("a, button, select, summary") !== null;
}

/** What a page hands `listen`. */
interface Listening {
  /**
   * The dialog listing the page's keys, which `?` and F1 open and close. A
   * page without one still answers its own keys.
   */
  readonly help?: HTMLDialogElement;
  /** What the page's own keys do. */
  readonly actions: Actions;
  /**
   * Whether the page's own keys answer at `target`: by default anywhere but
   * on a control with keys of its own.
   */
  readonly answersAt?: (target: Element) => boolean;
  /** What follows any of the page's own keys it took, such as focus moving back. */
  readonly afterwards?: () => void;
}

/** Opens `dialog` as a modal when it's shut, and shuts it when it's open. */
function toggle(dialog: HTMLDialogElement): void {
  if (dialog.open) {
    dialog.close();
  } else {
    dialog.showModal();
  }
}

/** Answers the page's keys as `listening` says, until the function returned is called. */
function listen(listening: Listening): () => void {
  const answersAt = listening.answersAt ?? ((target: Element) => !hasOwnKeys(target));
  const pressed = (event: KeyboardEvent): void => {
    const target = event.target instanceof Element ? event.target : document.body;
    const { key, altKey, ctrlKey, metaKey } = event;
    const choice = chosen(
      { key, altKey, ctrlKey, metaKey, typing: typing(target) },
      listening.actions,
    );
    if (choice === undefined) {
      return;
    }
    if (choice === "help") {
      if (listening.help) {
        toggle(listening.help);
        event.preventDefault();
      }
      return;
    }
    if (listening.help?.open === true || !answersAt(target) || choice() === false) {
      return;
    }
    event.preventDefault();
    listening.afterwards?.();
  };
  addEventListener("keydown", pressed);
  return () => removeEventListener("keydown", pressed);
}

export type { Action, Actions, Chosen, Listening, Press };
export { chosen, hasOwnKeys, listen, toggle };
