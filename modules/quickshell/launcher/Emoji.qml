// Emoji, a collection: what's typed finds them by name or meaning (Emoji.js
// says how), the best shows as one row with a count of the rest, and Return
// pastes it into the window the launcher opened over, or copies it with no
// window there; a step in are Paste and Copy. The list is Unicode's, with
// CLDR's English keywords, built at build time in the skin tone `tone`
// names, and its rows are made ready by clipboard-history, which pastes
// them (clipboard/src/emoji.rs), and held over one connection to its
// socket in the ready-row form (ReadyRows.qml), so nothing here reads a
// file or names an emoji. Each act is asked as the history serves it,
// shutting the launcher as it asks, as Copies' are. Once the history has
// been silent a second, its row and its head say so, as Copies' do
// (Pasting.qml's pressing).
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Emoji.js" as Emoji

Provider {
    id: provider

    what: "emoji"

    // Found only here, by Super+E or by its name: in the main search an
    // emoji named like what was typed ("turkey" for "tu") is a row to read
    // and rule out.
    collection: Words.collection("emoji", { searched: false })
    examples: ["emoji"]

    // The history's rows of Emoji, watched whether or not the launcher's
    // open, so Super+E opens on rows already made.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`
        place: "emoji"
        make: (row, key) => provider.made(row, key)
        maker: provider
    }

    // Every emoji's row, in Unicode's order, as the history serves them.
    readonly property var all: ready.rows()
    // What Emoji.js searches, each emoji made ready to be matched once, in
    // the order of `all`.
    readonly property var found: all.map(row => row.emoji)
    // The last search, so a longer query looks only among what it found,
    // started again whenever the rows change.
    property string lastTyped: ""
    property var lastFound: []
    onFoundChanged: lastTyped = ""

    // What its row and its head say of the history that pastes emoji.
    readonly property string pressing: Pasting.pressing
    onPressingChanged: changed()
    function homeSays() {
        return pressing !== "" ? pressing : null;
    }

    // An emoji's row, from the one the history serves: named and marked
    // there, its acts done as it serves them, and the emoji itself, made
    // ready to be searched, kept on it.
    function made(served, key) {
        const held = served.held ?? {};
        const readied = ready.thing(served, key, {
            thing: true,
            key: served.thing,
            hue: served.hue ?? "text",
            // Every emoji says the same keys where it isn't selected, its
            // verb being the window under's, so the foot weighs one of a
            // run for all (Pacing.toWeigh).
            keysAlike: "emoji"
        });
        if (readied === null)
            return null;
        const row = provider.gotten(readied, { verb: () => Pasting.verb() });
        row.emoji = Emoji.prepare([held.emoji ?? "", served.name ?? "", row.keywords?.text ?? "",
            held.group ?? ""], served.order ?? 0);
        row.emoji.row = row;
        const copying = (served.acts ?? []).find(one => one.key === "verb:emoji:copy");
        return Pasting.served(row, "verb:emoji:paste", copying, held.emoji ?? "",
            one => ready.act(key, one));
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const joined = words.join(" ");
        const among = lastTyped !== "" && joined.startsWith(lastTyped) ? lastFound : found;
        const hits = Emoji.search(among, words);
        lastTyped = joined;
        lastFound = hits;
        return { rows: hits.map(one => one.row), grade: Emoji.grade(hits, words) };
    }

    // Every row, and their names, as Desktop's row for Emoji reads them for
    // its count and its drawer on every step; the names are kept while the
    // rows stand, since naming 1,914 again each step was a fifth of a step
    // into Desktop under load (2026-11-10). A plain object, so keeping them
    // notifies nothing.
    readonly property var named: ({ rows: null, names: null })
    function everything() {
        return all;
    }
    function memberNames(held = everything()) {
        if (held !== all)
            return held.map(one => one.name);
        if (named.rows !== all) {
            named.rows = all;
            named.names = all.map(one => one.name);
        }
        return named.names;
    }
}
