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
//
// Meet's Recent (recentOf, which Applications.qml asks every provider that
// has it) is the calls Joel had through Meet: the meetings held lately on
// a Meet link, each once by its code, where it was newest, newest first
// (Calendar.called). A row is named by its meeting's title, what the call
// is, as Calendar names it and Now names a call on air; who it was with
// and when are at its side, "Ada Lovelace and Grace Hopper \u00b7
// yesterday", as a mail says who it's from and when, so the title isn't
// said twice, and its times below, its drawer the meeting's details.
// Return steps in to the meeting's acts, Join the Meeting first, so
// Return twice joins again, as an ended call's row does in Tuple's Recent
// (VKCF3Z): the call is over, so joining its room again is chosen, never
// what a press on a past call does at once.
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

    // Found from the top too, by their names: Join by Code, and the copy
    // only while it's granted, so it comes and goes as it does inside.
    foundActs: [{ program: "google-meet", key: "verb:meet:code" }].concat(granted
        ? [{ program: "google-meet", key: "verb:meet:link" }] : [])

    // Once made, it's copied after made's change is done: forgetting since,
    // or closing the launcher (which forgets it too), inside that change
    // would work made out again within its own change, a binding loop.
    onMadeChanged: {
        if (made.state === "made")
            Qt.callLater(provider.copy, made.link);
    }

    function copy(link) {
        since = null;
        if (!provider.launcher?.open)
            return;
        Quickshell.clipboardText = link;
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

    function recentOf(program) {
        return program === "google-meet" ? Calendar.called.map(call => provider.calledRow(call))
            : [];
    }

    function keepsRecentOf(program) {
        return program === "google-meet";
    }

    // What a call's side says: who it was with, in the vocabulary's words,
    // and when, in the words mail says when in.
    readonly property var said: Object.assign({}, Mail.dates, Words.of("meet:held").said ?? {})

    // So Programs › Meet, held as a call's meeting ends or a side moves on,
    // draws its Recent again, and only then.
    readonly property string recentSaid: JSON.stringify(Calendar.called
        .map(call => [call.code, Calendar.calledSide(call.meeting, said)]))
    onRecentSaidChanged: changed()

    // One call Joel had through Meet, as its Recent's row.
    function calledRow(call) {
        const meeting = call.meeting;
        const row = thing({
            key: `meet:held:${call.code}`,
            name: Calendar.titled(meeting),
            verb: Words.verb("meet:held"),
            glyph: Words.glyph("meet:held"),
            hue: "program",
            side: Calendar.calledSide(meeting, said),
            note: Calendar.span(meeting),
            drawer: Calendar.details(meeting),
            // A call is a thing with details, so the launcher gives it its
            // drawer (Launcher.qml, detailed), which it never does an act.
            thing: true,
            keywords: (meeting.guests ?? []).map(guest => guest.email).filter(Boolean)
                .join(" ") || null,
            at: meeting.start,
            stays: true,
            verbs: () => provider.meetingActs(meeting)
        });
        row.run = () => provider.launcher?.enter(row);
        return row;
    }

    // A call's meeting's acts, as the Calendar place's row of it holds
    // them, with Guests, Attachments and Move the daemon serves inside it
    // (Meetings.qml's lent), or Calendar.acts' where it serves none.
    function meetingActs(meeting) {
        const calendar = (provider.launcher?.sources ?? [])
            .find(one => one.settings?.name === "calendar") ?? null;
        const served = calendar?.lent?.(`calendar:meeting:${meeting.id}`) ?? null;
        return typeof served?.verbs === "function" ? served.verbs()
            : Calendar.acts(meeting, provider);
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
