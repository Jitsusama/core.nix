// Emoji, a collection: Unicode's groups with nothing typed, each stepped
// into for its emoji, and what's typed finds them by name or meaning, ranked
// by the history that serves them (clipboard/src/search.rs says how). Return
// pastes one into the window the launcher opened over, or copies it with no
// window there. The list is Unicode's, with CLDR's English keywords, built at
// build time in the skin tone `tone` names, and its rows are made ready by
// clipboard-history, which pastes them (clipboard/src/emoji.rs), and held
// over one connection to its socket in the ready-row form (ReadyRows.qml),
// so nothing here reads a file, names an emoji or matches one. An emoji's
// row is made only once it's shown, in its group or found: every one of
// 1,914 made at the shell's start held about 30 MB (2026-11-19). Each act is
// asked as the history serves it, shutting the launcher as it asks, as
// Copies' are. Once the history has been silent a second, its row and its
// head say so, as Copies' do (Pasting.qml's pressing).
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "ReadyRows.js" as Ready
import "EmojiDrawer.js" as Drawer
import "EmojiSearch.js" as Search

Provider {
    id: provider

    what: "emoji"

    // Found only here, by Super+E or by its name: in the main search an
    // emoji named like what was typed ("turkey" for "tu") is a row to read
    // and rule out.
    collection: Words.collection("emoji", { searched: false })
    examples: ["emoji"]

    // The history's rows of Emoji, watched whether or not the launcher's
    // open, so Super+E opens on rows already made. Its answer to what's
    // typed, in Emoji, in a group or in Desktop's list, is searched again
    // once it comes (Launcher.qml's foundMore), ranked as the key's own
    // search where it came within a frame of the key, as Code's are.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`
        place: "emoji"
        make: (row, key) => provider.made(row, key)
        drawerMaker: row => Drawer.made(row, provider.drawerWords)
        maker: provider
        onSearched: words => provider.foundMore(
            Date.now() - Math.max(provider.askedAt, ready.typedAt) < provider.freshFor)
    }
    property real askedAt: 0
    readonly property int freshFor: 50
    // What an emoji's drawer names its lines, made from its row as it's drawn (EmojiDrawer.js).
    readonly property var drawerWords: ({
        keywords: Words.name("emoji:keywords"),
        code: Words.name("emoji:code")
    })

    // What its row and its head say of the history that pastes emoji.
    readonly property string pressing: Pasting.pressing
    onPressingChanged: changed()
    function homeSays() {
        return pressing !== "" ? pressing : null;
    }

    // A group's row, stepped into for its emoji and searched within by the
    // history; or an emoji's, from the one the history serves: named and
    // marked there, its acts done as it serves them. A group's emoji are
    // things (nested), so a terminal names an emoji's acts as its own, and
    // selecting a group in normal mode reads none of them, which would make
    // them all (Launcher.qml's collects and readSelectedActions).
    function made(served, key) {
        if (served.kind === "emoji-group")
            return ready.thing(served, key,
                { hue: "text", nested: true, finds: provider.findsIn(key) });
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
        const row = provider.gotten(readied, { verb: pastingVerb });
        const copying = (served.acts ?? []).find(one => one.key === "verb:emoji:copy");
        return Pasting.served(row, "verb:emoji:paste", copying, held.emoji ?? "",
            actor(key));
    }

    // An emoji's verb and its act, and what's typed in a group, made apart
    // from made(), as Qt's engine keeps every argument and local a
    // closure's function has alive with it (ReadyRows.qml's thing() says
    // what it cost): one verb for every row, and an act holding only its
    // key.
    function pastingVerb() {
        return Pasting.verb();
    }
    function actor(key) {
        return one => ready.act(key, one);
    }
    function findsIn(key) {
        return letters => provider.typedInGroup(key, letters);
    }

    // What's typed in a group, as the history answers it. While the answer
    // to these letters is asked, the last answer's emoji stand only where
    // they hold every letter typed, as Emoji's own search keeps them.
    function typedInGroup(key, letters) {
        const rows = ready.typedIn(key, letters) ?? [];
        return ready.typing.words === letters ? rows : Search.holding(rows, letters);
    }

    // The emoji the history finds for what's typed, best first, and how
    // many it found, so the foot says those past the hundred it answers
    // with. While its answer is asked, what stands is EmojiSearch.js's
    // whileAsked, for the moment the answer takes, which the launcher
    // holds a key's list for a moment (`awaits`); the last answer's rows
    // are made only where what's typed goes on from it.
    function search(typed) {
        const answer = ready.search(typed);
        if (answer !== null)
            return {
                rows: ready.foundRows(answer),
                grade: answer.keys.length > 0 ? answer.grade : 3,
                found: answer.found
            };
        if (ready.asking === typed)
            provider.askedAt = Date.now();
        const goesOn = Search.goesOn(ready.found.words, typed);
        return Search.whileAsked(goesOn ? ready.foundRows(ready.found) : [], typed);
    }

    // Whether a search of these letters waits on the history's answer,
    // asked now if it isn't yet, so the launcher holds the key's list for
    // it a moment (Launcher.qml's answersWaitedFor) rather than drawing
    // none first.
    function awaits(letters) {
        if (letters.trim() === "" || ready.searchless || !ready.link)
            return false;
        return ready.search(letters) === null && ready.asking === letters;
    }

    // Its groups, as the history serves them.
    function everything() {
        return ready.rows();
    }

    // Closing lets go of what was found and of the emoji made in the groups
    // stepped into, so each is held only while a step or a search shows it
    // and the launcher's last few lists hold it (ReadyRows.qml's
    // letGoInside); the groups' own rows stand, for Super+E to open on.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                return;
            ready.ended();
            ready.letGoInside();
        }
    }

    // Every emoji's name, and how many there are, as Desktop's row for
    // Emoji reads them for its drawer and its count on every step, read
    // from the history's records rather than made rows, so none is made for
    // them. Kept while the records stand, since naming 1,914 again each step
    // was a fifth of a step into Desktop under load (2026-11-10). A plain
    // object, so keeping them notifies nothing.
    readonly property var named: ({ placings: -1, names: [] })
    function names() {
        if (named.placings !== ready.placings) {
            const held = ready.held;
            named.names = ready.keys.map(key => Ready.rowOf(held[key]))
                .filter(row => row?.kind === "emoji").map(row => row.name ?? "");
            named.placings = ready.placings;
        }
        return named.names;
    }
    function memberNames(held) {
        return names();
    }
    function counted(held) {
        return countedAs(names().length);
    }
}
