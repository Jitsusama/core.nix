pragma Singleton
// A conversation in Slack, one thing wherever the launcher shows it: a
// person's direct messages with Joel in their Recent (SlackSource.qml),
// what in Slack wants him in Now (NewSlack.qml), and Slack's own Unread
// and its search once they come. `row` gives its whole row, made with the
// launcher's `thing` of the surface showing it, so no surface makes a
// second version (Joel, 2026-10-28 22:20). The surface says what's beside
// it, its note and its drawer, as only it knows them. Return opens it in
// Slack; inside are Open in Slack, then Copy the Link where it has a link
// to share, Copy last as everywhere.
import qs
import Quickshell
import QtQuick

Singleton {
    // `fields` is { id, name, opens, link, side, note, drawer }: `opens`
    // the slack:// link Slack opens it by, `link` one to share, or none.
    function row(fields, thing) {
        const open = () => Niri.spawn(["xdg-open", fields.opens]);
        const made = {
            key: `slack:conversation:${fields.id}`,
            what: "conversation",
            // Made here, which the launcher's walk holds every Slack
            // conversation row to (Launcher.qml's components).
            component: "slackConversation",
            thing: true,
            of: "program:slack",
            part: fields.name,
            name: fields.name,
            verb: Words.verb("verb:slack:open"),
            glyph: Words.glyph("slack:conversation"),
            hue: "program",
            side: fields.side ?? "",
            note: fields.note ?? "",
            later: true,
            run: open,
            verbs: () => acts(fields, open, thing)
        };
        if (fields.drawer !== undefined)
            made.drawer = fields.drawer;
        return thing(made);
    }

    function acts(fields, open, thing) {
        const act = (key, more) => thing(Object.assign({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, more));
        const listed = [act("verb:slack:open", { later: true, run: open })];
        if (fields.link)
            listed.push(act("verb:slack:link", {
                now: true,
                run: () => {
                    Quickshell.clipboardText = fields.link;
                }
            }));
        return listed;
    }
}
