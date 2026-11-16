// mycelium's own files, for the machines it runs on, read from its site
// beside this one, again each minute while the page shows. They're
// mycelium's to keep, not net's, so they're read as files rather than
// watched; a file that can't be read is taken as saying nothing.

import { type Mycelium, myceliumOf } from "./mycelium.ts";

/** How often mycelium's files are read again while the page shows. */
const EVERY_MS = 60_000;

const beside = (file: string): Promise<unknown> =>
  fetch(`../mycelium/${file}`, { cache: "no-store" })
    .then((answer) => (answer.ok ? answer.json() : undefined))
    .catch(() => undefined);

/** Reads mycelium's files while the page shows, telling `heard` each reading. */
function followMycelium(heard: (mycelium: Mycelium) => void): void {
  let timer: ReturnType<typeof setInterval> | undefined;
  const read = (): void => {
    Promise.all([beside("now.json"), beside("homes.json")])
      .then(([cells, homes]) => heard(myceliumOf(cells, homes)))
      .catch(() => undefined);
  };
  const poll = (): void => {
    clearInterval(timer);
    timer = undefined;
    if (document.visibilityState === "visible") {
      read();
      timer = setInterval(read, EVERY_MS);
    }
  };
  document.addEventListener("visibilitychange", poll);
  poll();
}

export { followMycelium };
