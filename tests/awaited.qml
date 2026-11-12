// What a search does about the answers it waits for, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-holds-a-key-for-its-answers` check copies
// modules/quickshell/launcher/Awaited.js and tests/lib/Expect.js beside
// this file and runs it offscreen. A key typed and Return pressed inside
// the few milliseconds a key is held can't be timed on niri, so each case
// here is the state Launcher.qml's searchNow finds as a key or a setter
// comes.
import QtQuick
import "Expect.js" as Expect
import "Awaited.js" as Awaited

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: {
        try {
            cases();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message ?? String(error));
        }
    }

    // A key as Launcher.qml's keysDue holds it, named by what it typed.
    function key(text) {
        return { text: text, began: 0 };
    }

    // The sources a search waits on, counted as they're asked.
    property int asked: 0
    function waitingOn(sources) {
        return () => {
            asked += 1;
            return sources;
        };
    }
    readonly property var notices: ["notices"]

    // The launcher as a search comes, with none of its keys searched yet
    // (`due`), the key held for answers or null, whether typing asked, and
    // whether a question is open.
    function at(due, held, typing, asking) {
        return { due: due, held: held, typing: typing, asking: asking };
    }

    function cases() {
        const fo = { due: [key("f"), key("o")], hold: notices };

        Expect.same("a key whose search awaits an answer is held",
            Awaited.next(at([key("f")], null, true, false), waitingOn(notices)),
            { hold: notices, due: [key("f")] });
        Expect.same("a key none waits on is searched at once",
            Awaited.next(at([key("f")], null, true, false), waitingOn([])),
            { due: [key("f")], waited: null });

        // Typed over a held key, the held one's search is dropped, and its
        // key timed by the search of the one typed over it.
        Expect.same("a key typed while one is held holds both, searching neither",
            Awaited.next(at([key("r")], fo, true, false), waitingOn(notices)),
            { hold: notices, due: [key("f"), key("o"), key("r")] });
        Expect.same("a key typed over a held one with nothing to wait on searches now",
            Awaited.next(at([key("r")], fo, true, false), waitingOn([])),
            { due: [key("f"), key("o"), key("r")], waited: null });

        // Return, or anything else that reads what was found, searches the
        // held key at once, what has answered and no more.
        Expect.same("Return while a key is held searches it now, ending its wait",
            Awaited.next(at([], fo, false, false), waitingOn(notices)),
            { due: [key("f"), key("o")], waited: fo });
        Expect.same("Return with keys unsearched searches them without holding",
            Awaited.next(at([key("f")], null, false, false), waitingOn(notices)),
            { due: [key("f")], waited: null });

        // Nothing due and nothing held, nothing is searched; a turn's end
        // with nothing new typed leaves the held key waiting.
        const before = asked;
        Expect.same("nothing typed and nothing held searches nothing",
            Awaited.next(at([], null, false, false), waitingOn(notices)), null);
        Expect.same("a turn ending with nothing new leaves the held key held",
            Awaited.next(at([], fo, true, false), waitingOn(notices)), null);
        Expect.same("a question open waits on no answer",
            Awaited.next(at([key("y")], null, true, true), waitingOn(notices)),
            { due: [key("y")], waited: null });
        Expect.expect("the sources are asked only when a key could be held",
            asked === before, `asked ${asked - before} times`);

        // A key is counted from what the field read before it, a held key's
        // text too, though it isn't searched yet: from what was last
        // searched, the held key's letters were counted again, eight
        // letters for kitty's five (80ECO2, 2026-11-12).
        Expect.same("a key comes after the held one",
            Awaited.before(at([], { due: [key("ki")] }, true, false), "k"), "ki");
        Expect.same("or after the last one due",
            Awaited.before(at([key("kit")], { due: [key("ki")] }, true, false), "k"), "kit");
        Expect.same("or, with none, after what was searched",
            Awaited.before(at([], null, true, false), "k"), "k");
    }
}
