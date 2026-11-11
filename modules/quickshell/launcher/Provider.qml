// The shape of one kind of thing the launcher finds: windows, applications,
// actions, the session. Each kind is a file starting from this one, listed
// in jitsusama.quickshell.launcher.providers, so leaving one out of the list
// removes it whole, and a machine can add its own beside them:
//
//     Provider {
//         things: [thing({ key: "mine:hello", verb: "say", name: "Say hello",
//                          glyph: "…", hue: "voice", run: () => … })]
//     }
//
// A thing is a row: what learning knows it by (key), what thing it is now
// (is, when that's not its key: `window:<niri id>` for a window, so a pi
// session's window, its Agents row and its quest are one row, the one
// with the highest `rich`), what Return does to it
// (verb), its name and what else finds it (keywords), its glyph and the
// theme's colour for its kind (hue), its chord or what it is (chord, side),
// what Return and Shift+Return run (run, below), whether running it waits
// for the launcher to be gone (later) or happens while it still holds the
// keys (now, for the clipboard), where it is (context), the collection it
// lives in as well, by its provider's name (inside), so a search that
// finds the collection whole lists it only there, and what else can
// be done to it (verbs, a function giving rows of the same shape), nested
// when those rows are things with verbs of their own, a level down. A verb
// that needs a target asks for one (ask) from the rows `targets` gives,
// each with a `note` for the footer, and is named on Return by `verb`.
// One that needs words instead, such as a password, asks for them (ask)
// in the field, dotted when `secret`, and hands them to `given`. A thing
// whose outcome takes a while `stays`, keeping the launcher open, and its
// `state()` tells the footer how it's going.
//
// A provider can answer rather than find: `answer(typed)` gives what's
// typed plainly asks for, { headline, copy, rows, note }, or null. The
// headline is drawn after what's typed and copied by Return, or, for an
// answer that does something, done by its `run` under its `verb`; the rows,
// made with copyRow, carry the detail. One that has to ask the network
// answers at once with what it has, `asking` true while it waits, and says
// `answered` when more arrives, so the launcher asks again; `ask` keeps
// what a URL said for a while. One asked by words rather than a pattern
// lists them as its `questions`, best first, and answers each read
// plainly; the start of one, three letters or more, shows its answer too,
// asked by the first it starts, so a word half remembered still finds it.
//
// Long because it's the one shape every source is written to: split, a
// source's author would read half of what it promises and miss the rest.
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../ProgramGlyphs.js" as ProgramGlyphs
import "Asking.js" as Asking
import "Numbers.js" as Numbers

