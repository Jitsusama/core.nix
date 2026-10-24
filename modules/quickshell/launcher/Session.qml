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

    readonly property var awake: thing({
        key: "session:awake",
        verb: Caffeinate.on ? "let sleep" : "keep awake",
        name: Caffeinate.on ? "Let the machine sleep" : "Keep the machine awake",
        keywords: "caffeinate keep awake stay awake keep on no sleep coffee insomnia "
            + "presentation lid let it sleep",
        side: !Caffeinate.on ? "" : Caffeinate.lid ? "awake, lid shut too" : "awake while open",
        glyph: String.fromCodePoint(0xf0176),
        hue: "machine",
        now: true,
        run: () => Caffeinate.on ? Caffeinate.release() : Caffeinate.keep()
    })

    // Do not disturb (Quiet.qml): Return holds notices for an hour, and the
    // right arrow offers holding them until it's ended.
    readonly property var quiet: thing({
        key: "session:quiet",
        verb: Quiet.on ? "turn off" : "turn on",
        name: Quiet.on ? "Turn off do not disturb" : "Turn on do not disturb",
        keywords: "quiet focus dnd silence mute snooze pause notifications notices alerts hold "
            + "end donotdisturb",
        side: Quiet.on ? Quiet.lasts : "for an hour",
        chord: bound(holds),
        glyph: String.fromCodePoint(0xf00a0),
        hue: "voice",
        run: () => Quiet.on ? Quiet.end() : Quiet.keep(60),
        verbs: Quiet.on ? null : () => [60, 0].map(minutes => provider.thing({
            key: `session:quiet:${minutes}`,
            name: minutes > 0 ? "For an hour" : "Until it's ended",
            glyph: String.fromCodePoint(minutes > 0 ? 0xf1862 : 0xf00a0),
            hue: "action",
            run: () => Quiet.keep(minutes)
        }))
    })

    // Each power profile is found by its own name, the one in use saying
    // so; the charger's coming and going picks one again. What's learned
    // knows each by the name it had first (was), so a rename keeps it.
    readonly property var profiles: [
        { profile: PowerProfile.PowerSaver, name: "Use power saver", was: "Power Saver",
            glyph: 0xf032a, words: "eco low power battery saver slow cool" },
        { profile: PowerProfile.Balanced, name: "Use balanced power", was: "Balanced Power",
            glyph: 0xf05d1, words: "normal default balanced" },
        { profile: PowerProfile.Performance, name: "Use full performance", was: "Performance",
            glyph: 0xf04c5, words: "turbo fast boost max speed" }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile).map(choice => thing({
        key: `session:power:${choice.was}`,
        verb: "use",
        name: choice.name,
        keywords: `power profile mode energy ${choice.words}`,
        side: PowerProfiles.profile === choice.profile ? "in use" : "",
        glyph: String.fromCodePoint(choice.glyph),
        hue: "machine",
        run: () => PowerProfiles.profile = choice.profile
    }))

    // Starting a recording is Capture's (Capture.qml). While anything
    // records, stopping it leads the home screen, so it never runs unseen.
    readonly property var recording: thing({
        key: "session:recording",
        verb: "stop",
        name: "Stop recording",
        keywords: "record recording video screencast capture",
        side: `recording ${Recorder.what}`,
        glyph: Theme.glyph.recording,
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
    // what Joel taught it, and that name still finds it.
    things: [awake, quiet].concat(Recorder.recording ? [recording] : [], profiles, [
        {
            name: "Lock the screen",
            was: "Lock",
            verb: "lock",
            does: locks,
            words: "lock screen lockscreen away leave",
            glyph: 0xf023,
            run: () => provider.launcher.lock.lock()
        },
        {
            name: "Suspend the machine",
            was: "Suspend",
            verb: "suspend",
            words: "sleep sleep now hibernate nap standby",
            glyph: 0xf04b2,
            run: () => Niri.spawn(["systemctl", "suspend"])
        },
        {
            name: "Log out",
            was: "Log Out",
            verb: "log out",
            words: "logout sign out signout quit niri end session",
            glyph: 0xf0343,
            sure: "Log out now",
            run: () => Niri.act({ Quit: { skip_confirmation: true } })
        },
        {
            name: "Reboot the machine",
            was: "Reboot",
            verb: "reboot",
            words: "restart reset relaunch",
            glyph: 0xf0453,
            sure: "Reboot now",
            run: () => Niri.spawn(["systemctl", "reboot"])
        },
        {
            name: "Power off the machine",
            was: "Power Off",
            verb: "power off",
            words: "shutdown shut down poweroff power down turn off halt",
            glyph: 0xf011,
            sure: "Power off now",
            run: () => Niri.spawn(["systemctl", "poweroff"])
        }
    ].map(action => thing({
        key: `session:${action.was}`,
        verb: action.verb,
        name: action.name,
        keywords: `${action.was.toLowerCase()} ${action.words}`,
        chord: action.does ? bound(action.does) : [],
        glyph: String.fromCodePoint(action.glyph),
        hue: "machine",
        run: action.run,
        // Those that end the session ask once, as a second Return, since
        // everyday words now find them.
        confirm: !!action.sure,
        verbs: action.sure ? () => [thing({
            key: `session:${action.was}:sure`,
            verb: action.verb,
            name: action.sure + provider.closing,
            glyph: String.fromCodePoint(action.glyph),
            hue: "alert",
            run: action.run
        })] : null
    })))

    function home() {
        return (Caffeinate.on ? [awake] : []).concat(Quiet.on ? [quiet] : [],
            Recorder.recording ? [recording] : []);
    }
}
