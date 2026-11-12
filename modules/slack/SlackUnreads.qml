pragma Singleton
// What in Slack is unread and what Joel read last, as Slack's daemon
// serves it on its socket (src/daemon.rs): one connection, watching every
// slack-unread, slack-recent, slack-message, slack-seen and slack-account record, each
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
        kind: ["slack-unread", "slack-recent", "slack-message", "slack-seen", "slack-account"]
        keptThroughGap: false
    }

    // What's kept of what the daemon holds, the account and each other
    // kind's by key, and each part listed from it, remade only when a
    // change names one of its keys (SlackUnreads.js's keep), so a message
    // heard lists the messages alone and wakes nothing reading the rest,
    // where all 400 held were gathered again at each (RCTRF8, 2026-11-10).
    // Plain objects, so keeping one notifies nothing.
    readonly property var kept: Unreads.keeping()
    property var account: null
    property var conversationsHeld: []
    property var recentHeld: []
    property var messagesHeld: []
    property var seenHeld: []

    Connections {
        target: slack
        function onHeard(changed, gone) {
            root.took(changed.concat(gone));
        }
    }

    function took(keys) {
        const moved = Unreads.keep(kept, slack.held, keys);
        if (moved.account)
            account = kept.account;
        if (moved.conversations)
            conversationsHeld = Unreads.conversationsOf(kept);
        if (moved.recent)
            recentHeld = Unreads.recentOf(kept);
        if (moved.messages)
            messagesHeld = Unreads.messagesOf(kept);
        if (moved.seen)
            seenHeld = Unreads.seenOf(kept);
    }

    // What the daemon holds, { state, conversations, recent, messages,
    // whole }, or null while it isn't answering. What's made from it
    // reads the part it lists, so a part unmoved is the same list and
    // notifies nothing.
    readonly property var told: Unreads.toldBy(account, { conversations: conversationsHeld,
        recent: recentHeld, messages: messagesHeld }, slack.state === "answering")
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
    readonly property var conversations: stands === "ok"
        ? conversationsHeld.filter(Unreads.wants) : []
    // Every one unread, those wanting him first and then the channels
    // merely unread, for Slack's own Unread, of which Now's are the first
    // three groups, so the two never disagree.
    readonly property var unread: stands === "ok" ? conversationsHeld : []
    // What he read last, newest first, for Slack's Recent.
    readonly property var recent: stands === "ok" ? recentHeld : []
    // Each message wanting him, done ones too, the newest first, while the
    // daemon holds every one; else null, and the conversations stand for
    // them (SlackUnreads.js's messages). For You's Slack row counts those
    // still to read, and lists the rows the daemon serves (SlackRows.qml).
    readonly property var messages: Unreads.messages(told, stands)
    readonly property var toRead: messages === null ? null
        : messages.filter(one => !Unreads.done(one))

    // Slack's notices still unread whose message was read in Slack, found
    // again only when the notices or the messages held change, as this
    // watch hears them whether or not the launcher is open, so none waits
    // in Notices or comes back while Slack's daemon is away (Joel,
    // 2026-11-08: "i shouldn't be required to mark a notice as read if I
    // looked at the slack message in slack"). Each is read in one change,
    // saying where, so History says "read in Slack" rather than "read here".
    // They're read once this change is done: reading them rewrites the
    // history readThere is found from, which inside its own change is a
    // binding loop (tests/read-there.qml). Those whose conversation's read
    // mark has passed them are read too, the notices of channels that don't
    // mention him among them, which no message held stands for. It's a
    // stopgap in QML: the notices daemon is to hear Slack's marks and read
    // them itself.
    readonly property var readThere: {
        const held = Unreads.readInSlack(Notices.history, messages);
        return held.concat(Unreads.seenRead(Notices.history, seenHeld)
            .filter(id => !held.includes(id)));
    }
    onReadThereChanged: {
        if (readThere.length > 0)
            Qt.callLater(readReadThere);
    }

    // Reads what's read in Slack still, as a change since may have read it.
    function readReadThere() {
        if (readThere.length > 0)
            Notices.markRead(readThere, "readInSlack");
    }
    // Threads, which no message stands for, beside the messages.
    readonly property var unheld: Unreads.threads(conversations)
}