Scope {
    id: provider

    // Its settings from Nix, as the option gives them.
    property var settings: ({})
    // How its matches rank against the rest's, as a multiple of their score.
    property real weight: 1
    // The launcher, for what a thing needs of it: the lock, opening below.
    property var launcher: null
    // Whether what it shows is still being read for the first time, as
    // Machine's daemon's first answer is, so a terminal's `launcher show`
    // asks again for its figures rather than saying the placeholder
    // (Launcher.qml's shownToTerminal). One that reads nothing it waits
    // on leaves it false.
    property bool stillReading: false
    property var things: []
    // The options :set takes from it, by name, as the command line reads
    // them (SetWords.setting): { kind: "switch" | "number" | "choice",
    // takes, least, most, values, fixed, stepsOnly, ends (a switch whose
    // off ends what it holds, so its no form is never run by Return, as
    // Wi-Fi's drops the network), now: () => where it
    // stands or null where it can't be read, choices: () => the values
    // :set wifi= lists, set: (to, change) => "" or what to say }, so
    // :set nowifi turns off what its Wi-Fi Off row does. Each is still
    // found as a row.
    property var setOptions: ({})
    // The desktop's actions its rows stand for, by what they do (Shortcuts'
    // `does`), each row drawing the action's chord, so the action is listed
    // only here rather than twice under two names: the session's Lock is
    // Super+L's "Lock the screen".
    property var draws: []
    signal answered
    // Its search found more after it was asked, as People's does once the
    // people daemon answers, so what's typed is searched again: ranked as
    // a key's search is when `fresh`, come so soon after the key that it
    // was hardly seen, else in place.
    signal foundMore(bool fresh)
    // Whether its search of these letters waits on an answer it asks a
    // daemon for, asked now if it isn't yet, so the launcher holds the
    // key's list for it a moment (Launcher.qml's answersWaitedFor) and
    // all of the key's rows land at once; it says so with foundMore once
    // the answer comes. Asked of every source at home, and inside a
    // place of the place's own.
    function awaits(letters) {
        return false;
    }
    // A collection's things changed while it might be open.
    signal changed

    // An answer worked out too late to say, as the maths' is when the
    // machine stalls past its patience, is asked again a moment later, a
    // few times at most for what's typed, rather than left unsaid until
    // the next key (the net's vm-desktop-looks, 2026-11-07: "12*7" lost).
    readonly property int lateTimes: 3
    property string lateFor: ""
    property int lateAsked: 0
    function late(typed) {
        if (typed !== lateFor) {
            lateFor = typed;
            lateAsked = 0;
        }
        if (lateAsked >= lateTimes || askAgain.running)
            return;
        lateAsked += 1;
        askAgain.restart();
    }

    // The shell's maths as an answer asks it for what's typed, a late sum
    // noted so the answer is asked again.
    function heeding(maths, typed) {
        return {
            calculate: expression => {
                const worked = maths.calculate(expression);
                if (worked.late)
                    late(typed);
                return worked;
            }
        };
    }

    Timer {
        id: askAgain

        interval: 100
        onTriggered: provider.answered()
    }

    // Says `changed` only when what its rows show has, for a collection
    // whose rows are made again whenever anything they're made from moves,
    // as the agents' are every few seconds, since each change searches an
    // open list again. What they show is each row's identity, name, what
    // Return does, words at the right and after it, note, glyph and how
    // far in it sits, as one
    // string; and for the row the launcher has stepped into, what it holds,
    // as far down as it's held, since that can change while the row reads
    // the same, as a window's do when it's floated or moved. Only the rows
    // held are asked what they hold, as the launcher would ask them again,
    // so a change makes no row's verbs that nobody's looking at.
    property string lastSaid: ""

    function changedTo(rows) {
        const now = says(rows, heldKeys(), true);
        if (now === lastSaid)
            return;
        lastSaid = now;
        changed();
    }

    // The keys of the rows held below its collection in the launcher, in
    // order, or none while it's closed or holds none of this provider's.
    function heldKeys() {
        const steps = launcher && launcher.open ? launcher.held ?? [] : [];
        const from = steps.findIndex(step => step.row.from === provider);
        return from < 0 ? [] : steps.slice(from + 1).map(step => step.row.key ?? "");
    }

    // What rows show, as `changedTo` compares them; the first row keyed as
    // the next held step says what it holds too: a thing's verbs, or, below
    // a row that isn't nested, a verb's targets, as the launcher steps in.
    // A side that runs on by itself (`counts`, Launcher.qml) is said by the
    // widest it reads until it grows a figure, since the launcher draws it
    // afresh, so only its growing changes what the rows say.
    function says(rows, held, things) {
        let open = held.length > 0;
        return rows.map(row => {
            const side = typeof row.counts === "function" ? row.sideWidest ?? "" : row.side ?? "";
            const own = [row.is ?? "", row.key ?? "", row.name ?? "", row.verb ?? "",
                side, row.count ?? "", row.note ?? "", row.glyph ?? "",
                row.depth ?? 0]
                .join("\u241f");
            if (!open || row.key !== held[0])
                return own;
            open = false;
            const asked = things ? row.verbs : row.targets;
            const inside = typeof asked === "function" ? asked() ?? [] : [];
            return `${own}\u241d${says(inside, held.slice(1), !!row.nested)}\u241c`;
        }).join("\u241e");
    }

    // A row of its own, by its key, drawn as `row` from now until it makes
    // it so, as an act asking a daemon has it (launcher/Becoming.js); null
    // for a row that goes. If it never comes, the foot names the row as
    // `name` and what was asked by the verb `act` names in words.json's
    // field.becoming.did. An act whose outcome can't be foretold, or that's
    // slower than a daemon, says so in `how`: { done, within } (Becoming.js).
    function becomes(key, row, name, act, how) {
        becomesAll([{ key: key, row: row }], name, act, how);
    }

    // Several rows at once, each { key, row }, as one act changes them all:
    // the desk light's Brightness, the level chosen in its step, and how to
    // put it out.
    function becomesAll(holds, name, act, how) {
        launcher?.becomesAll(provider, holds, Object.assign({ name: name, act: act }, how ?? {}));
    }

    // How a row of its own reads now, by its key, wherever it's shown, or
    // null where it makes none; undefined where it can't say, which leaves
    // the launcher to look for it among what it shows (Launcher.qml's
    // nowRow). An answer's parts are said only by the answer, so the desk
    // light says its own.
    function rowNow(key) {
        return undefined;
    }

    // What an act asked wasn't done, said in its own words, lowercase; ""
    // when it was, which says nothing.
    function refused(key, why) {
        if (why !== "")
            launcher?.becameNot(key, why);
    }

    function answer(typed) {
        return null;
    }

    // How its row says it stands at its right, at home and wherever else
    // it's found, as Agents says how its sessions stand together, or null
    // to say how many it holds (PlaceRow.js).
    function homeSays() {
        return null;
    }
    // The same as items, each `{ said, keep }`, as Choice.qml gives a side
    // way by them, or none to say it as it is.
    function homeItems() {
        return [];
    }
    // Whether homeSays says something, so home reads none of its rows: a
    // property rather than asking it, since what it says may cost a walk
    // of everything it holds. Such a source names what it holds without
    // making its rows, by a memberNames of its own given nothing.
    property bool saysAtHome: false

    // Whether it waits its turn to wake as a kept opening fills in, a few
    // sources a frame (Launcher.qml's wakeNext). Its own, so waking it
    // makes no other source's wanted again.
    property bool asleep: false
    // Whether its home rows are made with the launcher shut, a source a
    // turn as the shell starts (Launcher.qml's warmNext).
    property bool warming: false

    // Whether its rows are wanted now: walked from outside, searched, held,
    // named at home, or an act of theirs waiting to be agreed (Launcher.qml's
    // wants). A source whose rows cost much builds them only then, so a
    // change nobody sees builds none, and builds them once as they're
    // wanted again (Joel, 2026-11-08: "weird lag can happen"). One with no
    // launcher to ask, as a check's, always is.
    readonly property bool wanted: typeof launcher?.wants !== "function"
        || launcher.wants(provider)

    // Rows a binding makes, by name: made by `make` while wanted, and while not, what it made
    // last, so it reads `wanted` alone and a change nobody sees wakes nothing. One that has made
    // nothing makes it once regardless, for what reads it shut (Sound's spots). Shut on Joel's
    // shell, Sound, Spend and Now made 426 rows in four minutes (YQZGPC, 2026-11-10). Kept in a
    // plain object, so keeping notifies nothing. Each `make` is kept too, for `standing`.
    readonly property var madeLast: ({})
    readonly property var makers: ({})
    function whileWanted(name, make) {
        provider.makers[name] = make;
        if (provider.wanted || !(name in provider.madeLast))
            provider.madeLast[name] = make();
        return provider.madeLast[name];
    }

    // What whileWanted's `name` makes as it stands while the launcher looks through its rows
    // shut (looksThrough), and otherwise `bound`, the property it made: the first made may have
    // come before what they're made of was heard. Made here, not by its binding, since a
    // binding told to make them again as the launcher looks through them, as a source's change
    // can ask (Launcher.qml's onScreen), was told while it was still telling of its last: Sound
    // warned "Binding loop detected for property all" in a gate under load (2026-11-11).
    readonly property var lookingThrough: ({ now: false })
    function standing(name, bound) {
        if (!provider.lookingThrough.now || provider.wanted || !(name in provider.makers))
            return bound;
        provider.madeLast[name] = provider.makers[name]();
        return provider.madeLast[name];
    }

    // A row kept from the making before while what it's made from says the
    // same, `from` being all of that as text, or else made anew by `make`.
    // A keeper whose `from` is values rather than text passes `same`, how
    // two of them are told alike, such as PiSource.js's alike.
    // So a change remakes only the rows it changed, as Quests keeps its
    // own, rather than every row at every change. A keeper is a plain
    // object, `{ rows: {}, next: null }` (keeper()), held by a property
    // that's never reassigned, so writing into it notifies nothing and a
    // binding that keeps its rows there can't loop. Between keepingBegun
    // and keptOnly, each row asked for is kept in `next` too, and
    // keptOnly lets the rest go.
    function keeper() {
        return { rows: {}, next: null };
    }
    function keptOr(keeping, key, from, make, same) {
        const before = keeping.rows[key];
        const alike = before && (same ? same(before.from, from) : before.from === from);
        const kept = alike ? before : { from: from, row: make() };
        (keeping.next ?? keeping.rows)[key] = kept;
        return kept.row;
    }
    // keptOr for a row made from several things at once, `parts`, kept
    // while each is the same as it was, a row by its identity and a text
    // by what it says, so nothing has to be joined into one text to be
    // compared.
    function keptAlike(keeping, key, parts, make) {
        const before = keeping.rows[key];
        const same = before !== undefined && before.from.length === parts.length
            && parts.every((part, at) => part === before.from[at]);
        const kept = same ? before : { from: parts, row: make() };
        (keeping.next ?? keeping.rows)[key] = kept;
        return kept.row;
    }
    function keepingBegun(keeping) {
        keeping.next = {};
    }
    function keptOnly(keeping) {
        if (keeping.next === null)
            return;
        keeping.rows = keeping.next;
        keeping.next = null;
    }

    // The names its row says after its own: each thing it holds, but for
    // one folded into it, as Agents' Start a Session does something there
    // rather than being one of what it holds. Given what it holds where
    // its row has read it already, so a row reads it once; one that
    // saysAtHome is asked with nothing, and names them from elsewhere.
    function memberNames(held = everything()) {
        return held.filter(one => one.folded !== true).map(one => one.name);
    }

    // The one row a source ends every search at the top with, after
    // everything found, as Search Slack for what's typed is (VKCF3Z,
    // d7c0f933), or null. Asked each time the list is put together, so it
    // reads only what the source already holds.
    function last(typed) {
        return null;
    }

    property var questions: []

    // What's typed, read as answers read it.
    function plainly(typed) {
        return Fuzzy.plainly(typed);
    }

    // Which of its questions what's typed asks, read plainly, or its
    // plural ("fans" asks "fan"), or "", so an answer asked by words
    // answers exactly the words its start is found by.
    function asks(typed) {
        const text = Fuzzy.plainly(typed);
        return questions.find(question => question === text || `${question}s` === text) ?? "";
    }

    // Things worth typing to try it, as its data: the field no longer
    // suggests them at home, since the commands category lists each typed
    // command with its form (Words.qml's commands).
    property var examples: []

    // A provider with too many things to mix into the main search, such as
    // emoji, is a collection: { noun, title, glyph, ask }, its title naming it
    // on the home screen, `words` the others a person might type for it, by
    // which and by its title and noun a search goes straight to it whole,
    // `empty` saying what it says with nothing in it,
    // and searched false if it adds no row to a search, only opened by its
    // chord or from home. Its members stay out of the main list, though its
    // `things` don't. In the main search the launcher asks `gather(typed)`
    // for its matches, best first, with how scattered the best is as
    // Fuzzy.js grades a match, { rows, grade }, and shows the best as one row
    // with a count of the rest, below the main search's matches as tidy and
    // above any looser; the right arrow steps into all of them, with what
    // was typed kept, and `search(typed)` narrows them there. `everything()`
    // gives them all once nothing is typed.
    property var collection: null

    function search(typed) {
        return { rows: [], grade: 0 };
    }

    // What a search has to look through: every word typed has to start a
    // word of a row, so typing on only ever finds fewer, and while what's
    // typed goes on from the last search of the same rows, only the rows
    // that one matched are looked at again, as Emoji's are. Rows made
    // anew, as a new notice makes them, are looked through whole. A search
    // says what it matched, at any grade, with `matchedIn`.
    property var lastMatched: ({ from: null, words: "", rows: [] })

    function poolFor(rows, typed) {
        const words = Fuzzy.words(typed).join(" ");
        const last = lastMatched;
        return last.from === rows && last.words !== "" && words.startsWith(last.words)
            ? last.rows : rows;
    }

    function matchedIn(rows, typed, matched) {
        lastMatched = { from: rows, words: Fuzzy.words(typed).join(" "), rows: matched };
    }

    // Its row in the main search: its matches, unless it answered what's
    // typed, since its answer above them already lists them.
    function gather(typed) {
        return answer(typed) ? { rows: [], grade: 0 } : search(typed);
    }

    function everything() {
        return [];
    }

    // The rows its search looks through, whose texts the launcher makes
    // ready while it's closed: everything, unless it searches more than it
    // lists, as Quests searches its finished quests too. Each is as it stands, not as first
    // made: Sound's, first made before the sound server said any device, were none, so the
    // first search after a restart made every device's text (2026-11-11).
    function looksThrough() {
        const was = lookingThrough.now;
        lookingThrough.now = true;
        try {
            return everything();
        } finally {
            lookingThrough.now = was;
        }
    }

    // Makes ready the texts of one of those rows that its search reads, each
    // made as it's first read: its name, and its other words for one that
    // has them.
    function readyTexts(row) {
        return [row.prepared, row.also];
    }

    // What a collection's row says at its right where it says nothing of
    // how it stands: how many it holds, grouped by thousands, of what its
    // row has read where it's given.
    function counted(held = everything()) {
        return countedAs(held.length);
    }

    // A count as the collection says one, grouped by thousands, with what
    // it counts where its name doesn't say it (VKCF3Z, 2026-11-08: "78
    // keys"), as Numbers.js's counted says it.
    function countedAs(size) {
        return Numbers.counted(size, collection);
    }

    // What a collection's row says below it while it's picked, as the
    // queue says how much room the machine has: nothing, for most.
    function noted() {
        return "";
    }

    // A collection was stepped into, so all of it has been seen.
    function opened() {
    }

    // The key that opens a collection, pressed while the launcher shows,
    // `inside` it or showing anything else. One that answers true has done
    // what the key means there; false steps into it, as the key does with
    // the launcher closed. Capture's takes the screen as it is.
    function pressedOver(inside) {
        return false;
    }

    // What one of its rows is, in a word, said beside the field while one
    // is selected so a completion says what sort of thing it is: "window",
    // "program", "emoji". An answer's rows are called answers wherever they
    // come from, so a provider that only answers leaves it empty.
    property string what: ""

    // How many rows this source has made since the shell started, so the
    // launcher can say which made what as it opened (Launcher.qml's
    // madeSince), and the records it looked at. Plain counts in an object,
    // so making a row notifies nothing.
    readonly property var tally: ({ rows: 0, looked: 0 })

    // A collection's row: a thing whose name is made ready to match only
    // when it's first drawn, since a collection has thousands.
    function member(fields) {
        tally.rows += 1;
        const made = Object.assign({
            what: provider.what,
            chord: [],
            side: "",
            count: "",
            context: "",
            note: "",
            below: null,
            verbs: null,
            plain: false,
            later: false,
            now: false,
            keywords: null,
            weight: provider.weight
        }, fields);
        let prepared = null;
        Object.defineProperty(made, "prepared", {
            enumerable: true,
            get: () => prepared ?? (prepared = Fuzzy.kept(made.name))
        });
        return made;
    }

    // A row made again with each of `getters` read as it's asked for, in
    // place of the field it has. Never turn a field a row has into a getter
    // with Object.defineProperty: Qt 6.12 keeps the slot of a getter's
    // setter once for every shape grown from the one it was first turned
    // in, so the next such turn writes over another field, or past the
    // row's end, which crashed the shell on 2026-11-11. Its other fields'
    // getters are copied unread.
    function gotten(row, getters) {
        const fields = Object.getOwnPropertyDescriptors(row);
        for (const name of Object.keys(getters))
            fields[name] = { enumerable: true, configurable: true, get: getters[name] };
        return Object.defineProperties({}, fields);
    }

    // A file's text as it is now. A FileView blocks only on its first load:
    // after that, a reload or a new path loads in the background and text()
    // still says what the last load read, unless the load is waited for.
    function read(file) {
        file.reload();
        file.waitForJob();
        return file.text();
    }

    // What a JSON file another program writes says, or nothing at all when
    // it's missing, half written or not JSON, so a reader asks only for the
    // fields it needs and finds them or doesn't.
    function parsed(text) {
        try {
            return JSON.parse(text) ?? {};
        } catch (error) {
            return {};
        }
    }

    // A row whose Return copies `copy`, or its name, set while the launcher
    // holds the keys, which a Wayland program needs to set the clipboard.
    // A row with an empty `copy` says something and copies nothing.
    // It's a figure, saying a value, so it wears no glyph unless it does
    // more (verbs) or says it's an act to copy (figure false), as a theme to
    // paste is (Choice.qml).
    // A row that only shows a value copies it (Joel, 2026-10-26: every row
    // has a Return act, and one that only shows a value copies it): what
    // it says to copy, or else its value as it reads when Return comes,
    // so a live figure copies what it shows. One still being read copies
    // nothing yet, and nor does one whose side says how things stand
    // rather than a value (`copiesNothing`, as a reader not answering):
    // pasted, those words would be no use anywhere, so Return only closes.
    function copyRow(fields) {
        const valueOf = side => typeof side === "string" && side !== "\u2026" ? side : "";
        const textOf = from => fields.copiesNothing === true ? ""
            : (from.copy ?? from.name) === "" ? valueOf(from.side)
            : from.copy ?? from.name;
        const row = thing(Object.assign({
            verb: textOf(fields) === "" ? "close" : "copy",
            hue: "voice",
            figure: true,
            plain: true,
            now: true,
            // What it copies as it says it then, for y to join with other
            // rows' copies.
            copies: () => textOf({ copy: fields.copy, name: fields.name, side: row.side }),
            run: () => {
                const text = textOf({ copy: fields.copy, name: fields.name, side: row.side });
                if (text !== "")
                    Quickshell.clipboardText = text;
            }
        }, fields));
        return row;
    }

    // What y takes from a row that copies `text`, as copyRow would: plain
    // fields rather than a row made searchable, since y over hundreds asks
    // every row (Launcher.qml's actionsOf), and each of a row's verbs made
    // searchable costs about 5 microseconds.
    function copyOperand(key, text) {
        return {
            key: key,
            verb: "copy",
            copies: text,
            run: () => {
                if (text !== "")
                    Quickshell.clipboardText = text;
            }
        };
    }

    // A figure holding more, as a CPU's Cores or the weather's Rain: it says
    // a value, wears its glyph dimmed (Choice.qml's "more"), and Return
    // and the right arrow both step into what `inside` gives, made then,
    // its copy a row in there after what can be done (Launcher.qml's
    // copiesOnly; Joel, 2026-10-28: Return steps in where the right arrow
    // would open, and "the copy at the top of most screens is confusing"). A setting, as a display's Scale (`offers`), or one with no
    // value yet steps in on Return as well, choosing being a setting's act.
    // The launcher asks a row for its verbs at the top and for its targets
    // a step in (enter), so it gives the same rows as both, or a part such
    // as Rain, a step inside the weather, would open nothing.
    function holderRow(fields, inside) {
        const row = copyRow(Object.assign({ copy: "", hue: "machine", note: "" }, fields));
        if (!row.offers && row.verb === "copy")
            return Object.assign(row, { verbs: inside, targets: inside });
        return entering(row, inside);
    }

    // A thing holding more, as a Bluetooth device or a camera, whose side
    // says how it stands rather than a figure: Return steps in, as the
    // right arrow does, to what `inside` gives.
    function thingRow(fields, inside) {
        return entering(copyRow(Object.assign({ copy: "", hue: "machine", note: "" }, fields)),
            inside);
    }

    // A row made to step in on Return to what `inside` gives.
    function entering(row, inside) {
        return Object.assign(row, {
            verb: "more",
            stays: true,
            now: false,
            verbs: inside,
            targets: inside,
            run: () => provider.launcher?.enter(row)
        });
    }

    // A part's name as the vocabulary says it within `scope`, as
    // displays:depth is "Colour depth", or the fact's own name capitalised
    // for one it doesn't know yet, so a new fact still reads as a name. It
    // asks the launcher's vocabulary, since this file mustn't import qs
    // (Launcher.qml's vocabulary says why).
    function partNamed(scope, id) {
        const key = `${scope}:${id.toLowerCase().split(" ").join("-")}`;
        const named = provider.launcher?.vocabulary?.name(key) ?? key;
        return named !== key ? named : id.charAt(0).toUpperCase() + id.slice(1);
    }

    // What a daemon says of one of the provider's things, as a list of
    // { name, value }, each a part named first and saying its value, its
    // Return copying the value: the ids in `first` lead, as they're what's
    // asked most, and the rest keep the daemon's order. One place makes
    // them, so every provider's facts read the same way round.
    function factParts(scope, thing, facts, first) {
        const leading = first ?? [];
        const said = (facts ?? []).map(fact => ({ id: fact.name.toLowerCase(), value: fact.value }));
        return said.filter(fact => leading.includes(fact.id))
            .concat(said.filter(fact => !leading.includes(fact.id)))
            .map(fact => copyRow({
                key: `fact:${scope}:${thing}:${fact.id}`,
                name: partNamed(scope, fact.id),
                side: fact.value,
                copy: fact.value
            }));
    }

    // What each URL said, and when: { pending, value, failed, at }.
    property var asked: ({})

    // What a URL said as JSON, asking it again once `fresh` milliseconds
    // have passed, and only then: the value it last gave, if any, comes
    // back at once, and `answered` says when one saying something new has
    // arrived (Asking.js's heard). One
    // that hasn't answered in `patience` milliseconds has failed, and a
    // failure is asked again after ten seconds.
    function ask(url, fresh) {
        const now = Date.now();
        const kept = asked[url];
        if (Asking.stands(kept, now, fresh, 10000))
            return kept;
        const entry = Asking.askedAgain(kept, now);
        asked[url] = entry;
        const request = new XMLHttpRequest();
        request.onreadystatechange = () => {
            // A late answer, once it was given up on, changes nothing.
            if (request.readyState !== XMLHttpRequest.DONE || !entry.pending)
                return;
            delete provider.requests[url];
            if (Asking.heard(entry, { status: request.status, text: request.responseText },
                Date.now()))
                provider.answered();
        };
        requests[url] = request;
        request.open("GET", url);
        request.send();
        if (!deadline.running) {
            deadline.interval = patience;
            deadline.start();
        }
        return entry;
    }

    readonly property int patience: 5000
    // What's still being asked, by URL, to give up on.
    property var requests: ({})

    // Gives up on what's taken its patience, as failed, and waits for the
    // next one still being asked, so it runs only while something is.
    function giveUp() {
        const now = Date.now();
        let soonest = null;
        let gaveUp = false;
        for (const url of Object.keys(asked)) {
            const entry = asked[url];
            const left = Asking.overdueIn(entry, now, patience);
            if (left === 0) {
                if (Asking.gaveUp(entry, now))
                    gaveUp = true;
                requests[url]?.abort();
                delete requests[url];
            } else if (left !== null && (soonest === null || left < soonest)) {
                soonest = left;
            }
        }
        if (soonest !== null) {
            deadline.interval = soonest;
            deadline.start();
        }
        if (gaveUp)
            provider.answered();
    }

    Timer {
        id: deadline
        onTriggered: {
            provider.giveUp();
        }
    }

    // The acts this provider gives programs (programActs) that are found
    // from the top as well, each { program, key }: the program's desktop
    // entry id and the act's key, as Meet's Join by Code is. Programs makes
    // a row of each as this changes, never on a key, so it's changed only
    // as an act comes or goes, as Copy a New Meeting's Link does with
    // Meet's scope; acts that change all the time, as Slack's Unread
    // does, aren't listed (Applications.qml).
    property var foundActs: []

    // The acts foundActs names, as rows, for a program. Programs asks it
    // inside a binding, so it may read only what the rows say and write
    // nothing; by default it's every act the program is given
    // (programActs), and a provider whose programActs asks or makes more,
    // as Neovim's asks open-now and makes each open file's row, gives the
    // listed ones alone.
    function foundActsOf(program) {
        return typeof provider.programActs === "function" ? provider.programActs(program) : [];
    }

    // The programs this provider gives acts to (programActs) whose desktop
    // entries hide themselves, as Tuple's does, having no window to open:
    // Programs lists them all the same, each stepping in on Return rather
    // than starting (Applications.qml).
    property var hiddenPrograms: []

    // An application's own glyph, by its desktop entry's id or a window's
    // app id, from the one table its notices read too (ProgramGlyphs.js),
    // with a machine's own from `settings.glyphs`; anything it doesn't know
    // wears the theme's program mark, through the launcher, since this file
    // mustn't import qs.
    function programGlyph(id) {
        const marks = provider.launcher?.marks ?? {};
        const found = ProgramGlyphs.glyph(id, settings.glyphs, marks);
        return found === "" ? marks.program ?? "" : found;
    }

    // A window's title without its program's name trailing it, as Chrome
    // adds " - Google Chrome", since the glyph and the row already say
    // whose window it is.
    function tidy(title, program) {
        if (program === "")
            return title;
        for (const joint of [" - ", " \u2013 ", " \u2014 ", " | "])
            if (title.endsWith(joint + program))
                return title.slice(0, title.length - joint.length - program.length);
        return title;
    }

    // Where a window is, as its row says it: its workspace when that isn't
    // the one in front (here), and when other windows read the same (alike,
    // how many do), its column and how far down it, the top one only its
    // column, so no two rows read alike. The Windows collection and a
    // program's own list say it the same way.
    function placeOf(window, workspace, here, alike) {
        const tile = window.layout?.pos_in_scrolling_layout ?? [];
        const column = tile[0];
        const elsewhere = workspace && !(here && workspace.id === here.id);
        const down = tile[1] > 1 ? `, ${Numbers.ordinal(tile[1])} down` : "";
        return [elsewhere ? `workspace ${workspace.idx}` : "",
            alike > 1 && column ? `${Numbers.ordinal(column)} column${down}` : ""]
            .filter(part => part !== "").join(", ");
    }

    // What a desktop entry runs, spelled out for niri to spawn: in kitty
    // when the entry asks for a terminal, and from its own directory when it
    // names one.
    function command(entry) {
        const program = entry.runInTerminal ? ["kitty"].concat(entry.command) : entry.command;
        if (entry.workingDirectory === "")
            return program;
        return ["sh", "-c", "cd -- \"$0\" && exec \"$@\"", entry.workingDirectory].concat(program);
    }

    // A row with what's left unsaid filled in, its name and keywords
    // prepared for matching once rather than on every key.
    function thing(fields) {
        tally.rows += 1;
        const made = Object.assign({
            // An act done to a thing (its key `verb:`) isn't of the
            // thing's kind: Return does what its name says, so the foot
            // says "go to" for Go To, not a thing's "open".
            what: String(fields.key ?? "").startsWith("verb:") ? "" : provider.what,
            chord: [],
            side: "",
            count: "",
            context: "",
            note: "",
            below: null,
            verbs: null,
            plain: false,
            later: false,
            now: false
        }, fields);
        made.weight = provider.weight;
        made.prepared = Fuzzy.prepare(made.name);
        // Its keywords beside its name, so a search can say some words of
        // each: "wifi radio" finds Wi-Fi off, "radio" being a keyword.
        // A word of its name joined by a hyphen is found closed up too, as
        // it's typed: "wifi" finds Wi-Fi.
        const closed = Fuzzy.words(made.name).filter(word => word.includes("-"))
            .map(word => word.replace(/-/g, ""));
        made.keywords = fields.keywords || closed.length > 0
            ? Fuzzy.prepare([made.name].concat(closed, fields.keywords ? [fields.keywords] : [])
                .join(" "))
            : null;
        return made;
    }
}
