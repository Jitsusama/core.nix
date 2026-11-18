// How On the Selection says the selected text at its head, names an
// answer, keeps pi-verbs' order and words an answer's state, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-heads-the-selection-at-a-whole-word` check copies
// modules/quickshell/launcher/OnSelection.js and tests/lib/Expect.js beside
// this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/OnSelection.js" as OnSelection

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

    readonly property var said: ({ asking: "thinking", answering: "answering",
        answered: "answered; Return copies it", copied: "copied; paste it over the selection",
        failed: "it couldn't answer" })

    function cases() {
        Expect.same("a short selection is its head as it is",
            OnSelection.headOf("the build failed", 60), "the build failed");
        Expect.same("lines and runs of spaces read as one line",
            OnSelection.headOf("the build\n\n  failed\tagain", 60), "the build failed again");
        Expect.same("a long one gives way at the last whole word, its comma dropped",
            OnSelection.headOf("error: the derivation failed, see the log for more", 30),
            "error: the derivation failed\u2026");
        Expect.same("a first word past the room is cut where the room ends",
            OnSelection.headOf("aaaaaaaaaaaaaaaaaaaa", 8), "aaaaaaaa\u2026");
        Expect.same("nothing selected has no head", OnSelection.headOf("", 60), "");

        Expect.same("an answer is named by its first sentence",
            OnSelection.firstSentence("It means the build failed. Read the log."),
            "It means the build failed.");
        Expect.same("a version number isn't a sentence's end",
            OnSelection.firstSentence("Use 1.5 or later. Then retry."), "Use 1.5 or later.");
        Expect.same("one with no stop is named whole",
            OnSelection.firstSentence("still answering"), "still answering");

        const held = { "verb:explain": { key: "verb:explain" },
            "verb:fix": { key: "verb:fix" }, "verb:hand": { key: "verb:hand" } };
        Expect.same("records keep the order pi-verbs listed them in, those told of after last",
            OnSelection.served(["verb:fix", "verb:explain"], held).map(one => one.key),
            ["verb:fix", "verb:explain", "verb:hand"]);
        Expect.same("a record listed and gone since isn't served",
            OnSelection.served(["verb:gone", "verb:fix"], { "verb:fix": { key: "verb:fix" } })
                .map(one => one.key),
            ["verb:fix"]);

        Expect.same("an answer replacing the selection says it was copied",
            OnSelection.noted("answered", "replace", said), said.copied);
        Expect.same("another answered says Return copies it",
            OnSelection.noted("answered", "say", said), said.answered);
        Expect.same("one stopped says it couldn't answer",
            OnSelection.noted("stopped", "say", said), said.failed);
        Expect.same("one being written says so", OnSelection.noted("answering", "say", said),
            said.answering);
        Expect.same("one asked says it's thinking", OnSelection.noted("asking", "say", said),
            said.asking);
    }
}
