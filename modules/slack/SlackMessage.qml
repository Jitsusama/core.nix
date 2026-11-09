pragma Singleton
// A Slack message wanting Joel, as For You's Slack row lists each
// (NewSlack.qml), held by Slack's daemon (src/messages.rs): named by who
// said it, its note its first words, so two from one person tell
// themselves apart, and its side only when, "14:02" (VKCF3Z, 2026-11-08).
// Its drawer's eyebrow says where, "in #deploys · message", or "direct
// message", and its body says it all. `row` gives its whole row, made
// with the launcher's `thing` of the surface showing it, so For You's list
// and its History show the same one. Return opens its conversation in
// Slack; inside are Open in Slack, then Copy the Link, Copy last as
// everywhere.
import qs
import Quickshell
import QtQuick
import "FileFacts.js" as FileFacts
import "When.js" as When

Singleton {
    // How the launcher says when, in its words (FileFacts.js's wordsOf).
    readonly property var whenWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // `one` is a message as the daemon says it: { channel, ts, place, who,
    // first, text, at, link, team }.
    function row(one, thing) {
        const said = Words.of("slack:message").said ?? {};
        const open = () => Niri.spawn(["xdg-open",
            `slack://channel?team=${one.team ?? ""}&id=${one.channel}`]);
        const now = Date.now();
        // A direct message's conversation starts D, as Slack names them.
        const direct = one.channel.startsWith("D");
        const day = When.day(one.at, now, whenWords.days);
        const clock = FileFacts.clock(new Date(one.at));
        return thing({
            key: `slack:message:${one.channel}:${one.ts}`,
            what: "slack message",
            // Made here, which the launcher's walk holds every message
            // row of For You's to (Launcher.qml's components).
            component: "slackMessage",
            thing: true,
            of: "program:slack",
            part: one.who,
            name: one.who,
            verb: Words.verb("verb:slack:message:open"),
            glyph: Words.glyph("slack:message"),
            hue: "program",
            side: FileFacts.when(one.at, now, whenWords),
            note: one.first ?? "",
            drawer: {
                eyebrow: direct ? said.direct ?? ""
                    : String(said.where ?? "").replace("{place}", one.place ?? ""),
                eyebrowNames: !direct,
                title: one.who,
                facts: [{ label: Words.name("slack:message:sent"), value: day === "" ? clock : `${day} ${clock}` }],
                body: one.text ?? ""
            },
            later: true,
            run: open,
            verbs: () => acts(one, open, thing)
        });
    }

    function acts(one, open, thing) {
        const act = (key, more) => thing(Object.assign({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, more));
        const listed = [act("verb:slack:message:open", { later: true, run: open })];
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
