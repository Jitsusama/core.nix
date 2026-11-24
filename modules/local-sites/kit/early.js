// What every page of the sites' own runs before it's drawn: the mark that
// its scripts run, the look Joel last chose, and, where a page's html names
// them, the room its parts took when it was last left at this width, so it
// doesn't shift as its feeds arrive (data-room, the key the page keeps them
// under, and data-rooms, its parts' ids). It's a file of its own, loaded
// before the page's styles are read, since the server lets none of the
// sites' own pages run a script written into the page (src/http.rs says
// why). Written by hand rather than in TypeScript, since it runs as a plain
// script, which the kit's modules can't be.
"use strict";
{
  const root = document.documentElement;
  root.classList.add("js");
  try {
    const theme = localStorage.getItem("theme");
    if (theme === "light" || theme === "dark") {
      root.dataset.theme = theme;
    }
  } catch (_error) {
    // A browser that keeps nothing shows the system's theme, as it should.
  }
  const kept = root.dataset.room;
  if (kept !== undefined) {
    try {
      const room = JSON.parse(localStorage.getItem(kept) ?? "null");
      if (room?.width === innerWidth) {
        for (const id of (root.dataset.rooms ?? "").split(" ")) {
          const px = Number(room.heights?.[id]);
          if (px > 0) {
            root.style.setProperty(`--room-${id}`, `${px}px`);
          }
        }
      }
    } catch (_error) {
      // Nothing kept, or kept unreadably, holds no room: the page draws as its feeds arrive.
    }
  }
}
