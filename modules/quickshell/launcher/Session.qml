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

    readonly property var awake: thing({
        key: "session:awake",
        verb: Caffeinate.on ? "let it sleep" : "keep awake",
        name: Caffeinate.on ? "Let It Sleep" : "Keep Awake",
        keywords: "caffeinate awake sleep lid",
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
        verb: Quiet.on ? "end it" : "hold notices",
        name: Quiet.on ? "End Do Not Disturb" : "Do Not Disturb",
        keywords: "quiet focus dnd silence notifications notices hold",
        side: Quiet.on ? Quiet.lasts : "for an hour",
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
    // so; the charger's coming and going picks one again.
    readonly property var profiles: [
        { profile: PowerProfile.PowerSaver, name: "Power Saver", glyph: 0xf032a },
        { profile: PowerProfile.Balanced, name: "Balanced Power", glyph: 0xf05d1 },
        { profile: PowerProfile.Performance, name: "Performance", glyph: 0xf04c5 }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile).map(choice => thing({
        key: `session:power:${choice.name}`,
        verb: "use",
        name: choice.name,
        keywords: "power profile energy battery speed",
        side: PowerProfiles.profile === choice.profile ? "in use" : "",
        glyph: String.fromCodePoint(choice.glyph),
        hue: "machine",
        run: () => PowerProfiles.profile = choice.profile
    }))

    // Starting a recording is Capture's (Capture.qml). While anything
    // records, stopping it leads the home screen, so it never runs unseen.
    readonly property var recording: thing({
        key: "session:recording",
        verb: "stop it",
        name: "Stop Recording",
        keywords: "record recording video screencast capture",
        side: `recording ${Recorder.what}`,
        glyph: Theme.glyph.recording,
        hue: "machine",
        run: () => Recorder.stop()
    })

    things: [awake, quiet].concat(Recorder.recording ? [recording] : [], profiles, [
        {
            name: "Lock",
            words: "lock screen away leave",
            glyph: 0xf023,
            run: () => provider.launcher.lock.lock()
        },
        {
            name: "Suspend",
            words: "sleep hibernate nap standby",
            glyph: 0xf04b2,
            run: () => Niri.spawn(["systemctl", "suspend"])
        },
        {
            name: "Log Out",
            words: "logout sign out signout quit niri end session",
            glyph: 0xf0343,
            run: () => Niri.act({ Quit: { skip_confirmation: true } })
        },
        {
            name: "Reboot",
            words: "restart reset",
            glyph: 0xf0453,
            run: () => Niri.spawn(["systemctl", "reboot"])
        },
        {
            name: "Power Off",
            words: "shutdown shut down turn off halt",
            glyph: 0xf011,
            run: () => Niri.spawn(["systemctl", "poweroff"])
        }
    ].map(action => thing({
        key: `session:${action.name}`,
        verb: action.name.toLowerCase(),
        name: action.name,
        keywords: action.words,
        glyph: String.fromCodePoint(action.glyph),
        hue: "machine",
        run: action.run
    })))

    function home() {
        return (Caffeinate.on ? [awake] : []).concat(Quiet.on ? [quiet] : [],
            Recorder.recording ? [recording] : []);
    }
}
