// What in Slack wants Joel (SlackUnreads.qml), as one of what wants him in
// Now and the margin, after the mail (VKCF3Z's placement): how many to
// read, and the first conversations by name. It's never a notice, since
// Slack's own say each message as it comes. Return lists each
// conversation, direct messages, then channels, then Threads, each newest
// first, as the one row every surface shows (SlackConversation.qml), and
// after them Show Slack, which goes to Programs › Slack. It's here only
// while something waits to be read: when Slack can't be asked, that's no
// want of Joel's, so Slack's program row says it (SlackProgram.qml), and
// a refused sign-in waits in For You (VKCF3Z, b8c8920e and 347abdb7);
// with Slack's daemon not answering, there's no count to show at all. In
// Now it's For You's Slack row, among what came for him (Now.qml's
// Lately), and the margin shows it as it is. While the daemon holds every
// message wanting him, Return lists each message instead, by who said it
// (SlackMessage.qml), then Threads, whose replies it doesn't hold, and
// each read in Slack leaves by itself for For You's History (Joel,
// 2026-11-08: "i shouldn't be required to mark a notice as read if I
// looked at the slack message in slack"). Its rows, and History's, are
// those Slack's daemon serves ready (SlackRows.qml); its count, who and
// when are SlackUnreads'.
import qs
import QtQuick
import "../Marks.js" as Marks

Piece {
    id: slack

    active: SlackUnreads.shown
    // Slack's daemon says what's unread whether or not there's any, so
    // Slack's own notices stand for nothing while it answers.
    answers: SlackUnreads.stands === "ok"
    kind: "wants"
    wants: true
    // Muted while what's unread is only the many: where the machine says
    // who Joel is in Slack, its mark lights strong once a notice to him
    // (one the notices daemon says chimes) comes, and goes muted again
    // once it's read or he's been in Slack's window since (VKCF3Z's ruling
    // f94c382e), since a thread's reply is never said read. A QML
    // stopgap: Slack's daemon is to serve whether one waits.
    hushed: (Options.notices.addressed ?? []).includes("slack")
        && !Marks.toHim(Notices.history, "slack", slack.visited)

    // When Joel was last in Slack's window.
    property real visited: 0

    Connections {
        target: Niri

        function onCurrentChanged() {
            const window = Niri.windows[Niri.current];
            if ((window?.app_id ?? "").toLowerCase() === "slack")
                slack.visited = Date.now();
        }
    }
    hue: "person"
    title: "Slack"
    value: SlackUnreads.value
    hint: SlackUnreads.hint
    context: SlackUnreads.context
    rows: thing => SlackRows.forYou(thing)
    lately: "slack"
    // Slack's timestamps are seconds, as text; a message's, milliseconds.
    came: (SlackUnreads.toRead ?? []).map(one => ({ who: one.who, at: one.at }))
        .concat((SlackUnreads.toRead === null ? SlackUnreads.conversations : SlackUnreads.unheld)
            .map(one => ({
                who: one.name,
                at: one.latest ? Number(one.latest) * 1000 : null
            })))
    finished: SlackRows.finished
    actions: [{
        glyph: Theme.glyph.chat,
        name: "Show Slack",
        verb: "show",
        run: shell => {
            shell.launcher.open = true;
            shell.launcher.goTo({
                trail: ["collection:applications", "program:slack"],
                spot: ""
            });
        }
    }]
}
