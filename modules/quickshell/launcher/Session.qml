// Locking, keeping awake and ending the session. These live here rather than
// on a key, so no chord ends the session by accident. While the machine is
// kept awake, letting it sleep again leads the home screen, since a mode
// should never hold unseen.
import qs
import QtQuick

Provider {
    id: provider

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

    things: [awake].concat([
        { name: "Lock", glyph: 0xf023, run: () => provider.launcher.lock.lock() },
        { name: "Suspend", glyph: 0xf04b2, run: () => Niri.spawn(["systemctl", "suspend"]) },
        {
            name: "Log Out",
            glyph: 0xf0343,
            run: () => Niri.act({ Quit: { skip_confirmation: true } })
        },
        { name: "Reboot", glyph: 0xf0453, run: () => Niri.spawn(["systemctl", "reboot"]) },
        { name: "Power Off", glyph: 0xf011, run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => thing({
        key: `session:${action.name}`,
        verb: action.name.toLowerCase(),
        name: action.name,
        glyph: String.fromCodePoint(action.glyph),
        hue: "machine",
        run: action.run
    })))

    function home() {
        return Caffeinate.on ? [awake] : [];
    }
}
