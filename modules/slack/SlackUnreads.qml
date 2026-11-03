pragma Singleton
// What in Slack is unread and what Joel read last, as Slack's daemon
// serves it on its socket (src/daemon.rs): one connection, watching every
// slack-unread, slack-recent and slack-account record, each change heard
// as Slack's daemon hears it, so nothing here keeps a clock. What's held
// is let go once the daemon stops answering, so no count outlives it.
// What it says and how it stands are SlackUnreads.js's.
import Quickshell
import QtQuick
import "SlackUnreads.js" as Unreads

Singleton {
    id: root

    Daemon {
        id: slack

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/slack.sock`
        kind: "slack-unread"
        watching: ({ kind: ["slack-unread", "slack-recent", "slack-account"] })
        keptThroughGap: false
    }

    // What the daemon holds, { state, conversations, recent }, or null
    // while it isn't answering; read beside `revision`, so it's gathered
    // once a change.
    readonly property var told: slack.revision >= 0
        ? Unreads.gathered(slack.held, slack.state === "answering") : null
    // "none", "ok", "signedOut" or "unknown" (SlackUnreads.js's standing).
    readonly property string stands: Unreads.standing(told)
    readonly property bool shown: Unreads.shown(told, stands)
    readonly property string value: Unreads.value(told, stands)
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
    // Who and where, of every one unread.
    readonly property string unreadContext: Unreads.context(told, stands, true)
    // What he read last, newest first, for Slack's Recent.
    readonly property var recent: stands === "ok" ? told.recent : []

    // A conversation's row's words, for the piece, which lives where
    // SlackUnreads.js can't be imported.
    function beside(one) {
        return Unreads.beside(one);
    }

    function note(one) {
        return Unreads.note(one);
    }
}
