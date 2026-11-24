// How the drawer lays a long page's run a part a turn while keys move on
// it and pages come, read apart from the drawing (Drawer.qml) so
// tests/query-keys.qml can count, in Qt's own engine, the parts a key lays
// and the going-ins a page shown says: costs that show nowhere else but
// as time.
.pragma library

// What a move needs laid of a run whose rest is still to come, so it never
// lands on what follows the run while that rest would come between:
// "whole" for one past the line the laid run ends on (`last`), as G's;
// "more" for one that comes within `off` lines of it, or whose view, a
// `height` tall from the move's top, reaches past that line's `top`; or
// "" for one that stays clear of it.
function reach(landed, last, edge) {
    if (landed.line > last) {
        return "whole";
    }
    if (landed.line + edge.off >= last || landed.top + edge.height > edge.top) {
        return "more";
    }
    return "";
}

// What a move does next while its run's rest is still to come, from what
// reach says the move (`landed`) needs of the run laid to `last`, in its
// view's `edge`: "whole" lays the rest at once, read once; "show" shows
// the parts laid quietly; "part" lays one part more, each read as it's
// laid; "" stops. `so` says how many parts it has laid already (`parts`)
// and whether the parts laid are still unshown (`quiet`). One that lands
// on the laid end again after a part, as G does each time, lays the rest
// whole, read once rather than some 45 times on Emoji's; a page move that
// needs a second part lays only that, since the rest laid whole in its
// key's turn would cost some 850 ms there.
function step(landed, last, edge, so) {
    const needs = reach(landed, last, edge);
    if (needs === "whole") {
        return "whole";
    }
    if (needs !== "more") {
        return "";
    }
    if (so.quiet) {
        return "show";
    }
    return so.parts > 0 && landed.line >= last ? "whole" : "part";
}

// Where a key's move lands while its run's rest is still to come, as the
// drawer's go() asks: from where `moved()` says it lands on what's laid,
// each step lays what it needs and moves again, until the run is laid or
// the move stays clear of it. `run` lays it: `laying()` says whether its
// rest is still to come, `end()` the line its laid part ends on (-1 with
// none), `edge(end)` the view's edge there and `quiet()` whether the parts
// laid are unshown; `whole()`, `show()` and `part()` do each step. The
// parts it lays are counted here, so a move landing on the laid end again
// lays the rest whole rather than a part a step.
function walk(moved, run) {
    let landed = moved();
    let parts = 0;
    for (let end = run.end(); run.laying() && end >= 0; end = run.end()) {
        const next = step(landed, end, run.edge(end), { parts, quiet: run.quiet() });
        if (next === "whole") {
            run.whole();
        } else if (next === "show") {
            run.show();
        } else if (next === "part") {
            run.part();
            parts += 1;
        } else {
            break;
        }
        landed = moved();
    }
    return landed;
}

// A page shown anew while the drawer holds the keys, laid by `lay` in a
// later turn, once Qt has laid it out. It's said at once by `coming`,
// while the key that asked it is still held, so the launcher times it from
// that key rather than from the turn's end. Pages shown before that turn
// are laid by the one lay waiting, as both would lay the page shown last
// and the launcher keeps one going-in at a time. Where `held()` says the
// keys have gone meanwhile nothing is laid, though the next page shown is.
// `waiting` keeps whether a lay waits, from one page shown to the next;
// `later` puts off a function to a later turn, as Qt.callLater does.
function anew(waiting, hooks) {
    if (waiting.lay) {
        return;
    }
    waiting.lay = true;
    hooks.coming();
    hooks.later(() => {
        waiting.lay = false;
        if (hooks.held()) {
            hooks.lay();
        }
    });
}
