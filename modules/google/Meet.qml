// Meet's own acts in the launcher, listed after its desktop entry's New
// Meeting (home.nix), as the program's acts (programActs, which
// Applications.qml asks every provider that has it): Join by Code, before
// its Recent, which asks for a meeting's code, typed or pasted with or
// without its dashes or as its whole link, and joins its call through
// google-opens, in a window of its own, anything Meet wouldn't know as a
// code refused, the foot saying so; and last, as every copy is, Copy a
// New Meeting's Link, which has the google program make one and copies
// its link without opening Meet.
//
// Copy a New Meeting's Link is listed only once Joel's sign-in grants
// Meet's scope (src/api.rs), so it's never offered when it can't work.
// The launcher stays while Google makes it, the foot saying so; the link
// is copied the moment it comes, while the launcher still holds the keys
// the clipboard needs, and the launcher leaves saying "copied", as every
// copy does. One Google won't make stays, saying so, and copies nothing
// (VKCF3Z, 959f52e2).
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "call"

    // What the foot says of the last code given: joining it, or that it
    // isn't one.
    property string told: ""

    // When the last new meeting was asked for, or null.
    property var since: null
    readonly property var made: Google.making(since)
    readonly property bool granted: Google.meet.granted

    // Programs › Meet lists the copy only while it's granted.
    onGrantedChanged: changed()

    onMadeChanged: {
        if (made.state !== "made")
            return;
        since = null;
        if (!provider.launcher?.open)
            return;
        Quickshell.clipboardText = made.link;
        provider.launcher.leaveSaying("copied");
        provider.launcher.open = false;
    }

    // A meeting asked for as the launcher closed isn't copied later, when
    // the launcher no longer holds the keys.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.since = null;
        }
    }

    function programActs(program) {
        if (program !== "google-meet")
            return [];
        const key = "verb:meet:code";
        const acts = [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            ask: Words.of(key).ask ?? "",
            chip: Words.name(key),
            state: () => provider.told,
            given: words => provider.join(words)
        })];
        return granted ? acts.concat([copying()]) : acts;
    }

    function copying() {
        const key = "verb:meet:link";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => ({ making: "making it", failed: "Google didn't make one" })[
                provider.made.state] ?? "",
            run: () => provider.make()
        });
    }

    function make() {
        since = Date.now();
        Google.make();
    }

    function join(words) {
        const code = Calendar.meetCode(words);
        if (code === "") {
            told = "not a Meet code";
            return;
        }
        told = `joining ${code}`;
        Niri.spawn(["google-opens", `https://meet.google.com/${code}`]);
    }
}
