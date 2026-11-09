pragma Singleton
// What in Slack is unread and what Joel read last, as Slack's daemon
// serves it on its socket (src/daemon.rs): one connection, watching every
// slack-unread, slack-recent, slack-message and slack-account record, each
// change heard as Slack's daemon hears it, so nothing here keeps a clock.
// What's held is let go once the daemon stops answering, so no count
// outlives it. What it says and how it stands are SlackUnreads.js's.
// Slack's own desktop notice of a message read in Slack is read in
// Notices too, as the message leaves For You by itself.
import qs
import Quickshell
import QtQuick
import "SlackUnreads.js" as Unreads

Singleton {
    id: root

    Daemon {
        id: slack

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/slack.sock`
        kind: ["slack-unread", "slack-recent", "slack-message", "slack-account"]
        keptThroughGap: false
    }

    // What the daemon holds, { state, conversations, recent, messages,
    // whole }, or null while it isn't answering; read beside `revision`,
    // so it's gathered once a change.
    readonly property var told: slack.revision >= 0
        ? Unreads.gathered(slack.held, slack.state === "answering") : null
    // "none", "ok", "signedOut" or "unknown" (SlackUnreads.js's standing).
    readonly property string stands: Unreads.standing(told)
    readonly property bool shown: Unreads.shown(told, stands)
    readonly property string value: Unreads.value(told, stands)
    // How many to read, bare, for a row whose name says what it counts, as
    // Slack's own Unread (VKCF3Z, 2026-11-08), counted over the
    // conversations it lists, never the messages For You lists.
    readonly property string counted: stands === "ok"
        ? String(Unreads.conversationsCount(told)) : ""
    readonly property string context: Unreads.context(told, stands)
    readonly property string hint: Unreads.hint(told, stands)
    // Why there's no count, for Slack's program row (SlackProgram.qml).
    readonly property string programNote: Unreads.programNote(stands)
    // Each { id, name, kind, count, link } that wants Joel, for Now, only
    // while Slack answers, so no old count is offered as Slack's.
    readonly property var conversations: stands === "ok" ? told.conversations.filter(Unreads.wants)
        : []
    // Every one unread, those wanting him first and then the channels
    // merely unread, for Slack's own Unread, of which Now's are the first
    // three groups, so the two never disagree.
    readonly property var unread: stands === "ok" ? told.conversations : []
    // What he read last, newest first, for Slack's Recent.
    readonly property var recent: stands === "ok" ? told.recent : []
    // Each message wanting him, done ones too, the newest first, while the
    // daemon holds every one; else null, and the conversations stand for
    // them (SlackUnreads.js's messages). For You's Slack row lists those
    // still to read, and its History those done (NewSlack.qml).
    readonly property var messages: Unreads.messages(told, stands)
    readonly property var toRead: messages === null ? null
        : messages.filter(one => !Unreads.done(one))
    readonly property var doneWith: messages === null ? null : messages.filter(Unreads.done)

    // Slack's notices still unread whose message was read in Slack, found
    // again only when the notices or the messages held change, as this
    // watch hears them whether or not the launcher is open, so none waits
    // in Notices or comes back while Slack's daemon is away (Joel,
    // 2026-11-08: "i shouldn't be required to mark a notice as read if I
    // looked at the slack message in slack"). Each is read in one change,
    // saying where, so History says "read in Slack" rather than "read here".
    readonly property var readThere: Unreads.readInSlack(Notices.history, messages)
    onReadThereChanged: {
        if (readThere.length > 0)
            Notices.markRead(readThere, "readInSlack");
    }
    // Threads, which no message stands for, beside the messages.
    readonly property var unheld: stands === "ok" ? Unreads.unheld(told) : []

    // A conversation's row's words, for the piece, which lives where
    // SlackUnreads.js can't be imported.
    function beside(one) {
        return Unreads.beside(one);
    }

    function note(one) {
        return Unreads.note(one);
    }
}
