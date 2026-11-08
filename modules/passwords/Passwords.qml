// Passwords, the launcher's place for what Joel's password managers keep,
// as People is for everyone he deals with: the managers are its sources,
// each a program the passwords daemon runs (jitsusama.passwords.managers),
// and none is named here. The daemon holds every entry without its
// secrets; a secret is asked of it only to copy, and goes from the
// manager to the clipboard, marked sensitive and cleared after the time
// the daemon says, never through here, so nothing on screen shows one.
//
// It's found by typing "passwords", or an entry's name, username or
// website. With nothing typed it holds the favourites, then what changed
// lately, newest first (Passwords.js). Each entry is its title and its
// kind's mark, its username or website below, and Return copies its
// password and stays, the foot counting down to the clipboard clearing.
// Inside: Copy the Password, Copy the One-Time Code, Copy the Username,
// Open the Website, Show in its manager, Copy the Link (VKCF3Z, 22949064).
// Its drawer says every figure its manager gives but the secret.
//
// 1Password asks Joel through its own window to let its command in, the
// first time and again once its leave lapses, so the daemon asks a manager
// anything only for what Joel pressed: this asks for the list (`asked`)
// only as he steps into the place or types in it, and for an entry in
// full only as he steps into it, never as the shell starts or on a timer.
// The daemon's watch keeps what's held current after. While a manager
// can't read, the place's first row lets the launcher in, as Let the
// Launcher In leads Programs › 1Password, which, once it's let in, holds
// its entries by vault beside Quick Access and Lock.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Passwords.js" as Passwords

