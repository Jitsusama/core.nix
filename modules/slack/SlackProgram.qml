// What Slack's program row in Programs says while Slack can't be checked,
// which Now doesn't, since it doesn't want Joel (VKCF3Z, b8c8920e): its
// note says signed out, or that Slack can't be reached (`programNote`,
// which Applications.qml asks every provider that has it); and signed
// out, its first act after Return's own is Sign In, as every signed-out
// source's is (VKCF3Z, 347abdb7), which signs in again in pi, whose
// Slack sign-in (/slack-auth) writes the file slack-unreads reads within
// half a minute.
//
// While Slack answers, its program holds Unread after Slack's own acts,
// and Recent after that (VKCF3Z, d7c0f933): Unread says how many to read
// beside it and who in its note, and steps in to each conversation
// unread, direct messages, then channels mentioning him, then Threads,
// then channels merely unread, of which Now's Slack row shows the first
// three; Recent is the nine conversations he read last. Both lists are
// Slack's daemon's, served ready in that order and worded
// (src/rows.rs), and each is made the one row every surface shows of a
// conversation (SlackConversation.qml's `served`). How Slack stands, and
// Unread's count, are SlackUnreads.qml's, until the daemon serves Unread's
// own row too, so this keeps nothing of its own.
import qs
import Quickshell
import QtQuick
import "Recent.js" as Recent

Provider {
    id: provider

    what: "program"

    // The desktop entry's id Slack's program row is made from.
    readonly property string program: "slack"
    readonly property string note: SlackUnreads.programNote
    // So Programs, read as the corpus is joined, says the note as it
    // stands now, and a program stepped into is drawn again as what's
    // unread or read changes.
    onNoteChanged: changed()

    // Unread's conversations and Recent's, as Slack's daemon serves them,
    // each change telling Programs to draw them again.
    ReadyRows {
        id: unreadRows

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/slack-unreads.sock`
        place: "slack-unread"
        maker: provider
        make: (row, key) => SlackConversation.served(unreadRows, row, key)
        drawn: provider.launcher?.open ?? false
        onHeard: provider.changed()
    }
    ReadyRows {
        id: recentRows

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/slack-unreads.sock`
        place: "slack-recent"
        maker: provider
        make: (row, key) => SlackConversation.served(recentRows, row, key)
        drawn: provider.launcher?.open ?? false
        onHeard: provider.changed()
    }

    function programNote(asked) {
        return asked === program ? note : "";
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        if (SlackUnreads.stands === "ok")
            return unreadRows.rows().length === 0 ? [] : [unread()];
        if (SlackUnreads.stands !== "signedOut")
            return [];
        const key = "verb:slack:sign-in";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            leads: true,
            run: () => Sessions.tell("/slack-auth", Sessions.home)
        })];
    }

    function recentOf(asked) {
        return asked === program ? recentRows.rows() : [];
    }

    // Slack's Recent is this one's to give, so it stands while none is read.
    function keepsRecentOf(asked) {
        return asked === program;
    }

    // Unread, which steps in to each conversation as it stands when
    // stepped into. Its row reads as every place holding things does:
    // nothing after its name, their names in its drawer, and how many to
    // read at its side, bare, since its name says what's counted, nothing
    // in the foot (VKCF3Z, 2026-11-07, 2026-11-08 and dafc2199).
    function unread() {
        const key = "slack:unread";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "program",
            heldNames: Recent.said(unreadRows.rows().map(one => one.name), false, {}).held,
            side: SlackUnreads.counted,
            ask: "which?",
            nested: true,
            targets: () => unreadRows.rows()
        });
    }
}
