// A cell named the one way every site names it: by the address the mesh gives
// its session, in the link the desktop hands the launcher,
// `mycelium:cell/<ADDRESS>@<machine>`, which modules/quickshell/mycelium-link
// reads and a notice's whole cell link already says. A page writes that link
// for a cell, and reads a cell out of it, or out of its address alone, with or
// without the machine, a step or an anchor after it, so a link copied from a
// notice opens the cell a page's own link does. The letters it lets through
// are mycelium-link's, so a page never offers a link that does nothing.

/** The scheme a link to anything of the mycelium starts with, in any case. */
const SCHEME = "mycelium:";

/** The kind of thing a cell's link names, after the scheme. */
const CELL = "cell/";

/** A session's address on the mesh: capitals and digits, twelve at most. */
const ADDRESS = /^[A-Z0-9]{1,12}$/u;

/** A machine's name: letters, digits, dots, hyphens and underscores, a letter or digit first. */
const MACHINE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/u;

/** What may follow a cell's name in its link: a step such as `/go`, a query or an anchor. */
const AFTER = /[/?#]/u;

/** The slashes a page may put after the scheme, as mycelium-link lets go. */
const SLASHES = /^\/+/u;

/**
 * The address `named` gives: a cell's link, `mycelium:cell/<ADDRESS>`, or the
 * address alone, either with `@<machine>` and anything after it. None for a
 * link to anything else, or where the address or the machine is misshaped.
 */
function addressIn(named: string): string | null {
  let rest = named;
  if (rest.slice(0, SCHEME.length).toLowerCase() === SCHEME) {
    rest = rest.slice(SCHEME.length).replace(SLASHES, "");
    if (!rest.startsWith(CELL)) {
      return null;
    }
    rest = rest.slice(CELL.length);
  }
  const [name = ""] = rest.split(AFTER);
  const at = name.indexOf("@");
  const address = at === -1 ? name : name.slice(0, at);
  const placed = at === -1 || MACHINE.test(name.slice(at + 1));
  return placed && ADDRESS.test(address) ? address : null;
}

/**
 * The link to talk to the cell at `address`, saying the machine it runs on
 * where that has a name a link may carry, or none where the address isn't
 * one the launcher could reach.
 */
function cellLink(address: string, machine?: string | null): string | null {
  if (!ADDRESS.test(address)) {
    return null;
  }
  const on = machine !== undefined && machine !== null && MACHINE.test(machine);
  return on ? `${SCHEME}${CELL}${address}@${machine}` : `${SCHEME}${CELL}${address}`;
}

export { addressIn, cellLink };
