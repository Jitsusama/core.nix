// How bright the screen and the keyboard are, answered in the field:
// "brightness", "backlight", "screen light" or "keyboard light". The
// headline is the screen's brightness; the rows are each screen's light and
// each keyboard's, as a share and as the steps its driver counts. With a
// number, "brightness 40", Return sets the screen to that share, as the
// brightness keys do, so the backlight daemon (modules/backlight), which
// follows the room, learns it as Joel's pick. Read from the kernel's files when asked, their
// folders listed once by Qt, so nothing is spawned and nothing is watched.
import qs
import Qt.labs.folderlistmodel
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    examples: ["brightness"]

    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        const asked = /^(?:brightness|backlight|screen(?: light| brightness)?)\s+(?:to\s+)?(\d{1,3})\s*%?$/
            .exec(text);
        if (asked)
            return set(Math.min(100, Number(asked[1])));
        if (!/^(brightness|bright|backlight|screen (light|brightness)|(keyboard|key) (light|lights|backlight|brightness)|dim)$/.test(text))
            return null;
        return told();
    }

    // Setting the screen to a share, Return doing it; the first step stays
    // lit, since none at all leaves a screen black.
    function set(share) {
        const shown = told();
        if (shown === null || backlights.count === 0)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: `set the screen to ${kept}%`,
            verb: "set",
            run: () => Niri.spawn(["brightnessctl", "--class=backlight", "set", `${kept}%`]),
            rows: shown.rows,
            note: `now ${shown.headline}`
        };
    }

    // Each light in a folder whose name passes, as its share and its steps.
    function lights(folder, model, wanted) {
        const found = [];
        for (let i = 0; i < model.count; i++) {
            const name = model.get(i, "fileName");
            if (!wanted.test(name))
                continue;
            const now = Number(said(`${folder}/${name}/brightness`));
            const most = Number(said(`${folder}/${name}/max_brightness`));
            if (most > 0)
                found.push({ name: name, now: now, most: most });
        }
        return found;
    }

    function told() {
        const screens = lights("/sys/class/backlight", backlights, /./);
        const keyboards = lights("/sys/class/leds", leds, /kbd_backlight$/);
        if (screens.length === 0 && keyboards.length === 0)
            return null;
        const share = light => `${Math.round(light.now / light.most * 100)}%`;
        const rows = screens.map(light => copyRow({
            name: share(light),
            side: `the screen, ${light.now} of ${light.most}`,
            glyph: String.fromCodePoint(0xf00df)
        })).concat(keyboards.map(light => copyRow({
            name: light.now === 0 ? "off" : `${light.now} of ${light.most}`,
            side: "the keyboard's light",
            glyph: String.fromCodePoint(0xf030c)
        })));
        return {
            headline: screens.length > 0 ? `${share(screens[0])} bright` : rows[0].name,
            rows: rows,
            note: "the screen follows the room; the keys set it"
        };
    }

    FolderListModel {
        id: backlights
        folder: "file:///sys/class/backlight"
        showFiles: false
        showDotAndDotDot: false
    }

    FolderListModel {
        id: leds
        folder: "file:///sys/class/leds"
        showFiles: false
        showDotAndDotDot: false
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}
