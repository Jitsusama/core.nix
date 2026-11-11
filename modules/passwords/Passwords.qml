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
// kind's mark, its username at its side and its website below, and Return
// copies its password and the launcher goes, as every copy does, so Joel
// can paste at once; the margin wears the clipboard's mark until the copy
// clears (Copied.qml), and the foot, opened again, counts down to it.
// Inside: Copy the Password, Copy the One-Time Code, Copy the Username,
// Open in the browser its manager fills in, Open the Website, Show in its
// manager, Copy the Link (VKCF3Z, 22949064).
// Its drawer says every figure its manager gives but the secret.
//
// 1Password asks Joel through its own window to let its command in, the
// first time and again once its leave lapses, so the daemon asks a manager
// anything only for what Joel pressed: this asks for the list (`asked`)
// only as he steps into the place or types in it, and for an entry in
// full as he steps into it, or as his selection rests on it, which asks
// only a manager that has let the launcher in already, never as the shell
// starts or on a timer of its own.
// The daemon's watch keeps what's held current after. While a manager
// can't read, the place's first row lets the launcher in, as Let the
// Launcher In leads Programs › 1Password, which, once it's let in, holds
// its entries by vault beside Quick Access and Lock, then the place's own
// Needing Attention and Check Every Password.
//
// Check Every Password, last in the place, has the daemon read every
// password to find those reused and those breached (`audit`), only as
// Joel presses it, since 1Password asks his leave for it; its side says
// so while it runs. Each entry's Health then says what was found
// (Passwords.js), and Needing Attention, after any unlock row, holds
// each entry with a problem, counted.
//
// What the last check found outlives the daemon, which keeps it, and
// stands in For You as one to-do however many logins it found, so a
// check never floods it (VKCF3Z [6d59cde5], WCJQSG [faf5592e]): Change
// the Passwords the Check Found, how many are breached and reused at its
// side, Return stepping into those logins alone. Weak, unsecured and old
// logins stay here and are never in it. It goes to For You's History once
// a later check finds none breached or reused (`forYou`, which Now.qml
// gathers). Stepping into it is stepping into Passwords' list, so it asks
// for the list as stepping in here does.
//
// Long because it's one provider whose rows, acts and notes all read the
// one daemon's standing as it changes; what works on plain values already
// lives apart, in Passwords.js, Managers.js and Checked.js, where cases
// reach it.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Ago.js" as Ago
import "Checked.js" as Checked
import "ForYou.js" as ForYou
import "Passwords.js" as Passwords
import "Managers.js" as Managers

