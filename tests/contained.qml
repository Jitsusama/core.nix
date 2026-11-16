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
    }
}
