// Locking and ending the session. These live here rather than on a key, so
// no chord ends the session by accident.
import qs
import QtQuick

Provider {
    id: provider

    things: [
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
    }))
}