Provider {
    id: provider

    what: "entry"

    // While it's off, Home's row's drawer says why, and what to use meanwhile,
    // rather than only its one row's name, which its line already says.
    collection: off ? Object.assign({}, Words.collection("passwords"), { holding: offAbout })
        : Words.collection("passwords")

    Daemon {
        id: daemon

        // The shell's own child, with no socket another process could
        // reach, so what Joel's manager lets it read is the shell's alone.
        // Never run while the place is off. It copies nothing unless a
        // copier is named, which only the checks do until agents are
        // fenced off the clipboard; null keeps one out of its environment.
        command: provider.off ? [] : provider.settings.daemon ?? []
        environment: ({ PASSWORDS_COPIER: provider.settings.copier ?? null })
        kind: "entry"
        // Entries change as a manager is read, so each is kept by key and
        // a manager's standing told as it changes.
        onSourcesChanged: provider.changed()
        // What the last check found is asked as the daemon says hello, so
        // once as the shell starts and again whenever the daemon does, and
        // taken as its watch tells it changed.
        onTold: answer => provider.heardTold(answer)
    }

    // The managers, as the module lists them: each by its name in the
    // daemon, the desktop entry its program row is made from, its name as
    // Joel reads it, the pattern of its own link to an entry, and the app
    // ids its program's windows carry.
    readonly property var managers: settings.managers ?? []

    // Whether the place is off, as it is until agents are fenced off Joel's
    // runtime folder and Wayland: every agent runs as his uid, so a daemon
    // his manager let in would answer any of them (2026-11-10, Joel: "we
    // can't have a password backdoor here"). It then holds one row, saying
    // so and opening the browser his manager's own extension fills in.
    readonly property bool off: settings.off === true
    // Whether the launcher is walked from outside it, as `qs ipc call
    // launcher run`, `doing` and `show` walk it for any process of the
    // account's. While it is, the place holds no row and no act, and asks
    // its daemon nothing, so only Joel's keys in the launcher reach an
    // entry, a manager's let-in or a check.
    readonly property bool outside: provider.launcher?.cataloguing === true

    // The manager whose browser extension fills sign-ins in, and that
    // browser, which the place offers while it's off; null where none has.
    // Read from the entries as a binding, so one read before they're loaded
    // is read again once they are, which DesktopEntries.byId isn't.
    readonly property var offManager: {
        const ids = DesktopEntries.applications.values.map(entry => entry.id);
        return managers.find(one => (one.fills ?? "") !== "" && ids.includes(one.fills)) ?? null;
    }
    readonly property var offBrowser: offManager === null ? null : browserOf(offManager)

    // What the off row's drawer, and Home's row's, say: why it's off, and
    // what its Return opens meanwhile, where a manager's extension fills in.
    readonly property string offAbout: {
        // Sentences a drawer tells, so its words' tells rather than a said run.
        const words = Words.of("passwords").tells ?? {};
        const why = String(words.offWhy ?? "");
        if (offManager === null)
            return why;
        const fills = String(words.offFills ?? "").replace("{manager}", offManager.called)
            .replace("{browser}", offBrowser.name);
        return `${why} ${fills}`;
    }

    // The one row while it's off: Return opens the browser a manager's
    // extension fills sign-ins in, which needs no clipboard, or else what
    // Managers.js's offOpens says, going into its drawer when nothing can
    // open. Its drawer says why it's off, which its name alone doesn't, and
    // with nothing to open, why and what would. The entries are read as a
    // binding, so one read before they're loaded is read again once they
    // are, which DesktopEntries.byId isn't.
    readonly property var offRow: {
        const ids = DesktopEntries.applications.values.map(entry => entry.id);
        const listed = id => (id ?? "") !== "" && ids.includes(id) ? DesktopEntries.byId(id) : null;
        const opens = Managers.offOpens(managers, one => listed(one.fills),
            one => listed(one.program));
        const name = Words.name("passwords:off");
        const manager = opens.manager?.called ?? "";
        const row = {
            key: "passwords:off",
            name: name,
            glyph: Words.glyph("passwords:off"),
            note: String(said[opens.note] ?? "").replace("{manager}", manager),
            hue: "alert",
            leads: true,
            // An act with something to say beyond its name, so its drawer
            // comes out (Launcher.qml's detailed).
            thing: true,
            drawer: { title: name, body: provider.offDrawn(opens) }
        };
        Object.assign(row, opens.act === "open" ? {
            verb: Words.verb("passwords:off"),
            run: () => Niri.spawn(provider.command(opens.opened))
        } : {
            verb: "show",
            stays: true,
            run: () => provider.launcher?.intoDrawer?.()
        });
        return Object.assign(thing(row), { named: Fuzzy.letters([name.toLowerCase()]),
            worded: 0 });
    }

    // What the off row's drawer says: why it's off and what Return opens
    // (offAbout), or when nothing can open, why it's off, why nothing opens
    // and what would.
    function offDrawn(opens) {
        if (opens.act !== "drawer")
            return offAbout;
        const tells = Words.of("passwords:off").tells ?? {};
        const why = Words.of("passwords").tells?.offWhy ?? "";
        const lacks = opens.manager === null ? tells.noManager ?? ""
            : String(tells.noApp ?? "").replace("{manager}", opens.manager.called ?? "");
        return [why, lacks, tells.would].filter(one => (one ?? "") !== "").join(" ");
    }

    // How long after an ask of Joel's a manager's window may come and still
    // be his answer to it: the manager waits up to 20 s for a closed app to
    // open, and its prompt follows within a few more. One already answering
    // asks again only once its leave lapses, and at once, so its window is
    // given less, lest one changing for its own reasons close the launcher
    // as Joel reads.
    readonly property int opening: 30000
    readonly property int asking: 5000

    // Said just before an ask that may have a manager put its prompt up
    // while the launcher is open, so the launcher closes and that window
    // takes the keys rather than coming up beneath it (2026-11-10:
    // stepping into Passwords had 1Password ask beneath the launcher,
    // until Joel's next key closed it).
    function handOver() {
        const windows = managers.map(manager => manager.windows ?? [])
            .reduce((all, some) => all.concat(some), []);
        const answering = managers.every(manager =>
            daemon.sources[manager.name]?.state === "answering");
        if (windows.length > 0)
            provider.launcher?.handsOver?.(windows, answering ? asking : opening);
    }
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

    // The desktop entry of the browser a manager's own extension fills
    // sign-ins in, as 1Password's does in Chrome, or null where it has
    // none or the machine hasn't that browser.
    function browserOf(manager) {
        return (manager?.fills ?? "") === "" ? null : DesktopEntries.byId(manager.fills);
    }

    // The desktop entry of a manager's own app, or null where the machine
    // hasn't it.
    function appOf(manager) {
        return (manager?.program ?? "") === "" ? null : DesktopEntries.byId(manager.program);
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

    // Each entry's row, made again only when its record says something
    // new (Managers.js's rowsKept). The rows made are kept in a plain
    // object, which changing tells no binding, so keeping them never makes
    // them again.
    readonly property var madeOf: ({ rows: {} })
    readonly property var everyone: {
        const listed = Managers.rowsKept(records, madeOf.rows, record => provider.made(record));
        madeOf.rows = listed.kept;
        return listed.rows;
    }

    // Those listed with nothing typed, led by a row for each manager that
    // can't read, which lets the launcher in, then Needing Attention once a
    // check has found a problem, and ended by Check Every Password.
    readonly property var all: {
        if (provider.off)
            return [provider.offRow];
        if (provider.outside)
            return [];
        const rows = everyone;
        const byKey = {};
        for (const row of rows)
            byKey[row.key] = row;
        return unreadRows().concat(attention,
            Passwords.listed(records).map(record => byKey[record.key]), [checkRow]);
    }
    onAllChanged: changed()

    // Whether every password is being checked now, as Joel asked, and why
    // the last check couldn't finish or couldn't ask about breaches.
    property bool checking: false
    property string checkWhy: ""

    // Check Every Password, which reads every password, so it runs only as
    // Joel's own act, never as the place opens or on a timer. The launcher
    // goes, as letting a manager in does, so 1Password's own window can ask
    // him first; while it runs, its side says so, as the place's row at
    // home does, and it says below why the last couldn't finish.
    readonly property var checkRow: {
        const words = "verb:passwords:audit";
        const name = Words.name(words);
        return Object.assign(thing({
            key: words,
            name: name,
            side: checking ? said.checking ?? "" : "",
            note: checking ? "" : checkWhy,
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "action",
            run: () => provider.checkEvery()
        }), { named: Fuzzy.letters([name.toLowerCase()]), worded: 0 });
    }

    // Needing Attention, a part holding each entry the last check found a
    // problem with, by title, counted; none before a check, or with none.
    readonly property var attention: {
        const needing = Passwords.needing(records);
        if (needing.length === 0)
            return [];
        const rows = () => {
            const byKey = {};
            for (const row of provider.everyone)
                byKey[row.key] = row;
            return Passwords.needing(provider.records).map(record => byKey[record.key])
                .filter(Boolean);
        };
        const name = Words.name("passwords:attention");
        const part = thing({
            key: "passwords:attention",
            name: name,
            verb: Words.verb("passwords:attention"),
            glyph: Words.glyph("passwords:attention"),
            hue: "machine",
            count: String(needing.length),
            nested: true,
            stays: true,
            verbs: rows,
            targets: rows,
            run: () => provider.launcher?.enter(part)
        });
        return [Object.assign(part, { named: Fuzzy.letters([name.toLowerCase()]), worded: 0 })];
    }

    // Asks the daemon to check every password, Joel having pressed it: each
    // entry's health comes back on its record by the watch, and the answer
    // says only why it couldn't, if it couldn't.
    function checkEvery() {
        if (checking || off || outside)
            return;
        checking = true;
        checkWhy = "";
        handOver();
        daemon.ask({ ask: "audit", asked: true, patience: 120000 }, answer => {
            provider.checking = false;
            provider.checkWhy = answer === null ? said.away ?? ""
                : answer.error ? String(answer.error.message ?? answer.error.code)
                : String(answer.unreached ?? "");
            provider.askChecked();
        });
    }

    // What the last check of every password found, as the daemon keeps it
    // through a reboot (Checked.js's figures), or null before any check.
    // It's replaced only when a figure differs, so For You's to-do is made
    // again only then, never as an entry changes or the launcher opens.
    property var figures: null

    function tookChecked(checked) {
        const next = Checked.figures(checked ?? null);
        if (!Checked.sameFigures(figures, next))
            figures = next;
    }

    function askChecked() {
        daemon.ask({ ask: "health" }, answer => {
            if (answer !== null && answer.checked !== undefined)
                provider.tookChecked(answer.checked);
        });
    }

    function heardTold(answer) {
        if (answer.id === "hello")
            askChecked();
        else if (answer.id === "all" && answer.checked !== undefined)
            tookChecked(answer.checked);
    }

    // The to-do's own words, from the vocabulary.
    readonly property var todoSaid: Words.of("passwords:todo").said ?? {}

    // For You's one to-do for the passwords the last check found breached
    // or reused, never one a login, as Now gathers each source's (Now.qml's
    // waitingRows): while it stands, { row, thing }, which For You weighs
    // beside the rest; once a later check finds none, { row, done }, how
    // it went and when, for its History. Made only as the figures change.
    readonly property var forYou: {
        const found = figures;
        if (Checked.stands(found))
            return [{ row: todoRow(found, false), thing: Checked.todoThing(found) }];
        if (Checked.cleared(found))
            return [{ row: todoRow(found, true), done: { how: todoSaid.how ?? "",
                at: found.cleared } }];
        return [];
    }

    // The to-do's row: its name, the counts at its side or, done, how it
    // went and when, and Return stepping in, never acting, to the logins
    // it's for (troubledRows), as every For You row steps into what it
    // asks. Its drawer is made as it's shown, so its weight says now's.
    function todoRow(found, done) {
        const words = "passwords:todo";
        const rows = () => provider.troubledRows();
        // Made again below while done, so Return enters the row it is.
        let row = member({
            thing: true,
            what: "on",
            key: done ? `${words}:done` : words,
            // What it is and where it's from, which For You filters by.
            forYou: { kind: "do", source: "passwords" },
            verb: "more",
            stays: true,
            nested: true,
            name: Words.name(words),
            glyph: Words.glyph(words),
            hue: done ? "muted" : "strong",
            side: done ? "" : Checked.todoSide(found, todoSaid),
            // Where the row is narrow, how many are reused gives way
            // first, whole, rather than either being cut.
            sideItems: done ? [] : Checked.todoSideItems(found, todoSaid),
            empty: Words.of(words).empty,
            drawer: () => provider.todoDrawer(found, done),
            verbs: rows,
            targets: rows,
            run: () => provider.launcher?.enter(row)
        });
        // Done, it says when as the clock says it as it's read, so one
        // cleared yesterday names its day.
        if (done)
            row = gotten(row, { side: () => provider.clearedSide(found) });
        return row;
    }

    // What the to-do holds: a row for each manager that can't read, which
    // lets the launcher in, then each entry the last check found breached
    // or reused, by title, as Needing Attention lists them.
    function troubledRows() {
        const byKey = {};
        for (const row of everyone)
            byKey[row.key] = row;
        return unreadRows().concat(Passwords.needing(records)
            .map(record => byKey[record.key]).filter(Boolean));
    }

    // The to-do's drawer, in the facts every For You drawer gives
    // (Now.qml's drawerOf): where it's from and that it's a thing to do,
    // what it weighs, how long it takes, since when it has waited, when the
    // last check ran and what clears it. Its words open with why it's
    // there, the counts whole, which the side gives way by (VKCF3Z,
    // 4e0fcf16), then what to do. Done, how it went stands in place of
    // what it weighed and what clears it, as a done item's does.
    function todoDrawer(found, done) {
        const now = Date.now();
        const say = forYouSaid(now);
        const thing = Checked.todoThing(found);
        const fact = (id, value) => ({ label: Words.name(id), value: String(value) });
        const facts = [
            fact("on:waiting:weight", done ? "" : ForYou.weightSaid(thing, now, say)),
            fact("on:waiting:takes", done || thing.takes === 0 ? ""
                : ForYou.filled(say.takes, { minutes: thing.takes })),
            fact("on:waiting:since", found.since === null ? "" : clocked(found.since)),
            fact("on:waiting:resolved", done ? clearedSide(found) : ""),
            fact("on:waiting:checked", clocked(found.at)),
            fact("on:waiting:clears", done ? "" : todoSaid.clears ?? "")
        ].filter(one => one.value !== "");
        const tells = Words.of("passwords:todo").tells ?? {};
        const why = done ? ""
            : Checked.todoWhy(found, todoSaid, tells.why ?? "", clocked(found.at));
        const steps = done ? "" : tells.steps ?? "";
        return {
            eyebrow: ForYou.filled(say.from, { from: Words.name("passwords"), kind: say.todo }),
            eyebrowNames: true,
            title: Words.name("passwords:todo"),
            facts: facts,
            body: [why, steps].filter(one => one !== "").join("\n\n")
        };
    }

    // Done, how it went and when, "checked clear 14:02", as History says.
    function clearedSide(found) {
        return ForYou.historySide({ how: todoSaid.how ?? "", resolved: found.cleared,
            since: null }, forYouSaid(Date.now()));
    }

    // For You's words, with the clock and how long ago as the launcher
    // says them, at `now`, as Now says its own (Now.qml's forYouSaid).
    function forYouSaid(now) {
        return Object.assign({}, Words.of("on:waiting").said ?? {}, {
            clock: at => provider.clocked(at),
            ago: at => Ago.ago(at, now)
        });
    }

    // A time as the clock says it: today's by its hour, and another day's
    // with its day, as For You's own are.
    function clocked(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    // The record held by a key now, or null.
    function recordOf(key) {
        return daemon.held[key] ?? null;
    }

    // An entry's row: its title, its kind's mark, its username at its side
    // and its website below, its password copied by Return where it has
    // one, and stepping in where it hasn't, as a note or an identity.
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
            side: Passwords.side(record),
            note: note,
            // Its note, after its name, the drawer's Website says while
            // it's out, so it's said once.
            noteInDrawer: true,
            favourite: record.favourite === true,
            verbs: () => provider.inside(key),
            // How a copy of it stands, at its right, else only its note.
            state: () => provider.footFor(key, note),
            // What y takes: its username, which is no secret.
            operands: () => named === "" ? [] : [copyOperand("fact:passwords:username", named)]
        });
        Object.assign(row, Passwords.passworded(record)
            ? { returns: provider.act(key, "password") }
            : { verb: "more", stays: true, run: () => provider.launcher?.enter(row) });
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
    // password or a one-time code is asked of the daemon to copy, and the
    // launcher goes, as every copy does, so Joel can paste at once: the
    // daemon puts it on the clipboard itself, needing no keys of the
    // launcher's, the margin's mark stands until it clears, and the foot
    // says how it stands when the launcher opens again, its manager's
    // refusal among it. The username and the entry's link are no secret,
    // and copy as any row's value does, while the launcher holds the keys;
    // the website and the entry in its manager's program open through
    // niri as the launcher goes. A web page opens in the browser its
    // manager's extension fills in, where there's one, wearing its mark,
    // and the extension offers to fill it on the page, as the foot says:
    // the launcher never fills it, so no secret passes through here or the
    // daemon. Anything else opens as any address does (VKCF3Z, 2026-11-08:
    // one act, never a second doing the same).
    function act(key, id, shown) {
        const record = recordOf(key);
        if (record === null)
            return null;
        const words = `verb:passwords:${id}`;
        const manager = managerNamed(record.manager);
        const address = Passwords.website(record);
        const browser = id === "website" && Passwords.fillsAt(record) !== ""
            ? browserOf(manager) : null;
        const base = {
            key: words,
            name: Words.name(words, "", { called: manager?.called ?? "" }),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "action"
        };
        const copying = (detail, what) => ({
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
            // Where its manager fills in a browser, it opens there wearing
            // the browser's own mark.
            website: () => browser === null ? opening(address) : {
                later: true,
                run: () => Niri.spawn(provider.command(browser)
                    .concat([Passwords.fillsAt(record)])),
                state: () => Passwords.fill(Words.of(words).said?.fills ?? "",
                    { manager: manager?.called ?? "" }),
                glyph: provider.programGlyph(manager?.fills ?? "")
            },
            // It wears its manager's own mark, as its program's row does,
            // and names it on the : line, as :open 1password.
            show: () => Object.assign(opening(shown ?? ""), {
                fill: { manager: manager?.name ?? "" },
                glyph: provider.programGlyph(manager?.program ?? "")
            }),
            link: () => plain(shown ?? "")
        };
        return thing(Object.assign(base, made[id]()));
    }

    // Asks the daemon to put an entry's password, or a detail of it, on
    // the clipboard, the copy kept where the margin's mark and the foot
    // read it until it's cleared (Clearing.qml).
    function copySecret(key, detail, what) {
        const record = recordOf(key);
        if (record === null || outside)
            return;
        const copy = { key: key, title: Passwords.titleOf(record),
            detail: what, clears: 0, lapses: 0, period: 0 };
        Clearing.copied = copy;
        Clearing.now = Date.now();
        const asked = { ask: "copy", key: key };
        if (detail !== null)
            asked.detail = detail;
        handOver();
        daemon.ask(asked, answer => {
            if (Clearing.copied?.key !== key || Clearing.copied?.detail !== what)
                return;
            Clearing.now = Date.now();
            Clearing.copied = Passwords.answered(copy, answer, Clearing.now, said.away ?? "");
        });
    }

    // What the foot says for a row of this entry: how its copy stands while
    // there's something to say of it (Passwords.js's speaks), else
    // `otherwise`.
    function footFor(key, otherwise) {
        const copy = Clearing.copied?.key === key ? Clearing.copied : null;
        return Passwords.speaks(copy) ? Passwords.told(copy, Clearing.now, said) : otherwise;
    }

    // The row leading the place while a manager can't read: a thing, the
    // manager by its name, "1Password", its side saying it's locked and
    // Return unlocking it (VKCF3Z's ruling): it asks the manager to let
    // the launcher in, as Programs' Let the Launcher In does, and the
    // launcher goes, so its own window can ask Joel. It's the one row
    // whether 1Password is locked or was never let in, since the manager
    // can't tell them apart (op refuses both alike, and every manager
    // starts unlet after a boot, while 1Password is locked), and Return
    // does the same for both.
    // Below it, the manager's own words for why, as "is closed, and Unlock
    // opens it", so a closed app reads otherwise than a locked one (2026-11-10:
    // Unlock with 1Password closed did nothing, and said nothing).
    // It runs once the launcher has gone, so 1Password's prompt never comes
    // up beneath it.
    function unlock(manager) {
        const words = "verb:passwords:unlock";
        return thing({
            key: `${words}:${manager.name}`,
            name: Words.name(words, "", { manager: manager.called }),
            side: said.locked ?? "",
            note: whyNot(manager),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "action",
            later: true,
            run: () => provider.letIn(manager)
        });
    }

    // Why a manager can't read, by its own words less its name, or nothing.
    function whyNot(manager) {
        return Managers.noted(daemon.sources[manager.name]?.reason, manager.called);
    }

    // The row leading the place while a manager is refused, as 1Password
    // is while its Integrate with 1Password CLI is off: Return opens its
    // program, where the setting is, the note below saying which, since
    // the launcher can't turn it on and Unlock would ask for nothing
    // (2026-11-10: the place was empty while it was refused).
    function settle(manager) {
        const words = "verb:passwords:settle";
        const program = DesktopEntries.byId(manager.program);
        return thing({
            key: `${words}:${manager.name}`,
            name: Words.name(words, "", { manager: manager.called }),
            side: said.refused ?? "",
            note: whyNot(manager),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "alert",
            leads: true,
            later: true,
            run: () => {
                if (program)
                    Niri.spawn(provider.command(program));
            }
        });
    }

    // A row for each manager that can't read: Unlock while it waits on
    // Joel or its app, and its program while it's refused, never nothing.
    function unreadRows() {
        return Managers.unread(managers, daemon.sources)
            .map(unread => unread.row === "unlock" ? unlock(unread.manager)
                : settle(unread.manager));
    }

    // Answered once Joel answers his manager, for as long as he takes. The
    // launcher has gone by then, so one that didn't let it in says why the
    // next time it opens, by the manager's own reason, as a folder gone
    // before Return is (Launcher.qml's tellOnOpen): never nothing, as when
    // 1Password keeps its command out and no prompt ever comes.
    function letIn(manager) {
        if (off || outside)
            return;
        handOver();
        daemon.ask({ ask: "let-in", manager: manager.name, patience: 120000 }, answer => {
            provider.changed();
            const why = answer === null ? said.away ?? "" : answer.error?.message ?? "";
            if (why !== "")
                provider.launcher?.tellOnOpen([[manager.called,
                    Managers.noted(why, manager.called), Theme.alert]]);
        });
    }

    // Asks the daemon for every entry, as Joel steps into the place or types
    // in it, once each time the launcher opens: fresh, so what he sees is
    // his manager's now, and the watch says it as it lands.
    property bool listedThisTime: false

    function listAsked() {
        if (listedThisTime || off || outside)
            return;
        listedThisTime = true;
        handOver();
        daemon.ask({ ask: "list", asked: true, fresh: true }, null);
    }

    // The entries asked in full this opening, so reading one again asks
    // nothing more: `asked` for one Joel stepped into, his own ask, and
    // `rested` for one he rested on, which the manager answers only once
    // it has let the launcher in, so resting never has 1Password ask him.
    property var gotThisTime: ({})

    function getAsked(key, asked) {
        const had = gotThisTime[key];
        if (off || outside || had === "asked" || (!asked && had === "rested"))
            return;
        // Read in full already, from resting on it, so stepping in needs
        // nothing more.
        if (asked && Array.isArray(recordOf(key)?.details))
            return;
        gotThisTime[key] = asked ? "asked" : "rested";
        if (asked)
            handOver();
        daemon.ask({ ask: "get", keys: [key], asked: asked }, null);
    }

    // An entry the selection rests on is read in full, once the drawer has
    // come out for it, so its drawer says its Strength, its One-Time Code
    // and its health, and stepping in finds every act in place: one read
    // only once stepped into arrived after the rest of its step, and went
    // below them, out of the design's order, since a row arriving late
    // never moves those above it.
    Timer {
        id: resting

        interval: Theme.motion.medium
        onTriggered: {
            const row = provider.launcher?.selected ?? null;
            if (provider.launcher?.open && row && provider.recordOf(row.key ?? "") !== null)
                provider.getAsked(row.key, false);
        }
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
                provider.getAsked(last.key, true);
            // Stepping into For You's to-do is stepping into Passwords'
            // list, Joel's key, so the logins it's for are read.
            if (provider.launcher.open && String(last?.key ?? "").startsWith("passwords:todo"))
                provider.listAsked();
        }

        function onSelectedChanged() {
            resting.restart();
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
        if (off)
            return said.offSide ?? "";
        if (records.length === 0 && managers.some(manager => signedOut(manager)))
            return said.locked ?? "";
        if (records.length === 0 && managers.some(manager => refused(manager)))
            return said.refused ?? "";
        return records.length === 0 ? "empty"
            : String(records.length).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }

    // Below its row at home: why a manager can't read, where one can't for
    // a reason other than Joel's leave, as 1Password keeping its command
    // out, by the manager's name once: "1Password: needs Integrate with
    // 1Password CLI on, in Settings \u203a Developer".
    function noted() {
        if (off)
            return offRow.name;
        return (checking ? [said.checking ?? ""] : []).concat(managers
            .filter(manager => refused(manager))
            .map(manager => `${manager.called}: ${Managers.noted(
                daemon.sources[manager.name]?.reason, manager.called)}`))
            .join(" \u00b7 ");
    }

    function refused(manager) {
        return daemon.sources[manager.name]?.state === "refused";
    }

    function everything() {
        return all;
    }

    // What it holds, by name after its name wherever its row stands: its
    // managers and its entries, never Check Every Password, which is its
    // act (VKCF3Z, db2d4ed6).
    function memberNames(held = everything()) {
        return held.filter(one => one.key !== checkRow.key && one.folded !== true)
            .map(one => one.name);
    }

    // A search looks through every entry held, not only those listed, and
    // the place's own rows: Check Every Password, and Needing Attention.
    function looksThrough() {
        if (off)
            return [offRow];
        if (outside)
            return [];
        return everyone.concat(attention, [checkRow]);
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
        for (const row of looksThrough()) {
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

    // What its drawer in Programs says it holds (Applications.qml asks
    // every provider's programFacts), from the entries already held, and
    // nothing while the manager can't read.
    function programFacts(asked) {
        const manager = managerOf(asked);
        if (manager === null || signedOut(manager))
            return [];
        return Managers.held(records.filter(record => record.manager === manager.name), said)
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // Below its program: why it can't read, whether Joel's leave is all it
    // waits on or it's refused, as while 1Password keeps its command out.
    function programNote(asked) {
        const manager = managerOf(asked);
        return manager && (signedOut(manager) || refused(manager))
            ? Managers.noted(daemon.sources[manager.name]?.reason, manager.called) : "";
    }

    // A manager's program leads with Let the Launcher In while it can't
    // read (VKCF3Z, 347abdb7), and once it can, holds its entries by vault,
    // each a part beside Quick Access and Lock. After them come the place's
    // own Needing Attention, only while a check has found something, and
    // Check Every Password last, the same rows Passwords holds, so its
    // health is a key away from its program too (VKCF3Z, 04d3db68).
    function programActs(asked) {
        const manager = managerOf(asked);
        if (manager === null)
            return [];
        if (off)
            return [offRow];
        if (outside)
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
                // Once the launcher has gone, so 1Password's prompt is
                // never beneath it.
                later: true,
                run: () => provider.letIn(manager)
            })];
        }
        const byKey = {};
        for (const row of everyone)
            byKey[row.key] = row;
        return Managers.vaults(records.filter(record => record.manager === manager.name))
            .map(vault => {
                const rows = () => vault.records.map(record => byKey[record.key])
                    .filter(Boolean);
                const part = thing({
                    key: `passwords:vault:${manager.name}:${vault.id}`,
                    name: Words.name("passwords:vault", "", { vault: vault.name }),
                    verb: Words.verb("passwords:vault"),
                    glyph: Words.glyph("passwords:vault"),
                    hue: "machine",
                    side: Managers.holding(vault.records.length,
                        Words.of("passwords:vault").said ?? {}),
                    nested: true,
                    stays: true,
                    verbs: rows,
                    targets: rows,
                    run: () => provider.launcher?.enter(part)
                });
                return part;
            }).concat(attention, [checkRow]);
    }
}
