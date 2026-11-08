// Passwords, the launcher's place for what Joel's password managers keep,
// as People is for everyone he deals with: the managers are its sources,
// each a program the passwords daemon runs (jitsusama.passwords.managers),
// and none is named here. The daemon holds every entry without its
// secrets; a secret is asked of it only to copy or show.
//
// This first step gives each manager's program its one act while the
// manager can't read: Let the Launcher In, leading, as Sign In leads
// while a source is signed out (VKCF3Z, 347abdb7). 1Password asks Joel
// through its own window the first time, and again once its leave lapses,
// so the act is how its prompt answers his own key.
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "program"

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/passwords.sock`
        kind: "entry"
        // Entries change as a manager is read, so each is kept by key and
        // a manager's standing told as it changes.
        onSourcesChanged: provider.changed()
    }

    // The managers, as the module lists them: each by its name in the
    // daemon, the desktop entry its program row is made from, and its name
    // as Joel reads it.
    readonly property var managers: settings.managers ?? []

    // The manager whose program this desktop entry is, or null.
    function managerOf(program) {
        return managers.find(manager => manager.program === program) ?? null;
    }

    // Whether a manager can't read until Joel lets the launcher in.
    function signedOut(manager) {
        return daemon.sources[manager.name]?.state === "signed-out";
    }

    function programNote(asked) {
        const manager = managerOf(asked);
        return manager && signedOut(manager)
            ? String(daemon.sources[manager.name]?.reason ?? "") : "";
    }

    function programActs(asked) {
        const manager = managerOf(asked);
        if (manager === null || !signedOut(manager))
            return [];
        const key = "verb:passwords:let-in";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            leads: true,
            // Answered once Joel answers 1Password, for as long as he takes.
            run: () => daemon.ask({ ask: "let-in", manager: manager.name, patience: 120000 },
                () => provider.changed())
        })];
    }
}
