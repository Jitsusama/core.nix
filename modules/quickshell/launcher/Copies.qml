// The clipboard's history, a collection: every copy made this session, text
// and pictures, newest first, their rows made ready by clipboard-history
// (clipboard/src/main.rs and src/rows.rs) and held over one connection to
// its socket in the ready-row form (ReadyRows.qml), so nothing is started
// per copy, nothing is written to disk, and nothing here sorts or words a
// copy. A search with a match adds one row; the right arrow, or Super+V,
// shows them all. A text copy's row runs its lines together, cut short,
// and the selected one reads whole below the rows. A picture's row is
// named by its size, and the launcher shows the picture itself while it's
// selected (`picture`, its file). Return pastes the copy into the window
// the launcher opened over: the history, asked for it by its number, puts
// it back on the clipboard and, once the window has the keys again,
// presses that window's paste keys for it, nothing of it waiting on the
// shell: the act is asked as the history serves it, shutting the launcher
// (ReadyRows.qml's shut), and a paste refused is told as the launcher next
// opens. Super+E's emoji paste the same way, served by the same history
// (Emoji.qml), and Pasting.qml asks it as its helper. With no window
// there, Return only copies, as the right arrow's Copy always does.
// Forget the Clipboard, the clipboard's last row while there's something
// to forget and found by its words from anywhere, asks the history to
// forget every copy, and what the clipboard holds with them. Once the
// history has been silent a second, its row says "not answering" where it
// counts the copies, and its head says so while it's entered, as People's
// does; a paste that never landed is said there the same way (Pasting.qml's
// pressing). Each copy's row keeps its own note.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Order.js" as Order

Provider {
    id: provider

    what: "copy"

    collection: Words.collection("clipboard")

    // The history's rows of Copies, watched whether or not the launcher's
    // open, so it opens on rows already held.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`
        place: "copies"
        beside: ["summary"]
        make: (row, key) => provider.made(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
        onHeard: (changed, gone) => provider.heard(changed, gone)
    }

    // What was made of each copy, by its key, with the row it was made
    // from. Every copy says when it came, so every opening lets go of them
    // all to say it anew (ReadyRows' turned), and remaking 200 was most of
    // Super+V's opening; a row the history hasn't changed keeps what was
    // made of it, its side said again in place, as Copies' ages always
    // were. A plain object, so keeping them notifies nothing.
    readonly property var makings: ({})
    readonly property string summaryKey: "summary:copies"

    // Newest first, as the history serves them.
    readonly property var all: ready.rows()

    // Whether the history is connected, to copy and paste for Pasting.qml.
    readonly property bool linked: (ready.link?.wire ?? null) !== null

    Component.onCompleted: Pasting.helper = provider

    // Asks the history, as Daemon.qml asks; `then` is given its answer, or
    // null while it isn't there to ask.
    function ask(said, then) {
        const link = ready.link;
        if (link)
            link.ask(said, then);
        else if (then)
            then(null);
    }

    // What its row and its head say of the history, ahead of the count.
    readonly property string pressing: Pasting.pressing
    onPressingChanged: changed()
    function homeSays() {
        return pressing !== "" ? pressing : null;
    }

    readonly property var forget: thing({
        key: "copies:forget",
        // An act, not a copy, so it says no kind (docs/design.md).
        what: "",
        verb: Words.verb("copies:forget"),
        name: Words.name("copies:forget"),
        glyph: Words.glyph("copies:forget"),
        hue: "action",
        run: () => provider.ask({ ask: "clear" }, answer => {
            if (answer?.cleared === true)
                Report.say("forgot the copies");
        })
    })

    things: all.length > 0 ? [forget] : []

    // A copy's row, from the one the history serves: worded there, and its
    // acts done as it serves them, by the reader every place's served rows
    // share (ReadyRows.qml's thing), each shutting the launcher as it asks.
    function made(served, key) {
        const was = makings[key];
        if (was !== undefined && was.served === served) {
            was.row.side = ready.sideOf(served);
            was.row.sideItems = ready.itemsOf(served);
            return was.row;
        }
        const held = served.held ?? {};
        const picture = served.kind === "picture";
        const fields = {
            thing: true,
            key: served.thing,
            glyph: Theme.glyph[served.glyph] ?? Theme.glyph.clipboard,
            hue: served.hue ?? "text"
        };
        const kept = picture ? {
            id: held.id,
            picture: held.picture,
            across: held.across ?? 0,
            down: held.down ?? 0,
            // Its type and weight, its side but for when it was copied, for
            // GIMP's and Inkscape's acts on it (CopiedPicture.qml).
            about: ready.sideOf({ side: (served.side ?? []).filter(item => item.at === undefined) })
        } : {
            text: held.text,
            // As it was copied, lines and all, which the row cuts short.
            reads: held.reads
        };
        const readied = ready.thing(served, key, Object.assign(fields, kept));
        if (readied === null)
            return null;
        // What y joins with other rows' copies is the text, for a copy
        // that's one, and nothing for a picture, which y copies alone. Its
        // verb a getter, made afresh with it (Provider.qml's gotten), so no
        // field is lost.
        const copying = (served.acts ?? []).find(one => one.key === "verb:copy:copy");
        const row = Pasting.served(provider.gotten(readied, { verb: () => Pasting.verb() }),
            "verb:copy:paste", copying, readied.text ?? "", one => ready.act(key, one));
        makings[key] = { served, row };
        return row;
    }

    // Each copy heard, said for the checks that wait on it: a picture by
    // its name, and text by its first line.
    function heard(changed, gone) {
        // Its lists come through the signal converted, so they're read by
        // index, as ReadyRows reads Daemon's. A changed row is made anew
        // by its record being new (made), so only what went is let go.
        for (let at = 0; at < gone.length; at += 1)
            delete makings[gone[at]];
        for (let at = 0; at < changed.length; at += 1) {
            const key = changed[at];
            const row = ready.served(key);
            if (!row)
                continue;
            const said = row.kind === "picture" ? row.name
                : String(row.held?.text ?? "").split("\n")[0];
            Report.say(`heard copy ${said}`);
        }
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(all, typed)) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        matchedIn(all, typed, found.map(f => f.row));
        const asked = all.length > 0 ? Fuzzy.match(forget.prepared, words, false)
            : { score: -1 };
        // Newest first among equals, not shortest, since a copy is wanted
        // by when it was made: the history's order, which a stable sort
        // keeps.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        if (asked.score >= 0)
            finest = Math.min(finest, asked.grade);
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score).map(f => f.row);
        const forgets = asked.score >= 0 && asked.grade === finest;
        return { rows: forgets ? kept.concat([forget]) : kept, grade: finest };
    }

    // The clipboard holds its own Forget, after every copy, so stepping in
    // lands on the newest copy, never on forgetting them all, as Wi-Fi
    // holds its off (collection-order.md).
    function everything() {
        return all.length > 0 ? all.concat([forget]) : [];
    }

    // How many copies it keeps, its Forget not among them, as the history
    // says it.
    function counted() {
        void ready.placings;
        return String(ready.held[summaryKey]?.summary?.said ?? countedAs(all.length));
    }
}
