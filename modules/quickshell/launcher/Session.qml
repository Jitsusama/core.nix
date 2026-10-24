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
            keywords: Words.keywords(key, state),
            glyph: Words.glyph(key, state)
        }, fields));
    }

    readonly property var awake: said("session:awake", Caffeinate.on ? "on" : "", {
        side: !Caffeinate.on ? "" : Caffeinate.lid ? "awake, lid shut too" : "awake while open",
        hue: "machine",
        now: true,
        run: () => Caffeinate.on ? Caffeinate.release() : Caffeinate.keep()
    })

    // Do not disturb (Quiet.qml): Return holds notices for an hour, and the
    // right arrow offers holding them until it's ended.
    readonly property var quiet: said("session:quiet", Quiet.on ? "on" : "", {
        side: Quiet.on ? Quiet.lasts : "for an hour",
        chord: bound(holds),
        hue: "voice",
        run: () => Quiet.on ? Quiet.end() : Quiet.keep(60),
        verbs: Quiet.on ? null : () => [60, 0].map(minutes => provider.said(
            `session:quiet:${minutes}`, "", {
                verb: "hold",
                glyph: minutes > 0 ? String.fromCodePoint(0xf1862) : Theme.glyph.quiet,
                hue: "action",
                run: () => Quiet.keep(minutes)
            }))
    })

    // Each power profile is found by its own name, the one in use saying
    // so; the charger's coming and going picks one again. What's learned
    // knows each by the name it had first (was), so a rename keeps it.
    readonly property var profiles: [
        { profile: PowerProfile.PowerSaver, was: "Power Saver" },
        { profile: PowerProfile.Balanced, was: "Balanced Power" },
        { profile: PowerProfile.Performance, was: "Performance" }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile).map(choice => said(`session:power:${choice.was}`, "", {
        side: PowerProfiles.profile === choice.profile ? "in use" : "",
        hue: "machine",
        run: () => PowerProfiles.profile = choice.profile
    }))

    // Starting a recording is Capture's (Capture.qml). While anything
    // records, stopping it leads the home screen, so it never runs unseen.
    readonly property var recording: said("session:recording", "", {
        side: `recording ${Recorder.what}`,
        hue: "machine",
        run: () => Recorder.stop()
    })

    // What ending the session closes, said before it's done.
    readonly property string closing: {
        // Niri's windows are kept by id.
        const open = Object.keys(Niri.windows).length;
        return open === 0 ? "" : `, closing ${open === 1 ? "the window" : `${open} windows`}`;
    }

    // Each is learned by the name it had first (was), so renaming one keeps
    // what Joel taught it, and its old name, a hidden word, still finds it.
    things: [awake, quiet].concat(Recorder.recording ? [recording] : [], profiles, [
        { was: "Lock", does: locks, run: () => provider.launcher.lock.lock() },
        { was: "Suspend", run: () => Niri.spawn(["systemctl", "suspend"]) },
        { was: "Log Out", run: () => Niri.act({ Quit: { skip_confirmation: true } }) },
        { was: "Reboot", run: () => Niri.spawn(["systemctl", "reboot"]) },
        { was: "Power Off", run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => {
        const key = `session:${action.was}`;
        const sure = Words.of(key).sure ?? "";
        return said(key, "", {
            keywords: `${action.was.toLowerCase()} ${Words.keywords(key)}`,
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

    function home() {
        return (Caffeinate.on ? [awake] : []).concat(Quiet.on ? [quiet] : [],
            Recorder.recording ? [recording] : []);
    }
}
