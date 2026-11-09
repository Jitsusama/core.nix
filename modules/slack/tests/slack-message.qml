// A Slack message's one row, as SlackMessage.js works it out, run in Qt's
// own JavaScript engine, the one the shell runs it in: one key for a
// message whether held or found, named by who said it, its note its first
// line or, found past it, from the matched word on, where it is in its
// eyebrow, and Return opening the message itself. The
// `slack-message-is-one-row` check copies SlackMessage.js beside this file
// and runs `qml -platform offscreen slack-message.qml`.
import QtQuick
import "SlackMessage.js" as Message

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const said = { where: "in {place} \u00b7 message", direct: "direct message" };
        const held = { channel: "C1", ts: "1730000003.000100", place: "#deploys",
            who: "Grace Hopper", mine: false, first: "@joel ship it?",
            text: "@joel ship it?\nthe build is green", team: "T1",
            link: "https://slack.example/archives/C1/p1730000003000100" };
        const found = { key: "slack-message:C1:1730000003.000100", place: "#deploys",
            who: "grace", mine: false, first: "@joel ship it?",
            text: "@joel ship it?\nthe build is green", team: "T1", link: "" };
        const mine = { key: "slack-message:D2:1730000004.000100", place: "Ada Lovelace",
            who: "joel", mine: true, first: "lunch?", text: "lunch?", team: "T1" };

        check("held and found are one thing", [Message.key(held), Message.key(found)],
            ["slack:message:C1:1730000003.000100", "slack:message:C1:1730000003.000100"]);
        check("named by who said it", Message.who(held, "You"), "Grace Hopper");
        check("Joel's own as him", Message.who(mine, "You"), "You");
        check("its note its first line", Message.note(held, ""), "@joel ship it?");
        check("found in its first line, still its first line",
            Message.note(found, "ship"), "@joel ship it?");
        check("found past it, from the word the match is in",
            Message.note(found, "green build"), "build is green");
        check("matched inside a word, from that word", Message.note(found, "uil"),
            "build is green");
        check("matched nowhere, its first line", Message.note(found, "quokka"),
            "@joel ship it?");
        check("no first said, its text's first line",
            Message.note({ text: "one\ntwo" }, ""), "one");
        check("a channel's eyebrow says where", Message.eyebrow(held, said),
            "in #deploys \u00b7 message");
        check("a direct message's says so", Message.eyebrow(mine, said), "direct message");
        check("Return opens the message itself", Message.opens(held), held.link);
        check("with no link, its conversation in Slack", Message.opens(found),
            "slack://channel?team=T1&id=C1");

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
