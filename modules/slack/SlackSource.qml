// Slack as one of People's sources (PersonSource.qml): Open in Slack for
// anyone Slack knows by one of their addresses, in the harness's Slack
// sign-in's workspace, and how they are there: active, away or in a
// huddle, with their status's emoji, in their note, and their status and
// presence in the drawer (VKCF3Z, 42b57b7d and 67f6cbac). It knows nobody
// by itself, since a workspace has thousands; it only asks slack-people,
// a reader that never sends anything, about a person once Joel steps
// into them, and again on a later step-in once what it heard is a minute
// old, since how someone is changes while who they are doesn't. What
// it learns appears as Slack answers, the place told it changed. Whoever
// Slack found it then knows, with their time zone as a fact, which People
// follows with the time there.
// Slack refusing the sign-in stops the asking until the shell starts
// again; with no sign-in, slack-people asks nothing.
import qs
import Quickshell.Io
import QtQuick

PersonSource {
    id: source

    // What Slack said of each address asked about: { id, team, name,
    // zone, presence, status, emoji }, with no id for one Slack knows
    // nobody by.
    property var known: ({})
    // When each address was last asked about and not yet answered.
    property var asking: ({})
    // When each address was last answered, kept in place rather than set
    // anew, since nothing is drawn from it and setting it would remake
    // everyone.
    property var answered: ({})
    // How long an answer, or an ask Slack never answered, holds before a
    // step-in asks again.
    readonly property int fresh: 60 * 1000

    people: Object.values(known).filter(said => said.id).map(said => ({
        emails: [said.email],
        name: said.name,
        facts: facts(said)
    }))

    // How each stands in Slack, by address, in their note: active, away
    // or in a huddle, then their status's emoji.
    stands: {
        const by = {};
        for (const said of Object.values(known)) {
            if (said.id && said.presence)
                by[said.email] = {
                    said: [said.presence, said.emoji].filter(word => word).join(" "),
                    order: presenceOrder[said.presence] ?? presenceOrder.away
                };
        }
        return by;
    }
    readonly property var presenceOrder: ({
        active: 0,
        "in a huddle": 1,
        away: 2
    })

    // The drawer's facts of someone Slack found, each only while it says.
    function facts(said) {
        const status = [said.emoji, said.status].filter(word => word).join(" ");
        const presence = said.presence
            ? said.presence[0].toUpperCase() + said.presence.slice(1) : "";
        return [
            { label: "Status", value: status },
            { label: "Presence", value: presence },
            { label: "Time Zone", value: said.zone, zone: true }
        ].filter(fact => fact.value);
    }

    function acts(person, provider) {
        ask(person);
        const found = person.emails.map(email => known[email]).find(said => said && said.id);
        if (!found)
            return [];
        return [provider.thing({
            key: "verb:person:slack",
            name: Words.name("verb:person:slack"),
            verb: Words.verb("verb:person:slack"),
            glyph: Words.glyph("verb:person:slack"),
            hue: "action",
            main: true,
            later: true,
            run: () => Niri.spawn(["xdg-open", `slack://user?team=${found.team}&id=${found.id}`])
        })];
    }

    // Asks Slack about each of their addresses not heard of in the last
    // minute and not already being asked about.
    function ask(person) {
        const now = Date.now();
        const stale = email => !(now - (answered[email] ?? -Infinity) < fresh)
            && !(now - (asking[email] ?? -Infinity) < fresh);
        const due = person.emails.filter(stale);
        if (due.length === 0)
            return;
        if (!reader.running)
            reader.running = true;
        // Qt's JavaScript has no Object.fromEntries.
        const asked = Object.assign({}, asking);
        for (const email of due)
            asked[email] = now;
        asking = asked;
        for (const email of due)
            reader.write(`${email}\n`);
    }

    function heard(line) {
        let said;
        try {
            said = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (said?.signedOut) {
            reader.running = false;
            return;
        }
        if (typeof said?.email !== "string")
            return;
        answered[said.email] = Date.now();
        const left = Object.assign({}, asking);
        delete left[said.email];
        asking = left;
        // The same answer again changes nothing, so nobody is made anew.
        if (JSON.stringify(known[said.email]) === JSON.stringify(said))
            return;
        known = Object.assign({}, known, { [said.email]: said });
        place?.changed();
    }

    Process {
        id: reader
        command: ["slack-people", source.settings.credentials ?? ""]
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => source.heard(line)
        }
    }
}
