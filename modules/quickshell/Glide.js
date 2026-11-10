// How long the launcher's selection, and the list's scroll with it, glides
// to its row after a key. It lands within 50 ms of the key, a short glide on
// the theme's curve, since a highlight still moving once Joel has read its
// row is lag (VKCF3Z, 2026-11-10). Windows and menus keep the theme's
// medium, 150 ms; this is the launcher's own.
.pragma library

// Two frames at the panel's 60 Hz. Under the load of 2026-11-10 the
// launcher's journal timed 429 keys to their frames: half within 12 ms, and
// nine in ten within 38 ms. A 30 ms glide from a key drawn at 12 ms lands at
// 42 ms, within the bar.
const glide = 30;

// Joel's bar: the key to the frame where the selection stands on its row.
const lands = 50;

// How many milliseconds a glide runs that starts `since` milliseconds after
// its key was pressed, on a panel whose frames come `frame` apart. Its first
// frame can wait a whole frame more, so a key already late shortens the
// glide to fit, and one too late for a frame of glide jumps: no glide beats
// a late one. A move no key pressed, or a press of unknown time, glides
// whole.
function lasting(since, frame) {
    const late = since > 0 ? since : 0;
    const left = lands - late - frame;
    const run = Math.min(glide, left);
    return run < frame ? 0 : Math.round(run);
}
