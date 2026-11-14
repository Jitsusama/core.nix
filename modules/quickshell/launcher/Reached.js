// Whether a walk the launcher was asked from outside got where it went, so
// a place a terminal's `launcher do` or the catalogue's run asks open is
// said opened only where it is (Launcher.qml's doneWalked and
// catalogueRunWalked). Apart from the launcher, so tests/catalogue.qml
// holds it in Qt's own engine; Catalogue.js, beside it, is full.
.pragma library

// Whether what's held (`held`, its steps' steady ids) ends on the place,
// all of `ids` deep. A place can refuse its step, as Passwords does while
// it's walked from outside, and a row can go between the ask and the
// walk, which stops it short (Y2YBTS, 2026-11-13).
function reached(held, ids) {
    return held.length === ids.length && held.every((step, at) => step === ids[at]);
}
