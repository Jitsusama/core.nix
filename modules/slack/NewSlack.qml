// What in Slack wants Joel (SlackUnreads.qml), as one of what wants him in
// Now and the margin, after the mail (VKCF3Z's placement): how many to
// read, and the first conversations by name. It's never a notice, since
// Slack's own say each message as it comes. Return lists each
// conversation, direct messages, then channels, then Threads, each newest
// first, as the one row every surface shows (SlackConversation.qml), and
// after them Show Slack, which goes to Programs › Slack. When Slack can't
// be asked, or its count has gone stale, it says so in place of a count,
// listing nothing; it goes once nothing wants him and Slack has said so.
import qs
import QtQuick

Piece {
    active: SlackUnreads.shown
    kind: "wants"
    wants: true
    hue: "person"
    title: "Slack"
    value: SlackUnreads.value
    hint: SlackUnreads.hint
    context: SlackUnreads.context
    rows: thing => SlackUnreads.conversations.map(one => SlackConversation.row({
        id: one.id,
        name: one.name,
        opens: one.link,
        side: SlackUnreads.beside(one),
        note: SlackUnreads.note(one)
    }, thing))
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