Provider {
    id: provider

    what: "entry"

    collection: Words.collection("passwords")

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/passwords.sock`
        kind: "entry"
        // Entries change as a manager is read, so each is kept by key and
        // a manager's standing told as it changes.
        onSourcesChanged: provider.changed()
    }

    // The managers, as the module lists them: each by its name in the
    // daemon, the desktop entry its program row is made from, its name as
    // Joel reads it, and the pattern of its own link to an entry.
    readonly property var managers: settings.managers ?? []
    // Whether rows name their manager: only while two or more are here.
    readonly property bool naming: Passwords.named(managers)

    // The place's own words, from the vocabulary.
    readonly property var said: Words.of("passwords").said ?? {}

    // The manager whose program this desktop entry is, or null.
    function managerOf(program) {
        return managers.find(manager => manager.program === program) ?? null;
    }

    function managerNamed(name) {
        return managers.find(manager => manager.name === name) ?? null;
    }

    // Whether a manager can't read until Joel lets the launcher in.
    function signedOut(manager) {
        return daemon.sources[manager.name]?.state === "signed-out";
    }

    // Every entry held, in the order the daemon holds them.
    readonly property var records: {
        const held = daemon.revision >= 0 ? daemon.held : {};
        return Object.values(held).filter(record => record && record.gone !== true);
    }

    // Each entry's row, made again only when its record is: the daemon
    // replaces a record whole when it changes, so one the same object is
    // the same entry. The rows made are kept in a plain object, which
    // changing tells no binding, so keeping them never makes them again.
    readonly property var madeOf: ({ rows: {} })
    readonly property var everyone: {
        const kept = {};
        const rows = records.map(record => {
            const was = madeOf.rows[record.key];
            const row = was && was.record === record ? was.row : made(record);
            kept[record.key] = { record: record, row: row };
            return row;
        });
        madeOf.rows = kept;
        return rows;
    }

    // Those listed with nothing typed, led by a row for each manager that
    // can't read, which lets the launcher in.
    readonly property var all: {
        const rows = everyone;
        const byKey = {};
        for (const row of rows)
            byKey[row.key] = row;
        return managers.filter(manager => signedOut(manager)).map(manager => unlock(manager))
            .concat(Passwords.listed(records).map(record => byKey[record.key]));
    }
    onAllChanged: changed()

    // The record held by a key now, or null.
    function recordOf(key) {
        return daemon.held[key] ?? null;
    }

    // An entry's row: its title, its kind's mark, its username or website
    // below, its password copied by Return where it has one, and stepping
    // in where it hasn't, as a note or an identity.
    function made(record) {
        const key = record.key;
        const manager = managerNamed(record.manager);
        const title = Passwords.titleOf(record);
        const note = Passwords.note(record, naming ? manager?.called ?? "" : "");
        const named = Passwords.username(record);
        const row = member({
            thing: true,
            key: key,
            name: title,
            glyph: Theme.glyph[Passwords.mark(record.category)] ?? "",
            hue: "machine",
            note: note,
            favourite: record.favourite === true,
            stays: true,
            verbs: () => provider.inside(key),
            // The foot says how a copy of it stands, else its note.
            state: () => provider.footFor(key, note),
            // What y takes: its username, which is no secret.
            operands: () => named === "" ? [] : [copyOperand("fact:passwords:username", named)]
        });
        Object.assign(row, Passwords.passworded(record)
            ? { returns: provider.act(key, "password") }
            : { verb: "more", run: () => provider.launcher?.enter(row) });
        // The words it's found by besides its title, and its drawer, each
        // made when first asked for, since most entries are never searched
        // or shown.
        const words = Passwords.words(record).join(" ");
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(words))
        });
        let drawn = null;
        Object.defineProperty(row, "drawer", {
            enumerable: true,
            get: () => drawn ?? (drawn = Passwords.details(record, {
                name: (id, state) => Words.name(`passwords:${id}`, state),
                said: said,
                now: Date.now(),
                manager: naming ? manager?.called ?? "" : ""
            }))
        });
        return Object.assign(row, {
            named: Fuzzy.letters([title.toLowerCase()]),
            worded: Fuzzy.letters([words.toLowerCase()])
        });
    }

    // What's inside an entry, in its one order (Passwords.js's acts).
    function inside(key) {
        const record = recordOf(key);
        if (record === null)
            return [];
        const manager = managerNamed(record.manager);
        const link = Passwords.shows(record, manager?.shows ?? "");
        return Passwords.acts(record, link).map(id => act(key, id, link));
    }

    // One act inside an entry, by its id in Passwords.js's acts. A
    // password or a one-time code is asked of the daemon to copy, and
    // stays, the foot saying how the copy stands; the username and the
    // website's address are no secret, and copy as any row's value does,
    // while the launcher holds the keys; the website and the entry in its
    // manager's program open through niri as the launcher goes.
    function act(key, id, shown) {
        const record = recordOf(key);
        if (record === null)
            return null;
        const words = `verb:passwords:${id}`;
        const manager = managerNamed(record.manager);
        const address = Passwords.website(record);
        const base = {
            key: words,
            name: Words.name(words, "", { called: manager?.called ?? "" }),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "action"
        };
        const copying = (detail, what) => ({
            stays: true,
            state: () => provider.footFor(key, ""),
            run: () => provider.copySecret(key, detail, what)
        });
        const plain = text => ({
            now: true,
            copies: () => text,
            run: () => {
                Quickshell.clipboardText = text;
            }
        });
        const opening = link => ({ later: true, run: () => Niri.spawn(["xdg-open", link]) });
        const made = {
            // What an entry is for, so it leads its step though it copies
            // (Launcher.qml's copying).
            password: () => Object.assign(copying(Passwords.password(record)?.id ?? null,
                said.password ?? ""), { use: true }),
            code: () => copying(Passwords.code(record)?.id ?? null, said.code ?? ""),
            username: () => plain(Passwords.username(record)),
            website: () => opening(address),
            // It wears its manager's own mark, as its program's row does,
            // and names it on the : line, as :open 1password.
            show: () => Object.assign(opening(shown ?? ""), {
                fill: { manager: manager?.name ?? "" },
                glyph: provider.programGlyph(manager?.program ?? "")
            }),
            link: () => plain(address)
        };
        return thing(Object.assign(base, made[id]()));
    }

    // The last copy asked of the daemon, which is the one on the clipboard:
    // { key, title, detail, clears, lapses, period, why }, and the time
    // the foot counts it down by, moved each second only while it counts.
    property var copied: null
    property real now: Date.now()

    // Asks the daemon to put an entry's password, or a detail of it, on
    // the clipboard, saying so in the foot until it's cleared.
    function copySecret(key, detail, what) {
        const record = recordOf(key);
        if (record === null)
            return;
        const copy = { key: key, title: Passwords.titleOf(record),
            detail: what, clears: 0, lapses: 0, period: 0 };
        copied = copy;
        now = Date.now();
        const asked = { ask: "copy", key: key };
        if (detail !== null)
            asked.detail = detail;
        daemon.ask(asked, answer => {
            if (copied?.key !== key || copied?.detail !== what)
                return;
            now = Date.now();
            copied = Passwords.answered(copy, answer, now, said.away ?? "");
        });
    }

    // What the foot says for a row of this entry: how its copy stands,
    // else `otherwise`.
    function footFor(key, otherwise) {
        const copy = copied?.key === key ? copied : null;
        return copy ? Passwords.told(copy, now, said) : otherwise;
    }

    Timer {
        interval: 1000
        repeat: true
        running: (provider.copied?.clears ?? 0) > provider.now
        onTriggered: provider.now = Date.now()
    }

    // The row leading the place while a manager can't read: Return asks it
    // to let the launcher in, as Programs' Let the Launcher In does, and
    // the launcher goes, so its own window can ask Joel.
    function unlock(manager) {
        const words = "verb:passwords:unlock";
        return thing({
            key: `${words}:${manager.name}`,
            name: Words.name(words, "", { manager: manager.called }),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "action",
            note: String(daemon.sources[manager.name]?.reason ?? ""),
            run: () => provider.letIn(manager)
        });
    }

    // Answered once Joel answers his manager, for as long as he takes.
    function letIn(manager) {
        daemon.ask({ ask: "let-in", manager: manager.name, patience: 120000 },
            () => provider.changed());
    }

    // Asks the daemon for every entry, as Joel steps into the place or types
    // in it, once each time the launcher opens: fresh, so what he sees is
    // his manager's now, and the watch says it as it lands.
    property bool listedThisTime: false

    function listAsked() {
        if (listedThisTime)
            return;
        listedThisTime = true;
        daemon.ask({ ask: "list", asked: true, fresh: true }, null);
    }

    // The entries asked in full this opening, so stepping into one again
    // asks nothing more.
    property var gotThisTime: ({})

    function getAsked(key) {
        if (gotThisTime[key] === true)
            return;
        gotThisTime[key] = true;
        daemon.ask({ ask: "get", keys: [key], asked: true }, null);
    }

    function opened() {
        listAsked();
    }

    // Stepping into an entry, wherever it's listed, reads it in full, so
    // its one-time code and its details show. Read from where the launcher
    // stands, never from what a row holds, which a walk of every row asks.
    Connections {
        target: provider.launcher

        function onHeldChanged() {
            const steps = provider.launcher.held ?? [];
            const last = steps.length > 0 ? steps[steps.length - 1].row : null;
            if (provider.launcher.open && last && provider.recordOf(last.key ?? "") !== null)
                provider.getAsked(last.key);
        }

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.listedThisTime = false;
                provider.gotThisTime = {};
            }
        }
    }

    // How many entries it holds, or that a manager is locked while it holds
    // none, at its row's right at home.
    function counted() {
        if (records.length === 0 && managers.some(manager => signedOut(manager)))
            return said.locked ?? "";
        return records.length === 0 ? "empty"
            : String(records.length).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }

    // Below its row at home: why a manager can't read, where one can't for
    // a reason other than Joel's leave.
    function noted() {
        return managers.filter(manager => daemon.sources[manager.name]?.state === "refused")
            .map(manager => `${manager.called}: ${daemon.sources[manager.name]?.reason ?? ""}`)
            .join(" \u00b7 ");
    }

    function everything() {
        return all;
    }

    // A search looks through every entry held, not only those listed.
    function looksThrough() {
        return everyone;
    }

    // From the top, what's typed finds what's held, asking no manager
    // anything, since typing there is no step into Passwords.
    function gather(typed) {
        return found(typed);
    }

    // Inside the place, typing is Joel's key, so it lists them too.
    function search(typed) {
        listAsked();
        return found(typed);
    }

    // Every entry whose title, or username, website or tag, each word typed
    // starts a word of, best first, at the closest grade any reached: each
    // row's letters are tested first, which passes most over without a
    // call, as People's search does.
    function found(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const missed = { score: -1, grade: 3 };
        const matched = [];
        let grade = 3;
        for (const row of everyone) {
            const byName = (row.named & need) === need
                ? Fuzzy.match(row.prepared, words, false) : missed;
            const byWords = (row.worded & need) === need
                ? Fuzzy.match(row.also, words, false) : missed;
            const best = byName.score >= byWords.score ? byName : byWords;
            if (best.score < 0)
                continue;
            grade = Math.min(grade, best.grade);
            matched.push({ score: best.score, grade: best.grade, row: row, at: matched.length });
        }
        const kept = matched.filter(each => each.grade === grade)
            .sort((a, b) => Number(b.row.favourite) - Number(a.row.favourite)
                || b.score - a.score || a.at - b.at);
        return { rows: kept.map(each => each.row), grade: grade };
    }

    function programNote(asked) {
        const manager = managerOf(asked);
        return manager && signedOut(manager)
            ? String(daemon.sources[manager.name]?.reason ?? "") : "";
    }

    // A manager's program leads with Let the Launcher In while it can't
    // read (VKCF3Z, 347abdb7), and once it can, holds its entries by vault,
    // each a part beside Quick Access and Lock.
    function programActs(asked) {
        const manager = managerOf(asked);
        if (manager === null)
            return [];
        if (signedOut(manager)) {
            const key = "verb:passwords:let-in";
            return [thing({
                key: key,
                name: Words.name(key),
                verb: Words.verb(key),
                glyph: Words.glyph(key),
                hue: "action",
                leads: true,
                run: () => provider.letIn(manager)
            })];
        }
        const byKey = {};
        for (const row of everyone)
            byKey[row.key] = row;
        return Passwords.vaults(records.filter(record => record.manager === manager.name))
            .map(vault => {
                const rows = () => vault.records.map(record => byKey[record.key])
                    .filter(Boolean);
                const part = thing({
                    key: `passwords:vault:${manager.name}:${vault.id}`,
                    name: Words.name("passwords:vault", "", { vault: vault.name }),
                    verb: Words.verb("passwords:vault"),
                    glyph: Words.glyph("passwords:vault"),
                    hue: "machine",
                    count: String(vault.records.length),
                    nested: true,
                    stays: true,
                    verbs: rows,
                    targets: rows,
                    run: () => provider.launcher?.enter(part)
                });
                return part;
            });
    }
}
