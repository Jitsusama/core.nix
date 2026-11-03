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
// three; Recent is the nine conversations he read last. Each is the one
// row every surface shows of a conversation (SlackConversation.qml). What
// stands is SlackUnreads.qml's, so this keeps nothing of its own.
import qs
import QtQuick

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

    Connections {
        target: SlackUnreads

        function onUnreadChanged() {
            provider.changed();
        }

        function onRecentChanged() {
            provider.changed();
        }
    }

    function programNote(asked) {
        return asked === program ? note : "";
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        if (SlackUnreads.stands === "ok")
            return SlackUnreads.unread.length === 0 ? [] : [unread()];
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
        return asked === program ? SlackUnreads.recent.map(conversation) : [];
    }

    // Unread, which steps in to each conversation as it stands when
    // stepped into.
    function unread() {
        const key = "slack:unread";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "program",
            count: String(SlackUnreads.unread.length),
            side: SlackUnreads.value,
            note: SlackUnreads.unreadContext,
            ask: "which?",
            nested: true,
            targets: () => SlackUnreads.unread.map(conversation)
        });
    }

    function conversation(one) {
        return SlackConversation.row({
            id: one.id,
            name: one.name,
            opens: one.link,
            side: SlackUnreads.beside(one),
            note: SlackUnreads.note(one)
        }, thing);
    }
}
