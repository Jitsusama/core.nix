// Whether a card's words stay inside it, as Contained.js judges the boxes
// the launcher draws them in, run in Qt's own JavaScript engine. The
// `no-words-leave-their-room` check copies Contained.js and
// tests/lib/Expect.js beside this file and runs it offscreen.
import QtQuick
import "Contained.js" as Contained
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    readonly property var card: ({ x: 0, y: 0, width: 600, height: 400 })

    function box(x, y, width, height) {
        return { x, y, width, height };
    }

    // A text drawn in its room, the ink as wide as given.
    function text(id, said, ink, room, within) {
        return { id, said, ink, room: room ?? ink, within: within ?? [] };
    }

    function run() {
        Expect.same("words inside their room, the card and apart are no fault",
            Contained.faults([text(1, "Home", box(20, 10, 80, 20)),
                text(2, "read", box(500, 10, 60, 20))], card), []);
        Expect.same("ink run past the box it's laid in, uncut, is past its room",
            Contained.faults([text(1, "a long name", box(20, 10, 300, 20),
                box(20, 10, 120, 20))], card),
            [{ said: "a long name", fault: "past its room", with: "" }]);
        Expect.same("ink laid in a box as wide as itself, past the card's edge, is past the card",
            Contained.faults([text(1, "why did this window", box(400, 10, 500, 20))], card),
            [{ said: "why did this window", fault: "past the card", with: "" }]);
        Expect.same("lines below the card's foot are past it too",
            Contained.faults([text(1, "Attached:", box(20, 380, 100, 60))], card),
            [{ said: "Attached:", fault: "past the card", with: "" }]);
        Expect.same("two texts' ink on each other is over, said once",
            Contained.faults([text(1, "You Said", box(20, 50, 100, 20)),
                text(2, "t our agentic harness", box(0, 52, 560, 20))], card),
            [{ said: "You Said", fault: "over", with: "t our agentic harness" }]);
        Expect.same("a text held inside another, as a field's hint, is never over it",
            Contained.faults([text(1, "", box(20, 10, 300, 20)),
                text(2, "Say anything", box(20, 10, 120, 20), box(20, 10, 300, 20), [1])],
                card), []);
        Expect.same("ink meeting another's at an edge, or by a bearing's pixel, is no fault",
            Contained.faults([text(1, "Home", box(20, 10, 81, 20)),
                text(2, "Agents", box(100, 10, 80, 20)),
                text(3, "below", box(20, 29, 80, 20))], card), []);
        Expect.same("a bearing's pixel past its room or the card is no fault",
            Contained.faults([text(1, "W", box(-1, 0, 22, 20), box(0, 0, 20, 20))], card), []);
        lines();
    }

    // A text set at a fixed pitch, in a face of the given ascent and
    // descent, its first baseline where Qt puts it: at the face's ascent,
    // the pitch's spare room under its letters.
    function setIn(lines, ascent, descent) {
        return { fixed: true, lines, pitch: 24, baseline: ascent, ascent, descent, tall: 0 };
    }

    // How tall the letters of a text's lines stand, and from how far down
    // its box (Contained.inkedLines).
    function lines() {
        // A drawer's You Asked in the Note face over its prose in the words
        // face, each line set on its baseline a pitch apart: the prose's
        // box starts 3.7 px inside the name's pitch, yet its letters stand
        // clear of the name's (the session's drawer, 2026-11-16).
        const name = Contained.inkedLines(setIn(1, 14, 3.5));
        const prose = Contained.inkedLines(setIn(12, 17.75, 4.5));
        Expect.same("a line's letters stand from its top to its descent", name,
            { top: 0, height: 17.5 });
        Expect.same("many lines' letters end at the last one's descent", prose,
            { top: 0, height: 11 * 24 + 17.75 + 4.5 });
        const under = 24 + 14 - 17.75;
        Expect.same("a value set a pitch under its name, in a taller face, isn't over it",
            Contained.faults([text(1, "You Asked", box(25, 94.5, 81, name.height)),
                text(2, "why did", box(25, 94.5 + under, 237, prose.height))], card), []);
        Expect.same("a value set on its name's own baseline is over it",
            Contained.faults([text(1, "You Asked", box(25, 94.5, 81, name.height)),
                text(2, "why did", box(25, 94.5 + 14 - 17.75, 237, prose.height))], card),
            [{ said: "You Asked", fault: "over", with: "why did" }]);
        Expect.same("a text set in its face's own lines is as tall as they are",
            Contained.inkedLines({ fixed: false, lines: 2, pitch: 0, baseline: 14, ascent: 14,
                descent: 4, tall: 36 }), { top: 0, height: 36 });
    }
}
