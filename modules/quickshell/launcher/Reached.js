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

// Why an act walked from outside wasn't done, from what the launcher was
// told not done while it ran (`told`, each in the daemon's or the
// launcher's words, one told late as ""), or "" where nothing refused it.
// A yes to a secret ask walked so is withheld rather than sent
// (ReadyRows.qml's withheld), yet both walks answered it done, so the
// agent that asked waited until its ask timed out (5GLTPH, 2026-11-16).
function refused(told) {
    return told.find(why => why !== "") ?? "";
}

// What `launcher do` is answered for an act walked in `names`, from why it
// was refused (`refused`): done; unavailable where its daemon had no
// answer to give, `noAnswer` in the launcher's words, so the terminal
// exits 69 saying it may answer if asked again; else refused, 77.
function answered(why, names, noAnswer) {
    if (why === "") {
        return { done: true, in: names };
    }
    return why === noAnswer ? { unavailable: why, in: names } : { refused: why, in: names };
}
