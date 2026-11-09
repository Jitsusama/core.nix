// What zathura's provider in the launcher works out (ZathuraProgram.qml),
// apart from the view, so tests/zathura-program.qml holds it in Qt's own
// engine: the documents each zathura has open, as open-now prints them,
// the page each is on said beside its row, and what its drawer says of
// how far through each is.
.pragma library

// A share of a whole, as how far through a document is said.
const WHOLE = 100;

// Between parts said in one place, as every side and value joins them.
const BETWEEN = " \u00b7 ";

// The documents each zathura has open, from open-now's lines, each its
// process, a tab, the document's whole path, two tabs, the page it's on,
// counted from one, a tab and how many it has: { pid, path, page, pages },
// either figure 0 where zathura didn't say it, and a line without a
// whole path passed over.
function opened(lines) {
    return lines.map(line => line.split("\t"))
        .filter(fields => (fields[1] ?? "").startsWith("/"))
        .map(fields => ({
            pid: Number(fields[0]),
            path: fields[1],
            page: Number(fields[3]) || 0,
            pages: Number(fields[4]) || 0
        }));
}

// The page a document is on, "p. 41 of 212" in `said`'s words, or ""
// where zathura didn't say.
function pageSaid(one, said) {
    if (one.page < 1 || one.pages < 1) {
        return "";
    }
    return String(said.page ?? "").replace("{page}", String(one.page))
        .replace("{pages}", String(one.pages));
}

// How far through a document is, a share of its pages to the one it's on,
// "19% through", or "" where zathura didn't say.
function shareSaid(one, said) {
    if (one.page < 1 || one.pages < 1) {
        return "";
    }
    const share = Math.round(Math.min(one.page, one.pages) / one.pages * WHOLE);
    return String(said.through ?? "").replace("{share}", String(share));
}

// What each document's row says in open's place, by its path: its page,
// for each document only one zathura has open, since two would be on two
// pages, and the row's own "open in 2 windows" says more.
function through(open, said) {
    const held = {};
    for (const one of open) {
        held[one.path] = (held[one.path] ?? 0) + 1;
    }
    const sides = {};
    for (const one of open) {
        const side = pageSaid(one, said);
        if (held[one.path] === 1 && side !== "") {
            sides[one.path] = side;
        }
    }
    return sides;
}

// What zathura's drawer in Programs says of what it has open: a fact a
// document, by its name (`named`), saying its page and how far through it
// is, "p. 41 of 212 · 19% through", or that it's open where zathura didn't
// say its page.
function facts(open, said, named) {
    return open.map(one => {
        const value = [pageSaid(one, said), shareSaid(one, said)]
            .filter(part => part !== "").join(BETWEEN);
        return { label: named(one.path), value: value === "" ? String(said.open ?? "") : value };
    });
}
