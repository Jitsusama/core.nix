pragma Singleton
// A Slack message's one row, wherever it's listed: For You's Slack row
// (NewSlack.qml), its History, and what Search Slack finds
// (SlackSearch.qml), since a message is one thing (VKCF3Z, 2026-11-08).
// A message has no title, so it's named by who said it, its first line
// after the name on every row, as a place's members say what they hold,
// so two from one person tell themselves apart, and its side only when
// (VKCF3Z, 2026-11-08). Its drawer's eyebrow says where, "in #deploys ·
// message", or "direct message", and its body says it all, as prose.
// `row` gives its whole row,
// made with the launcher's `thing` of the surface showing it. Return
// opens the message where Slack shows it; inside are Open in Slack, then
// Copy the Link, Copy last as everywhere. What it works out is
// SlackMessage.js's.
import qs
import Quickshell
import QtQuick
import "FileFacts.js" as FileFacts
import "When.js" as When
import "SlackMessage.js" as Message

Singleton {
    // How the launcher says when, in its words (FileFacts.js's wordsOf).
    readonly property var whenWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // Joel, as a row of his own is named: "You".
    function you(said) {
        const word = String(said.you ?? "");
        return word.charAt(0).toUpperCase() + word.slice(1);
    }

    // `one` is a message as the daemon says it (SlackMessage.js). `found`
    // is what Search Slack gives: the `words` it was found by, the `key`
    // it lights under, and `around(one)`, Open in Slack's drawer, the
    // messages around it.
    function row(one, thing, found) {
        const said = Words.of("slack:message").said ?? {};
        const open = () => Niri.spawn(["xdg-open", Message.opens(one)]);
        const now = Date.now();
        const day = When.day(one.at, now, whenWords.days);
        const clock = FileFacts.clock(new Date(one.at));
        const name = Message.who(one, you(said));
        return thing({
            key: found?.key ?? Message.key(one),
            what: "slack message",
            // Made here, which the launcher's walk holds every message
            // row to (Launcher.qml's components).
            component: "slackMessage",
            thing: true,
            of: "program:slack",
            part: name,
            name: name,
            verb: Words.verb("verb:slack:message:open"),
            glyph: Words.glyph("slack:message"),
            hue: "program",
            side: FileFacts.when(one.at, now, whenWords),
            // Given way by whole words, never an ellipsis (Choice.qml).
            count: Message.note(one, found?.words ?? ""),
            countWords: true,
            drawer: {
                eyebrow: Message.eyebrow(one, said),
                eyebrowNames: !Message.direct(one),
                title: name,
                facts: [{
                    label: Words.name("slack:message:sent"),
                    value: day === "" ? clock : `${day} ${clock}`
                }],
                // What was said is prose, set as every drawer's sentences
                // are; when it was sent stays a fact.
                body: one.text ?? "",
                prose: true
            },
            later: true,
            run: open,
            verbs: () => acts(one, open, thing, found)
        });
    }

    function acts(one, open, thing, found) {
        const act = (key, more) => thing(Object.assign({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, more));
        const opening = { later: true, run: open };
        if (found?.around)
            opening.drawer = found.around(one);
        const listed = [act("verb:slack:message:open", opening)];
        if (one.link)
            listed.push(act("verb:slack:message:link", {
                now: true,
                run: () => {
                    Quickshell.clipboardText = one.link;
                }
            }));
        return listed;
    }
}
