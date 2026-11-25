// What the home of the local sites answers: j and k, or the arrows, move
// the cursor from site to site, Return opens the one it's on, g twice and G
// go to the first and the last, ? and F1 list the keys, the keys its head
// names can be pressed as well as typed, and the theme switch keeps its
// look. The home loads it as /local-sites/kit/home.js; its sites are the
// links in `.sites`, and its list of keys is the dialog `#keys`. The looks
// waiting on Joel are drawn into `#asks`, answerable there (asks.ts), and
// the part holding them shows only while one waits.

import { followAsks } from "./asks.ts";
import { listen, toggle } from "./keys.ts";
import { answersAt, onTwice, pressable, stepped } from "./steps.ts";
import { themeSwitch } from "./theme.ts";

/** Whether focus is on one of the theme switch's radios, whose arrows choose a look. */
function onRadio(): boolean {
  return document.activeElement?.matches(".switch input[type=radio]") === true;
}

/** Wires the home this runs in: its cursor, its keys and its theme switch. */
function wire(): void {
  const links = [...document.querySelectorAll<HTMLAnchorElement>(".sites a")];
  const help = document.getElementById("keys");
  let at = 0;
  const show = (place: number): void => {
    at = stepped(place, 0, links.length);
    for (const [n, link] of links.entries()) {
      link.setAttribute("aria-current", String(n === at));
    }
    links[at]?.focus();
  };
  // Focus reaching a site by Tab or a click moves the cursor with it.
  addEventListener("focusin", (event) => {
    const reached = event.target instanceof Element ? event.target.closest(".sites a") : null;
    const n = links.findIndex((link) => link === reached);
    if (n >= 0 && n !== at) {
      show(n);
    }
  });
  const top = onTwice(() => show(0));
  const next = (): void => show(at + 1);
  const back = (): void => show(at - 1);
  const last = (): void => show(links.length - 1);
  // On a radio the arrows are the switch's, so they're left to it.
  const arrow = (move: () => void) => (): boolean => {
    if (onRadio()) {
      return false;
    }
    move();
    return true;
  };
  const down = arrow(next);
  const up = arrow(back);
  const actions = { j: next, k: back, ArrowDown: down, ArrowUp: up, g: top, G: last };
  if (help instanceof HTMLDialogElement) {
    listen({ help, actions, answersAt });
    pressable(".page-head .keys kbd", { j: next, k: back, "?": () => toggle(help) });
  } else {
    listen({ actions, answersAt });
    pressable(".page-head .keys kbd", { j: next, k: back });
  }
  links[0]?.setAttribute("aria-current", "true");
  themeSwitch();
  const asks = document.getElementById("asks");
  const part = asks?.closest<HTMLElement>(".waiting") ?? null;
  if (asks !== null && part !== null) {
    followAsks<Element>({
      holder: asks,
      make: (tag) => document.createElement(tag),
      counted: (count) => {
        part.hidden = count === 0;
      },
    });
  }
}

// Imported where there is no page, as a test might, it wires nothing.
if (typeof document !== "undefined") {
  wire();
}
