// Locking, keeping awake, holding notices, stopping a recording, the power
// profile and ending the session. These live here rather than on a key, so no chord ends the
// session by accident. While the machine is kept awake or notices are held,
// undoing it leads the home screen, since a mode should never hold unseen.
import qs
import Quickshell.Services.UPower
import QtQuick

Provider {
    id: provider

    what: "session"

    // Locking and holding notices have keys too, so these rows stand for
    // those actions and draw their chords.
    readonly property string locks: "Lock the screen"
    readonly property string holds: "Hold notices, or let them show"
    draws: [locks, holds]

    function bound(does) {
        const action = Shortcuts.actions.find(candidate => candidate.does === does);
        return action ? action.chord : [];
    }

    // A thing's name, verb, words and glyph as the vocabulary says them
    // (Words.qml), in the state it's in.
    function said(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            glyph: Words.glyph(key, state)
        }, fields));
    }

    readonly property var awake: said("session:awake", Caffeinate.on ? "on" : "", {
        side: !Caffeinate.on ? "" : Caffeinate.lid ? "awake, lid shut too" : "awake while open",
        hue: "machine",
        stays: true,
        run: () => Caffeinate.on ? Caffeinate.release() : Caffeinate.keep()
    })

    // Do not disturb (Quiet.qml): Return holds notices for an hour, and the
    // right arrow offers holding them until it's ended.
    readonly property var quiet: said("session:quiet", Quiet.on ? "on" : "", {
        side: Quiet.on ? Quiet.lasts : "for an hour",
        chord: bound(holds),
        hue: "voice",
        stays: true,
        run: () => Quiet.on ? Quiet.end() : Quiet.keep(60),
        verbs: Quiet.on ? null : () => [60, 0].map(minutes => provider.said(
            `session:quiet:${minutes}`, "", {
                verb: "hold",
                glyph: minutes > 0 ? Theme.glyph.timed : Theme.glyph.quiet,
                hue: "action",
                stays: true,
                run: () => Quiet.keep(minutes)
            }))
    })

    // The power profile is a setting: one part saying which is in use, its
    // choices a step in, each named there as the part's value reads
    // ("Balanced") and found from anywhere by its own name ("Balanced
    // power") or its words ("eco"), since the launcher lists a part of
    // another (`of`) inside it and finds it outside only by a word past
    // the whole's name. The charger's coming and going picks one again.
    // What's learned knows each by the name it had first (was), so a
    // rename keeps it.
    readonly property var profileChoices: [
        { profile: PowerProfile.PowerSaver, was: "Power Saver" },
        { profile: PowerProfile.Balanced, was: "Balanced Power" },
        { profile: PowerProfile.Performance, was: "Performance" }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile)
    readonly property var profiles: profileChoices.map(choice => {
        const key = `session:power:${choice.was}`;
        return said(key, "", {
            side: PowerProfiles.profile === choice.profile ? "in use" : "",
            hue: "machine",
            of: Words.of(key).of,
            part: Words.of(key).part,
            stays: true,
            run: () => PowerProfiles.profile = choice.profile
        });
    })
    readonly property var profileInUse: {
        const choice = profileChoices.find(each => each.profile === PowerProfiles.profile);
        return choice ? `session:power:${choice.was}` : "";
    }
    readonly property var profile: thingRow({
        key: "session:profile",
        name: Words.name("session:profile"),
        side: profileInUse !== "" ? Words.of(profileInUse).part : "",
        glyph: profileInUse !== "" ? Words.glyph(profileInUse) : Words.glyph("session:profile")
    }, () => [])

    // Starting a recording is Capture's (Capture.qml). While anything
    // records, stopping it leads the home screen, so it never runs unseen.
    readonly property var recording: said("session:recording", "", {
        side: `recording ${Recorder.what}`,
        hue: "machine",
        stays: true,
        run: () => Recorder.stop()
    })

    // What ending the session closes, said before it's done.
    readonly property string closing: {
        // Niri's windows are kept by id.
        const open = Object.keys(Niri.windows).length;
        return open === 0 ? "" : `, Closing ${open === 1 ? "the Window" : `${open} Windows`}`;
    }

    // What's shown of them says how they stand now: Return on a setting
    // keeps the launcher, and its row says what it changed to (Launcher.qml's
    // refreshed). Ending the session takes Joel out, so those close it.
    onThingsChanged: changed()

    // Each is learned by the name it had first (was), so renaming one keeps
    // what Joel taught it.
    things: [awake, quiet].concat(Recorder.recording ? [recording] : [], [profile], profiles, [
        { was: "Lock", does: locks, run: () => provider.launcher.lock.lock() },
        { was: "Suspend", run: () => Niri.spawn(["systemctl", "suspend"]) },
        { was: "Log Out", run: () => Niri.act({ Quit: { skip_confirmation: true } }) },
        { was: "Reboot", run: () => Niri.spawn(["systemctl", "reboot"]) },
        { was: "Power Off", run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => {
        const key = `session:${action.was}`;
        const sure = Words.of(key).sure ?? "";
        return said(key, "", {
            chord: action.does ? bound(action.does) : [],
            hue: "machine",
            run: action.run,
            // Those that end the session ask once, as a second Return, since
            // everyday words now find them.
            confirm: sure !== "",
            verbs: sure !== "" ? () => [provider.thing({
                key: `${key}:sure`,
                verb: Words.verb(key),
                name: sure + provider.closing,
                glyph: Words.glyph(key),
                hue: "alert",
                run: action.run
            })] : null
        });
    }))
}
