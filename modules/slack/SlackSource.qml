// Slack as one of People's sources (PersonSource.qml): Open in Slack for
// anyone Slack knows by one of their addresses, in the harness's Slack
// sign-in's workspace. It knows nobody by itself, since a workspace has
// thousands; it only asks slack-people, a reader that never sends
// anything, about a person once Joel steps into them, and their act
// appears as Slack answers, the place told it changed. Whoever Slack
// found it then knows, with their time zone as a fact, which People
// follows with the time there.
// Slack refusing the sign-in stops the asking until the shell starts
// again; with no sign-in, slack-people asks nothing.
import qs
import Quickshell.Io
import QtQuick

PersonSource {
    id: source

    // What Slack said of each address asked about: { id, team, name,
    // zone }, with no id for one Slack knows nobody by.
    property var known: ({})
    // The addresses asked about and not yet answered.
    property var asking: ({})

    people: Object.values(known).filter(said => said.id).map(said => ({
        emails: [said.email],
        name: said.name,
        facts: said.zone ? [{ label: "Time Zone", value: said.zone, zone: true }] : []
    }))

    function acts(person, provider) {
        const found = person.emails.map(email => known[email]).find(said => said && said.id);
        if (!found) {
            ask(person);
            return [];
        }
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

    // Asks Slack about each of their addresses not yet asked about.
    function ask(person) {
        const fresh = person.emails.filter(email => !(email in known) && !(email in asking));
        if (fresh.length === 0)
            return;
        if (!reader.running)
            reader.running = true;
        // Qt's JavaScript has no Object.fromEntries.
        const asked = Object.assign({}, asking);
        for (const email of fresh)
            asked[email] = true;
        asking = asked;
        for (const email of fresh)
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
        known = Object.assign({}, known, { [said.email]: said });
        const left = Object.assign({}, asking);
        delete left[said.email];
        asking = left;
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
