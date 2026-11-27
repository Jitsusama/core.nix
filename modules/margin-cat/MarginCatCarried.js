// How the margin cat goes with its window as niri moves it under it
// (MarginCatScreen.qml's follow, carried and rideTo): whether it rides
// niri's scroll from where it's drawn, walks on to its seat or sits there
// at once, and where a ride sets off. It's kept apart from the way
// between workspaces (MarginCatStacked.js) so the view's choices here are
// reached by Qt cases (tests/margin-cat-one-screen.qml) rather than only
// by its wiring.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatLedges.js" as Ledges

// A ride on niri's scroll to `to`, which niri set off at `told.at` on its
// `told.spring`, from where the cat was drawn as niri set it off: at `place`, short
// of it by what was left then of the ride `carry` under way, or by nothing
// (MarginCatScreen.qml's rideTo). The scroll turned back mid-way carries
// on from where the cat was as niri turned it, never from where it's drawn
// now, which would count the time since twice and set it ahead of its
// window.
function rides(carry, place, to, told) {
    const { at, spring } = told;
    const left = carry === null ? { x: 0, y: 0 }
        : MarginCat.lags(carry.from, Math.min(at - carry.at, carry.ends), carry.spring);
    const from = { x: place.x + left.x - to.x, y: place.y + left.y - to.y };
    return { from, at, ends: MarginCat.slides(spring), spring };
}

// The window whose scroll carries the cat as it waits for the keys to
// rest (MarginCatScreen.qml's carried): the one it sat on, `sat`, or, let
// go, the one it `landed` on (MarginCatGames.js's drop), and none on the
// floor, where no scroll moves it.
function carriedBy(landed, sat) {
    return landed === null ? sat : landed.id ?? null;
}

// How the cat goes back to its seat `there` on the column it sat on, as
// niri's layout has it, from its `place` (MarginCatScreen.qml's follow):
// "walks" there as its column was grown or shrunk across (`at.reshaped`),
// or as it was held short of that seat, its walk there caught by a scroll
// (`at.held`), never put there at once; "stays" where it's there already,
// a ride under way running on as its window's spring does, never set off
// again from a lag gone by, as the keys come back to a column still in
// view; "rides" with its window from where it's drawn while niri's spring
// slides the strip, a ride under way (`at.riding`) or a scroll niri told
// of within its spring (`at.scrolling`), or from wholly off the screen,
// where only a scroll leaves it; and "sits" there at once otherwise,
// nowhere before or as a floating window dragged carries it, which niri
// doesn't animate.
function backOn(place, there, at) {
    if (place === null) {
        return "sits";
    }
    if (at.reshaped || at.held) {
        return "walks";
    }
    if (Math.abs(place.x - there.x) < 0.5 && Math.abs(place.y - there.y) < 0.5) {
        return "stays";
    }
    const off = place.x + at.wide <= 0 || place.x >= at.across;
    return at.riding || at.scrolling || off ? "rides" : "sits";
}

// Whether a cat drawn at `at`, `size` {wide, tall, air}, stands on the top
// of `tile` (MarginCatTiles.js's of), its middle over it and its feet on
// it, as a walk along that top has it; not mid-leap, on a side or on
// another window's top. Only then can it ride that window and sit.
function standsOnTop(at, tile, size) {
    if (!at || !tile) {
        return false;
    }
    const middle = at.x + size.wide / 2;
    return middle >= tile.left && middle <= tile.left + tile.width
        && Math.abs(at.y - Ledges.stands(tile.top, size)) < size.air / 2;
}

// What a walk does, caught as the window it walks to, `tile` as it was,
// moves under it (MarginCatScreen.qml's follow): "holds" where it's drawn
// and rides that window, short of its seat, only standing on its top as
// niri scrolls the strip (`at.scrolling`), its spring the one the ride
// runs on; and "walks on" to that window where it moved otherwise, from
// the air of a leap or a side, or as a floating window dragged or a
// column moved beside it carries it, which no scroll of niri's draws.
function caught(drawn, tile, at) {
    return at.scrolling && standsOnTop(drawn, tile, at) ? "holds" : "walks on";
}
