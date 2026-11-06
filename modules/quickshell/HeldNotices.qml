// The notices while Super+N holds them: Vim's quickfix list (VKCF3Z's
// vi-paradigm spec, Notices), a line a card, the newest on line 1, and the
// cursor card whole under its line. It's the stack's other face
// (Notifications.qml), drawn in the stack's place while the keys are held,
// and every key does what Vim's does (NoticeKeys.js holds them, `pressed`
// does them, NoticeList.js works out where they land).
//
// Normal stands on a card: j and k move, Return goes to its sender, l
// steps into its acts, d is done with it (marked read where its sender
// offers Read, which is sent at once, and then forgotten), y copies what it
// said, u brings cards back. V chooses several for d or y, / finds among
// them, narrowing the lines as it's typed, and : is the command line, with
// a menu of every command it takes (Ex.js reads it). :q, ZZ, Space w and
// Super+N let them go; Escape at rest does nothing, as Vim's, and q says
// it does nothing.
//
// It's the launcher's card in shape (VKCF3Z's family cut, 2026-11-05): a
// head reading Notices, where / types, as a place does there; a hairline;
// the lines; a hairline; the foot. Each line is the launcher's row: the
// sender's glyph, who or what at Name, its first words at
// Words and when at Note, after a gutter of relative numbers, as Joel's
// Neovim sets number and relativenumber. The foot says how things stand,
// the mode's word in a room of its own, and the cursor card's hints from
// the same table that does the keys, Space ? last.
pragma ComponentBehavior: Bound
import Quickshell
import QtQuick
import "Ex.js" as Ex
import "NoticeKeys.js" as NoticeKeys
import "NoticeList.js" as NoticeList
import "Search.js" as Search
import "launcher/Notice.js" as Notice

Floating {
    id: held

    // The stack these are (Notifications.qml), for when each arrived, what
    // each opens, the history it hands on to, and whether it holds them.
    required property var stack
    // The notices on screen, as their notifications.
    property var live: []

    keys: true

    // How the keys stand: the mode, and what's begun in it.
    property string mode: "normal"
    property var standing: NoticeKeys.rest()
    readonly property bool pending: standing.operator !== ""

    // What u and Ctrl+R move between, and the cards it holds away from the
    // list, let go for good as the keys go back.
    property var changes: NoticeList.changes()
    readonly property var awayIds: NoticeList.away(changes.done)
    // The last change, for . to do again.
    property var lastChange: null

    // The lines, the newest first. While visual, an operator or the command
    // line is at work they hold their order, and what arrives waits behind
    // a line saying how many came.
    property var frozen: null
    readonly property var all: NoticeList.ordered(live.map(one => one.id)
        .filter(id => !awayIds.includes(id)), stack.arrived)
    readonly property var rows: frozen === null ? all : frozen.filter(id => all.includes(id))
    readonly property int arrivals: frozen === null ? 0
        : all.filter(id => !frozen.includes(id)).length

    // The find: what was found, kept once its line is left, and the line
    // while it's typed. The lines narrow to what holds it.
    property string found: ""
    property var findLine: NoticeKeys.line("")
    readonly property bool finding: mode === "find" || mode === "command" && commandFrom === "find"
    readonly property string pattern: finding ? findLine.text : found
    readonly property var listed: pattern === "" ? rows : rows.filter(id => held.holds(id, pattern))

    // The cursor, bound to its card, and where it was, for when its card
    // goes.
    property var cursorId: null
    property int cursorAt: 0
    readonly property int cursor: {
        const at = listed.indexOf(cursorId);
        return at >= 0 ? at : Math.max(Math.min(cursorAt, listed.length - 1), 0);
    }
    readonly property var current: notice(listed[cursor])
    // Where it was is kept once the binding's settled, since the cursor
    // reads it, and the cursor's bound to the card it came to, as a find
    // narrowing the lines leaves it on what was found, as Vim's / does.
    onCursorChanged: Qt.callLater(remember)
    function remember() {
        cursorAt = cursor;
        if (listed[cursor] !== undefined)
            cursorId = listed[cursor];
        follow();
    }
    onCurrentChanged: {
        if (stack.holding && current)
            Report.say(`notice cursor ${current.summary}`);
    }

    // Visual's other end, and the last selection, for gv and '< '>.
    property var anchorId: null
    property var lastSelection: null
    readonly property var span: mode === "visual"
        ? NoticeList.chosen(Math.max(listed.indexOf(anchorId), 0), cursor) : null
    readonly property var selected: span === null ? [] : NoticeList.between(listed, span)
    onSelectedChanged: if (mode === "visual") Report.say(`notice selects ${selected.length}`)

    // The view: the first line shown, kept around the cursor, and how many
    // show, as many as fit the screen's height up to eight, and never more
    // than the frame has room for inside the screen's rung.
    property int firstShown: 0
    readonly property int room: Math.max(Math.min(NoticeList.room,
        Math.floor((screenHeight * linesShare) / rowHeight),
        Math.floor(listTall / rowHeight)), 1)
    // How much of the screen's height the lines take at most, leaving the
    // cursor card, the menu and the foot room beneath them. Half, so eight
    // of the launcher's rows fit a screen 768 tall: seven would keep the
    // cursor on the middle line, scrolloff held to half the view.
    readonly property real linesShare: 0.5
    property real screenHeight: 768
    // While the command line is up its menu takes the lines' room, and at
    // rest the cursor's open card does, so they give way round the cursor,
    // down to one, and the frame never passes the screen's rung on a short
    // screen.
    readonly property int shownRoom: mode === "command" ? commandRows
        : Math.max(1, Math.min(room, Math.floor((listTall - cardShown) / rowHeight)))
    // How tall the cursor's card stands under its row, as the card says it
    // with the notice it opens, so a card going away never clears the one
    // that replaced it.
    property var cardOf: null
    property real cardTall: 0
    // Taken a moment after the card says it, never while the rows it
    // shapes are being laid, and only when it changes, so a card laid
    // again with the same height leaves them be.
    function cardSays(of, tall) {
        if (cardOf !== of)
            cardOf = of;
        if (cardTall !== tall)
            cardTall = tall;
    }
    readonly property real cardShown: cardOf !== null && cardOf === listed[cursor]
        && mode !== "command" ? cardTall : 0
    readonly property int shownTop: Math.max(firstShown, cursor - shownRoom + 1)
    readonly property var inView: listed.slice(shownTop, shownTop + shownRoom)
    function follow() {
        firstShown = NoticeList.viewTop(firstShown, cursor, listed.length, room);
    }
    onListedChanged: Qt.callLater(follow)

    // Whether the gutter counts from the cursor, as Space u l switches.
    property bool relative: true

    // The cursor card's own acts as the commands that do them, as the card
    // lists them under its words and : puts them first in the menu: a
    // card's acts are commands, never a row stepped into (Joel, 2026-11-05:
    // "where in vi does that step into shit?"). Return goes to its sender.
    readonly property var cardCommands: {
        const one = current;
        if (!one)
            return [];
        const replies = one.hasInlineReply || replyOffer(one) ? ["reply"] : [];
        return replies.concat(offered.map(offer => offer.command)).map(command => `:${command}`);
    }
    readonly property string cardSays: cardCommands.join(" \u00b7 ")
    onCardSaysChanged: if (stack.holding)
        sayOffers()
    function sayOffers() {
        Report.say(`notice offers ${cardSays === "" ? "nothing" : cardSays}`);
    }

    // Space ? lists the keys in the lines' place, scrolling there.
    property bool listing: false
    property int listTop: 0
    readonly property var keyList: NoticeKeys.keyList()
    readonly property var listShown: keyList.slice(listTop, listTop + listRoom)
    onListingChanged: {
        listTop = 0;
        Report.say(listing ? "notice lists keys" : "notice hides keys");
    }
    onListTopChanged: if (listing) Report.say(`notice lists keys from ${listTop + 1}`)

    // The command line: what's typed, where it was opened from, which menu
    // line is chosen (-1 for none) and whether the menu's put away.
    property var commandLine: NoticeKeys.line("")
    property string commandFrom: "normal"
    property int menuChosen: -1
    property bool menuAway: false
    // While Tab chooses, the menu stays what was typed before it, as Vim's
    // wildmenu cycles through every match of what was typed: narrowed by
    // the line it writes, a second Tab would find only the first's choice.
    property string menuTyped: ""
    readonly property var menuFound: NoticeList.menu(entries,
        menuChosen >= 0 ? menuTyped : commandLine.text)
    onMenuChosenChanged: if (menuChosen >= 0)
        Report.say(`notice menu chose ${menuFound.shown[menuChosen]?.command ?? ""}`)

    // A reply's prompt on the card it answers, and its words before the
    // last change in normal over them, for u.
    property var replyTo: null
    property var promptLine: NoticeKeys.line("")
    property string promptBefore: ""

    // What the foot says until the next key: what was just done, or Vim's
    // error for what can't be.
    property string said: ""
    property bool saidWrong: false
    onSaidChanged: if (said !== "") Report.say(`notice says ${said}`)

    onModeChanged: {
        refreeze();
        Report.say(`notice mode ${mode}`);
    }
    onPendingChanged: refreeze()
    onPatternChanged: if (pattern !== "") Report.say(`notice finds ${pattern} in ${listed.length}`)

    function refreeze() {
        const holds = mode === "visual" || mode === "command" || pending;
        frozen = holds ? frozen ?? all.slice() : null;
    }

    // The keys taken, and the list as it was before: the cursor on the
    // card in front, which is the one Joel was looking at. Let go, the
    // cards done are let go for good.
    // Taken on the card it was on when let go, the cursor's card changes
    // nothing, so what it offers is said once the cursor has settled.
    function take(front) {
        reset();
        cursorId = front;
        Qt.callLater(sayOffers);
    }

    function reset() {
        mode = "normal";
        standing = NoticeKeys.rest();
        frozen = null;
        found = "";
        findLine = NoticeKeys.line("");
        commandLine = NoticeKeys.line("");
        replyTo = null;
        listing = false;
        lastChange = null;
        lastSelection = null;
        anchorId = null;
        said = "";
        firstShown = 0;
    }

    function letGo() {
        stack.dismiss(awayIds);
        changes = NoticeList.changes();
        reset();
    }

    // The live notice an id names, or null.
    function notice(id) {
        return live.find(one => one.id === id) ?? null;
    }

    function senderOf(id) {
        const one = notice(id);
        return one ? Notices.sender(one.appName).name : "";
    }

    // Whether what a card says, or the name it's signed with, holds what's
    // found.
    function holds(id, sought) {
        const one = notice(id);
        return one !== null && Search.found(sought,
            `${stack.said(one)}\n${Notices.sender(one.appName).name}`).length > 0;
    }

    // The sender's own Read, where it offers one, which done sends at once,
    // as the launcher's d does (Notice.js).
    function readOffer(one) {
        return Notice.readOffer(one.actions);
    }

    // The sender's own Reply, where it offers one rather than a reply typed
    // here.
    readonly property var replyAct: /reply/i
    function replyOffer(one) {
        return one.actions.find(act => replyAct.test(act.identifier) || replyAct.test(act.text))
            ?? null;
    }

    // What d says it does to the cards it would take: read, where each
    // sender offers it, or else forget.
    function doneWord(ids) {
        return ids.length > 0 && ids.every(id => notice(id) && readOffer(notice(id)) !== null)
            ? "read" : "forget";
    }

    function plural(count, one, many) {
        return count === 1 ? one : many;
    }

    // Done with these: each read at once, by its sender where it offers
    // Read, and forgotten, kept for u until the keys go back; or forgotten
    // unread, as :forget does.
    function finish(ids, kind) {
        if (ids.length === 0) {
            said = "not here";
            return;
        }
        if (kind === "done") {
            for (const id of ids) {
                const one = notice(id);
                Notices.mark(id, true);
                const offer = one ? readOffer(one) : null;
                if (offer)
                    offer.invoke();
            }
        }
        const left = listed.filter(id => !ids.includes(id));
        const from = Math.min(...ids.map(id => listed.indexOf(id)).filter(at => at >= 0));
        cursorId = left[Math.min(from, left.length - 1)] ?? null;
        cursorAt = Math.max(Math.min(from, left.length - 1), 0);
        changes = NoticeList.changed(changes, { ids, kind });
        const count = ids.length;
        said = `${count} ${kind === "done" ? "done" : "forgotten"} \u00b7 u brings `
            + `${plural(count, "it", "them")} back`;
        Report.say(`notice ${kind === "done" ? "done" : "forgets"} ${count}`);
    }

    // What the cards said onto the clipboard, a blank line apart.
    function copy(ids) {
        const said = ids.map(id => notice(id)).filter(one => one !== null)
            .map(one => stack.said(one));
        if (said.length === 0)
            return;
        Quickshell.clipboardText = said.join("\n\n");
        held.said = `${said.length} copied`;
        Report.say(`copied card ${notice(ids[0])?.summary ?? ""}`);
    }

    // An operator on what its motion takes from the cursor, or on the
    // selection, as Vim's linewise d and y; done in visual, it ends it.
    function operate(operator, motion, count) {
        const list = { count: listed.length, top: firstShown,
            senders: listed.map(id => senderOf(id)) };
        const took = motion === "selection" ? span : NoticeList.taken(motion, cursor, count, list);
        const ids = NoticeList.between(listed, took);
        if (mode === "visual") {
            lastSelection = { anchorId, cursorId };
            mode = "normal";
        }
        if (operator === "copy") {
            cursorId = listed[took.from] ?? cursorId;
            copy(ids);
            return;
        }
        lastChange = motion === "selection"
            ? { operator, motion: "line", count: ids.length } : { operator, motion, count };
        finish(ids, "done");
    }

    function undo(count) {
        const undone = NoticeList.undo(changes, count);
        if (undone.moved === null) {
            said = "Already at oldest change";
            return;
        }
        changes = undone.history;
        const ids = NoticeList.away(undone.moved);
        cursorId = ids[0];
        said = `${ids.length} brought back`;
        Report.say(`notice brought back ${ids.length}`);
    }

    function redo(count) {
        const redone = NoticeList.redo(changes, count);
        if (redone.moved === null) {
            said = "Already at newest change";
            return;
        }
        const ids = NoticeList.away(redone.moved);
        const left = listed.filter(id => !ids.includes(id));
        cursorId = left[Math.min(cursor, left.length - 1)] ?? null;
        changes = redone.history;
        said = `${ids.length} taken again`;
    }

    // Goes to whoever sent the cursor card: what it's about, by its
    // sender's own act, or else the sender's window, giving the keys back
    // so what's typed next reaches it.
    function open() {
        const one = current;
        const offer = one ? stack.opens(one) : null;
        const window = one ? Notices.senderWindow(one.appName) : null;
        if (!one || !offer && !window) {
            said = "not here";
            return;
        }
        Notices.mark(one.id, true);
        if (offer)
            Notices.open(offer, one.appName);
        else
            Niri.act({ FocusWindow: { id: window.id } });
        stack.holding = false;
    }

    // :reply: words given are sent at once where the card takes a reply
    // typed here, or else a prompt opens on it; a sender whose own Reply
    // opens it elsewhere is gone to.
    function reply(words) {
        const one = current;
        if (!one) {
            said = "not here";
            return;
        }
        if (one.hasInlineReply) {
            if (words !== "") {
                send(one, words);
                return;
            }
            replyTo = one.id;
            promptLine = NoticeKeys.line("");
            mode = "insert";
            return;
        }
        const offer = replyOffer(one);
        if (!offer) {
            said = `${one.summary} takes no reply`;
            return;
        }
        Notices.mark(one.id, true);
        offer.invoke();
        stack.holding = false;
    }

    function send(one, words) {
        one.sendInlineReply(words);
        Notices.mark(one.id, true);
        replyTo = null;
        mode = "normal";
        said = `replied to ${one.summary}`;
        Report.say(`notice replied ${one.summary}`);
    }

    // The commands the : line takes here, each its full name, its fixed
    // short form (checked against Neovim's own) and whether it takes a
    // range; then what the cursor card's sender offers, each by its first
    // word, where that word isn't already one of these.
    readonly property var commands: [
        { command: "delete", short: "d", ranged: true },
        { command: "forget", short: "forg", ranged: true },
        { command: "yank", short: "y", ranged: true },
        { command: "reply", short: "rep", ranged: false },
        { command: "open", short: "ope", ranged: false },
        { command: "edit", short: "e", ranged: false },
        { command: "quit", short: "q", ranged: false },
        { command: "xit", short: "x", ranged: false },
        { command: "close", short: "clo", ranged: false },
        { command: "help", short: "h", ranged: false },
        { command: "undo", short: "u", ranged: false },
        { command: "redo", short: "red", ranged: false },
        { command: "set", short: "se", ranged: false }
    ]
    readonly property var offered: {
        const one = current;
        if (!one)
            return [];
        return Notice.offers(one.actions)
            .filter(offer => !replyAct.test(offer.text))
            .map(offer => ({
                command: offer.text.split(" ")[0].toLowerCase(),
                identifier: offer.identifier,
                text: offer.text
            }))
            .filter(offer => /^[a-z]+$/.test(offer.command)
                && !commands.some(ours => ours.command.startsWith(offer.command)
                    || offer.command.startsWith(ours.short)));
    }
    readonly property var commandNames: commands.concat(offered.map(offer => ({
        command: offer.command, short: offer.command, ranged: false
    })))

    // The menu: the cursor card's or the selection's own acts, Return's
    // first; then the list's; then what every surface has.
    readonly property var entries: {
        const one = current;
        const out = [];
        // As the launcher's menu (CommandMenu.qml) reads a line: its command
        // and short form, what follows them where the line names its
        // argument, its title and its key; `writes` is what Tab writes.
        const entry = (command, short, name, key, writes, after) => ({
            command, short, name, title: name, key, writes: writes ?? command,
            after: after ?? ""
        });
        if (one && (stack.opens(one) || Notices.senderWindow(one.appName)))
            out.push(entry("open", "ope", Words.name("verb:notice:open"), "\u21b5"));
        if (one && (one.hasInlineReply || replyOffer(one)))
            out.push(entry("reply", "rep", "Reply", ""));
        for (const offer of offered)
            out.push(entry(offer.command, offer.command, offer.text, ""));
        out.push(entry("delete", "d", "Done", "dd"),
            entry("forget", "forg", Words.name("verb:notice:forget"), ""),
            entry("yank", "y", Words.name("verb:notice:copy"), "yy"),
            entry("edit", "e", Words.name("verb:notices:all"), "", "e Notices", " Notices"),
            entry("global", "g", "On Each Found", "", "g/", "/pat/cmd"),
            entry("undo", "u", "Bring Them Back", "u"),
            entry("redo", "red", "Take Them Again", "Ctrl+R"),
            entry("quit", "q", "Let Them Go", "ZZ"),
            entry("help", "h", "These Keys", "Space ?"),
            entry("set", "se", "Relative Numbers", "Space u l", "set rnu!", " rnu!"),
            entry("{n}", "", "Go to Line n", "nG", ""));
        return out;
    }

    // The cards the command line acts on: the selection it was opened from,
    // or the cursor card.
    readonly property var targets: commandFrom === "visual" && lastSelection
        ? selectedOf(lastSelection) : [listed[cursor]]

    // What a chosen menu line acts on, as a drawer would name it.
    readonly property string actsOn: {
        const chosenEntry = menuFound.shown[menuChosen] ?? null;
        if (chosenEntry === null)
            return "";
        const ids = commandFrom === "visual" || commandLine.text.startsWith(Ex.selected)
            ? (lastSelection ? selectedOf(lastSelection) : [listed[cursor]]) : [listed[cursor]];
        const names = ids.map(id => senderOf(id)).filter(name => name !== "");
        return ids.length === 1 ? `on ${notice(ids[0])?.summary ?? ""}`
            : `on ${ids.length} notices: ${names.join(", ")}`;
    }

    function selectedOf(selection) {
        const from = listed.indexOf(selection.anchorId);
        const to = listed.indexOf(selection.cursorId);
        return from < 0 || to < 0 ? [] : NoticeList.between(listed, NoticeList.chosen(from, to));
    }

    // Runs what's typed on the command line, exactly, as Vim's Return does.
    function run(text) {
        const from = commandFrom;
        commandLine = NoticeKeys.line("");
        mode = from === "prompt" ? "prompt" : "normal";
        const asked = Ex.parse(text, commandNames);
        if (asked.error) {
            wrong(asked.error);
            return;
        }
        const marks = {};
        if (lastSelection) {
            const chosenIds = selectedOf(lastSelection);
            if (chosenIds.length > 0) {
                marks["<"] = listed.indexOf(chosenIds[0]);
                marks[">"] = listed.indexOf(chosenIds[chosenIds.length - 1]);
            }
        }
        const at = { cursor, count: listed.length, marks };
        if (asked.global) {
            everywhere(asked, at);
            return;
        }
        const rows = Ex.resolve(asked.range, at);
        if (rows.error && (asked.range !== null || listed.length > 0)) {
            wrong(rows.error);
            return;
        }
        const ids = (rows.rows ?? []).map(row => listed[row]);
        does(asked, ids, from);
    }

    function wrong(error) {
        said = error;
        saidWrong = true;
    }

    // :g/pat/cmd and :v: the inner command on every line in the range that
    // holds the pattern, or doesn't; with none, the lines narrow to it, as
    // Vim lists them.
    function everywhere(asked, at) {
        const rows = Ex.resolve(asked.range, at);
        if (rows.error) {
            wrong(rows.error);
            return;
        }
        const ids = rows.rows.map(row => listed[row])
            .filter(id => held.holds(id, asked.global.pattern) !== asked.global.invert);
        const inner = asked.global.command;
        if (inner.name === "") {
            found = asked.global.pattern;
            return;
        }
        if (!["delete", "forget", "yank"].includes(inner.name)) {
            wrong("E481: No range allowed");
            return;
        }
        if (ids.length === 0) {
            wrong(`E486: Pattern not found: ${asked.global.pattern}`);
            return;
        }
        does(inner, ids, "normal");
    }

    function does(asked, ids, from) {
        switch (asked.name) {
        case "":
            if (ids.length > 0)
                cursorId = ids[ids.length - 1];
            break;
        case "delete":
            finish(ids, "done");
            break;
        case "forget":
            finish(ids, "forget");
            break;
        case "yank":
            copy(ids);
            break;
        case "reply":
            reply(asked.args);
            break;
        case "open":
            open();
            break;
        case "edit":
            if (asked.args === "")
                wrong("E32: No file name");
            else if (asked.args.toLowerCase() === "notices")
                stack.history();
            else
                wrong(`E447: Can't find file "${asked.args}" in path`);
            break;
        case "quit":
        case "xit":
        case "close":
            if (from === "prompt") {
                replyTo = null;
                mode = "normal";
            } else {
                stack.holding = false;
            }
            break;
        case "help":
            listing = true;
            listTop = NoticeKeys.helpAt(keyList, asked.args);
            break;
        case "undo":
            undo(0);
            break;
        case "redo":
            redo(0);
            break;
        case "set":
            setting(asked.args);
            break;
        default: {
            const offer = offered.find(one => one.command === asked.name);
            const one = current;
            const action = one && offer
                ? one.actions.find(act => act.identifier === offer.identifier) : null;
            if (action) {
                Notices.mark(one.id, true);
                action.invoke();
                stack.holding = false;
            }
        }
        }
    }

    // :set's one option here, Vim's relativenumber.
    function setting(option) {
        const names = { rnu: true, relativenumber: true, nornu: false, norelativenumber: false };
        if (option === "" || option === "rnu?" || option === "relativenumber?") {
            said = relative ? "  relativenumber" : "norelativenumber";
        } else if (option in names) {
            relative = names[option];
        } else if (["rnu!", "relativenumber!", "invrnu", "invrelativenumber"].includes(option)) {
            relative = !relative;
        } else {
            wrong(`E518: Unknown option: ${option}`);
        }
    }

    // A key as the table names it (NoticeKeys.js): a modifier, then the
    // letter it typed, or else its name, Shift unnamed beside a letter.
    readonly property var keyNames: ({
        [Qt.Key_Return]: "Return",
        [Qt.Key_Enter]: "Enter",
        [Qt.Key_Delete]: "Delete",
        [Qt.Key_Backspace]: "Backspace",
        [Qt.Key_Escape]: "Escape",
        [Qt.Key_Space]: "Space",
        [Qt.Key_Tab]: "Tab",
        [Qt.Key_Backtab]: "Tab",
        [Qt.Key_Home]: "Home",
        [Qt.Key_End]: "End",
        [Qt.Key_F1]: "F1",
        [Qt.Key_Left]: "Left",
        [Qt.Key_Up]: "Up",
        [Qt.Key_Right]: "Right",
        [Qt.Key_Down]: "Down"
    })
    function named(event) {
        const control = (event.modifiers & Qt.ControlModifier) !== 0;
        const shift = (event.modifiers & Qt.ShiftModifier) !== 0;
        // With Ctrl a letter types a control character, so it goes by name.
        const letter = !control && /^[!-~]$/.test(event.text) ? event.text : "";
        const byKey = control && event.key >= Qt.Key_A && event.key <= Qt.Key_Z
            ? String.fromCharCode(event.key) : "";
        const name = letter || (keyNames[event.key] ?? byKey);
        if (name === "")
            return "";
        return (control ? "Ctrl+" : "") + (shift && !letter ? "Shift+" : "") + name;
    }

    // Every key while the notices are held, in the mode it's pressed in.
    function pressed(event) {
        event.accepted = true;
        const key = named(event);
        if (key === "")
            return;
        said = "";
        saidWrong = false;
        const repeated = event.isAutoRepeat;
        switch (mode) {
        case "find":
            finds(key, repeated);
            break;
        case "command":
            commanded(key, repeated);
            break;
        case "insert":
            inserted(key, repeated);
            break;
        case "prompt":
            prompted(key);
            break;
        default:
            normal(key, repeated);
        }
    }

    // A key on the find line, which narrows the lines as it's typed; Return
    // goes to the chosen card's sender, and Escape gives normal on the
    // lines as found.
    function finds(key, repeated) {
        const typed = NoticeKeys.typing(findLine, key, "find", repeated);
        findLine = typed.line;
        switch (typed.act) {
        case "open":
            found = findLine.text;
            mode = "normal";
            open();
            break;
        case "choose":
        case "chooseBack":
            cursorId = listed[Math.max(0, Math.min(cursor + (typed.act === "choose" ? 1 : -1),
                listed.length - 1))] ?? cursorId;
            break;
        case "take":
            if (current)
                findLine = NoticeKeys.line(current.summary);
            break;
        case "paste":
            findLine = pasted(findLine);
            break;
        case "command":
            opensCommand("find", "");
            break;
        case "normal":
        case "leave":
            found = findLine.text;
            mode = "normal";
            break;
        }
    }

    function pasted(line) {
        const words = (Quickshell.clipboardText ?? "").split("\n")[0];
        return NoticeKeys.line(line.text.slice(0, line.caret) + words
            + line.text.slice(line.caret), line.caret + words.length);
    }

    function opensCommand(from, text) {
        commandFrom = from;
        commandLine = NoticeKeys.line(text);
        menuChosen = -1;
        menuAway = false;
        mode = "command";
        Report.say(`notice menu ${menuFound.shown.length}`);
    }

    // A key on the command line: typing narrows the menu, Tab and the
    // arrows choose a line and write it in, Return runs what's typed, and
    // Escape drops the line, putting back what it covered.
    function commanded(key, repeated) {
        const typed = NoticeKeys.typing(commandLine, key, "command", repeated);
        const before = commandLine.text;
        commandLine = typed.line;
        if (commandLine.text !== before)
            menuChosen = -1;
        const back = commandFrom === "find" ? "find" : commandFrom === "prompt" ? "prompt"
            : "normal";
        switch (typed.act) {
        case "run":
            run(commandLine.text);
            break;
        case "choose":
        case "chooseBack": {
            const shown = menuFound.shown;
            if (shown.length === 0)
                break;
            const by = typed.act === "choose" ? 1 : -1;
            if (menuChosen < 0)
                menuTyped = commandLine.text;
            menuChosen = menuChosen < 0 ? (by > 0 ? 0 : shown.length - 1)
                : (menuChosen + by + shown.length) % shown.length;
            const prefix = menuFound.prefix;
            const chosenEntry = shown[menuChosen];
            commandLine = NoticeKeys.line(prefix + chosenEntry.writes);
            break;
        }
        case "take":
            menuAway = true;
            break;
        case "away":
            menuAway = true;
            menuChosen = -1;
            break;
        case "paste":
            commandLine = pasted(commandLine);
            break;
        case "drop":
        case "leave":
            commandLine = NoticeKeys.line("");
            mode = back;
            break;
        }
    }

    // A key in a reply's prompt, typing its words; Return sends them and
    // Escape gives normal over them, the caret one left.
    function inserted(key, repeated) {
        const typed = NoticeKeys.typing(promptLine, key, "insert", repeated);
        promptLine = typed.line;
        const one = notice(replyTo);
        switch (typed.act) {
        case "send":
            if (one)
                send(one, promptLine.text);
            break;
        case "normal":
            promptBefore = promptLine.text;
            promptLine = NoticeKeys.line(promptLine.text, Math.max(promptLine.caret - 1, 0));
            mode = "prompt";
            break;
        case "paste":
            promptLine = pasted(promptLine);
            break;
        }
    }

    // A key in normal over a reply's words: Vim's editing, : for :q to drop
    // it, and Return to send it.
    function prompted(key) {
        const typed = NoticeKeys.prompting(standing, promptLine, key);
        standing = typed.standing;
        if (typed.line.text !== promptLine.text)
            promptBefore = promptLine.text;
        promptLine = typed.line;
        const one = notice(replyTo);
        switch (typed.act) {
        case "insert":
            mode = "insert";
            break;
        case "send":
            if (one)
                send(one, promptLine.text);
            break;
        case "command":
            opensCommand("prompt", "");
            break;
        case "undo":
            promptLine = NoticeKeys.line(promptBefore, Math.min(promptLine.caret,
                Math.max(promptBefore.length - 1, 0)));
            break;
        case "notHere":
            said = "not here";
            break;
        }
    }

    // A key in normal or visual, from the one table.
    function normal(key, repeated) {
        const read = NoticeKeys.read(standing, key, mode, { repeated, now: Date.now() });
        standing = read.standing;
        // Escape puts the key list away, as Vim's cancels which-key; at
        // rest it does nothing, so the table reads it as nothing.
        if (listing && key === "Escape") {
            listing = false;
            return;
        }
        if (read.act === "" || read.act === "pending") {
            if (read.act === "pending")
                Report.say(`notice pending ${standing.operator}`);
            return;
        }
        if (listing) {
            scrollsKeys(read);
            return;
        }
        act(read);
    }

    // Keys while the key list shows: the moves scroll it, and Escape, q
    // and Space ? put it away.
    function scrollsKeys(read) {
        const last = listLast;
        const by = Math.max(read.count, 1);
        const half = Math.floor(listRoom / 2);
        const scrolls = {
            down: listTop + by, up: listTop - by, first: 0, last,
            halfDown: listTop + half * by, halfUp: listTop - half * by,
            viewDown: listTop + listRoom * by, viewUp: listTop - listRoom * by
        };
        if (read.act in scrolls)
            listTop = Math.max(0, Math.min(scrolls[read.act], last));
        else if (["back", "keys", "quiet", "close"].includes(read.act))
            listing = false;
        else
            said = "not here";
    }

    // The view scrolled as Vim's is, the cursor kept in it (NoticeList's
    // scrolled): Ctrl+E Ctrl+Y, zz zt zb and the wheel.
    readonly property var scrolls: ["lineDown", "lineUp", "viewCentre", "viewAtTop",
        "viewAtBottom"]
    function scroll(motion, count) {
        const to = NoticeList.scrolled(motion, cursor, count,
            { count: listed.length, top: firstShown, shown: room });
        firstShown = to.top;
        cursorId = listed[to.cursor] ?? cursorId;
    }
    onFirstShownChanged: if (stack.holding) Report.say(`notice view from ${firstShown + 1}`)

    function act(read) {
        const count = read.count;
        const motions = ["down", "up", "first", "last", "viewTop", "viewMiddle", "viewBottom",
            "runOn", "runBack", "halfDown", "halfUp", "viewDown", "viewUp"];
        if (motions.includes(read.act)) {
            const to = NoticeList.moved(read.act, cursor, count,
                { count: listed.length, top: firstShown, senders: listed.map(id => senderOf(id)) });
            cursorId = listed[to] ?? cursorId;
            return;
        }
        if (scrolls.includes(read.act)) {
            scroll(read.act, count);
            return;
        }
        switch (read.act) {
        case "operate":
            operate(read.operator, read.motion, count);
            break;
        case "inRun": {
            const run = NoticeList.run(listed.map(id => senderOf(id)), cursor);
            anchorId = listed[run.from];
            cursorId = listed[run.to];
            break;
        }
        case "open":
            open();
            break;
        case "repeat":
            if (lastChange)
                operate(lastChange.operator, lastChange.motion,
                    count > 0 ? count : lastChange.count);
            break;
        case "undo":
            undo(count);
            break;
        case "redo":
            redo(count);
            break;
        case "select":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
            } else if (current) {
                anchorId = current.id;
                cursorId = current.id;
                mode = "visual";
            }
            break;
        case "otherEnd": {
            const other = anchorId;
            anchorId = cursorId ?? listed[cursor];
            cursorId = other;
            break;
        }
        case "reselect":
            if (lastSelection && selectedOf(lastSelection).length > 0) {
                anchorId = lastSelection.anchorId;
                cursorId = lastSelection.cursorId;
                mode = "visual";
            }
            break;
        case "find":
            findLine = NoticeKeys.line("");
            found = "";
            mode = "find";
            break;
        case "findStart":
        case "findEnd":
            findLine = NoticeKeys.line(found, read.act === "findStart" ? 0 : found.length);
            mode = "find";
            break;
        case "command":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
                opensCommand("visual", Ex.selected);
            } else {
                opensCommand("normal", count > 0 ? Ex.counted(count) : "");
            }
            break;
        case "close":
            stack.holding = false;
            break;
        case "quiet":
            said = "q does nothing \u00b7 :q closes";
            break;
        case "keys":
            listing = true;
            break;
        case "numbers":
            relative = !relative;
            Report.say(`notice numbers ${relative ? "relative" : "absolute"}`);
            break;
        // Escape ends visual, and never lets the notices go (Joel,
        // 2026-11-05: "fuck escape quitting").
        case "back":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
            }
            break;
        default:
            said = "not here";
        }
    }

    // The mode, at the foot's start in three capitals (NoticeKeys.mark).
    readonly property string modeMark: NoticeKeys.mark(mode)
    onModeMarkChanged: Report.say(`notice shows ${held.modeMark}`)

    // What the mode is doing, in the room kept for its widest beside the
    // facts: how many visual has chosen, a count being typed, or an
    // operator waiting with what it will take. The mode itself is named
    // at the foot's start, so this never says it again.
    readonly property string modeWord: {
        if (mode === "visual")
            return `${selected.length} chosen`;
        if (pending) {
            const word = standing.operatorAct === "done" ? doneWord([listed[cursor]])
                : "copy";
            return `${standing.operator} \u00b7 ${word} ${current?.summary ?? ""}`;
        }
        return standing.count;
    }

    // How things stand: which line of how many are listed. What a find
    // found is counted beside it in the head, as Vim's search count sits
    // beside the search, so the foot keeps only the list's own facts
    // (VKCF3Z, 2026-11-05).
    readonly property string facts: actsOn !== "" && mode === "command" ? actsOn
        : `${listed.length === 0 ? 0 : cursor + 1} of ${listed.length}`
    readonly property string foundCount: `${listed.length} of ${rows.length}`

    // The hints for the cursor card in this mode, from the table.
    readonly property var hints: {
        // The command line names its own keys: Tab chooses until a line is
        // chosen or typed, and then Return runs it.
        if (mode === "command") {
            if (commandLine.text === "" && menuChosen < 0)
                return [{ act: "choose", key: "Tab", word: "choose" }];
            const asked = Ex.parse(commandLine.text, commandNames);
            const name = asked.global ? "global" : asked.name ?? "";
            return [{ act: "run", key: "\u21b5", word: name === "" ? "run" : name }];
        }
        const hintMode = listing ? "listing" : pending ? "pending" : mode;
        const ids = mode === "visual" ? selected : [listed[cursor]];
        const one = current;
        const opens = one !== null && (stack.opens(one) !== null
            || Notices.senderWindow(one.appName) !== null);
        return NoticeKeys.hints(hintMode, { open: opens }, { done: doneWord(ids) });
    }

    // A held notice is a notice row of the launcher's (VKCF3Z,
    // 2026-11-05), so it takes the launcher's row (Choice.qml): four beats
    // tall, its Name's capitals centred in it, its ground close round its
    // ink. Every line's ink stands the inset from the frame's inside edge.
    readonly property real rowHeight: 4 * Theme.space.beat
    readonly property real rowCaps: nameCaps.tightBoundingRect.height
    // Where a row's Name has its baseline, down from the row's top.
    readonly property real rowBase: (rowHeight + rowCaps) / 2
    // What a row leaves under its baseline, which the last of anything
    // leaves too, so the hairline under it always stands as far off.
    readonly property real rowLeaves: (rowHeight - rowCaps) / 2
    // Where rich text sets the Words' first baseline, down from its top. Qt
    // lays a rich line of fixed height with its baseline four fifths down it
    // (QTextDocumentLayout's lineAdjustment), while it reports the face's
    // ascent as the baseline, as it is for styled text; placed by that, a
    // washed card's Words dropped 3 device pixels at 2 under a find.
    readonly property real richBase: Theme.level.words.pitch * 4 / 5
    // The columns, the launcher's (VKCF3Z's family mocks, 2026-11-05): the
    // gutter's figures right-aligned in the room of three, the ground a
    // mark on and the mark's cell a mark inside it, and the Name a glyph's
    // column after the mark's start, so names align whatever a mark's
    // width.
    readonly property real groundAt: Theme.space.rung.inset + gutterFigures.advanceWidth
        + Theme.space.rung.mark
    readonly property real markAt: groundAt + Theme.space.rung.mark
    readonly property real nameAt: markAt + Theme.level.name.pitch + Theme.space.rung.mark
    // Space ?'s pitch, the drawer's key list's: Figure's, at Note.
    readonly property real keyPitch: Theme.level.figure.pitch
    // How many of its lines show: as many as stand within the rows' room,
    // the first on the first row's baseline and the last a row's leaves
    // over where the last row's box would end.
    readonly property real listSpare: (room - 1) * rowHeight
    readonly property var listSteps: ({ pitch: keyPitch, beat: Theme.space.beat })
    readonly property int listRoom: NoticeKeys.shownFrom(keyList, listTop, listSpare, listSteps)
    // The furthest it scrolls, its last line shown.
    readonly property int listLast: NoticeKeys.lastTop(keyList, listSpare, listSteps)

    // The faces, measured by TextMetrics where a binding reads what's
    // measured, since a FontMetrics call in a binding is asked once, before
    // the face has loaded; the calls below read the face first, so they're
    // asked again as it does.
    FontMetrics {
        id: note
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    TextMetrics {
        id: nameCaps
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
    }

    // The gutter's width, by the widest it holds.
    TextMetrics {
        id: gutterFigures
        font: note.font
        text: "999"
    }

    // How wide a run of Note's is, asked again as the face loads.
    function noteWide(text: string): real {
        void note.font;
        return note.advanceWidth(text);
    }

    // How wide a key list's chords are drawn: each key of a chord a tie
    // from the next, each chord apart from the next.
    function chordsWide(chords): real {
        return chords.reduce((sum, chord, at) => sum + (at > 0 ? Theme.space.rung.apart : 0)
            + chord.split(" ").reduce((wide, key, part) => wide
                + (part > 0 ? Theme.space.rung.tie : 0) + held.noteWide(key), 0), 0);
    }
    // The keys' column, as wide as the widest line's chords.
    readonly property real keysWide: keyList.reduce((most, one) =>
        Math.max(most, chordsWide(one.chords)), 0)

    FontMetrics {
        id: figure
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
    }

    TextMetrics {
        id: titleCaps
        text: "H"
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize
        font.weight: Theme.level.title.weight
    }

    // The head's one baseline, as a place's in the launcher (VKCF3Z's
    // family cut, 2026-11-05): the place's Title its capitals the inset
    // under the frame, and the hairline under it set so the first row's
    // baseline stands a row under it.
    readonly property real headBase: Theme.space.rung.inset + titleCaps.tightBoundingRect.height
    readonly property real headTall: Math.round(headBase + rowHeight - Theme.border - rowBase)

    // The caret's blink, as the launcher's and his kitty's: a key shows it
    // whole, it fades out and back in, and with no key for a while it holds
    // steady, so a line left still draws nothing more (Prompt.qml).
    readonly property bool typing: mode === "find" || mode === "command" || mode === "insert"
    readonly property string lineText: mode === "find" ? findLine.text
        : mode === "command" ? commandLine.text : mode === "insert" ? promptLine.text : ""
    property real caretLight: 1
    readonly property bool blinks: typing && visible && Options.caret.fade > 0
    onLineTextChanged: wakes()
    onBlinksChanged: wakes()
    function wakes() {
        blinking.stop();
        caretLight = 1;
        if (blinks) {
            blinking.start();
            stilled.restart();
        }
    }
    SequentialAnimation {
        id: blinking
        loops: Animation.Infinite
        NumberAnimation {
            target: held
            property: "caretLight"
            from: 1
            to: 0
            duration: Options.caret.fade
        }
        NumberAnimation {
            target: held
            property: "caretLight"
            from: 0
            to: 1
            duration: Options.caret.fade
        }
    }
    Timer {
        id: stilled
        interval: Options.caret.stopsAfter
        onTriggered: {
            blinking.stop();
            held.caretLight = 1;
        }
    }

    // A match's ink and ground, as Joel's Neovim draws a search's.
    // On a chosen line, which lies on the selection, its ground is the
    // theme's for a selected row, so the match still shows.
    function wash(chosen) {
        return {
            ink: `${Theme.wash.ink}`,
            ground: `${chosen ? Theme.wash.selected : Theme.wash.ground}`
        };
    }

    // The frame's inside edge to edge, its head's capitals the inset under
    // its top, and the foot's baseline the inset over its bottom.
    Column {
        id: sheet
        width: parent.width
        // The foot's room under its row, so its baseline, where the row's
        // box puts it, stands the inset over the frame (Launcher.qml's).
        bottomPadding: Math.round(Theme.space.rung.inset
            - (held.rowHeight - (held.rowHeight - note.height) / 2 - note.ascent))

        // The wheel scrolls three lines a notch, as his Neovim's mousescroll
        // does, the cursor kept in view; over the key list, the list.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => {
                const notches = Math.round(-event.angleDelta.y / 120);
                if (notches === 0)
                    return;
                if (held.listing) {
                    held.listTop = Math.max(0, Math.min(held.listTop + 3 * notches,
                        held.listLast));
                    return;
                }
                held.scroll(notches > 0 ? "lineDown" : "lineUp", 3 * Math.abs(notches));
            }
        }

        // The head: where Joel is, the notices' glyph and their Title, as a
        // place in the launcher reads, led by the find's /, which types
        // here, since a find narrows the list beneath. The / stands at the
        // inset over the gutter, as Vim's lines start at column 0, muted at
        // rest and the accent while it finds; the glyph and the Title follow
        // a mark apart, as one place, and what's typed follows the Title at
        // Figure, on its baseline (VKCF3Z's family mocks, 2026-11-05).
        Item {
            id: head

            width: sheet.width
            height: held.headTall

            // How far in the caret stands.
            TextMetrics {
                id: soughtBefore
                font: figure.font
                text: held.findLine.text.slice(0, held.findLine.caret)
            }

            Text {
                id: slash
                x: Theme.space.rung.inset
                y: held.headBase - baselineOffset
                text: "/"
                color: held.mode === "find" ? Theme.accent : Theme.muted
                font: figure.font
            }

            Text {
                id: placeMark
                x: slash.x + slash.implicitWidth + Theme.space.rung.mark
                y: held.headBase - baselineOffset
                text: Theme.glyph.notifications
                color: Theme.accent
                font: note.font
            }

            Text {
                id: place
                x: placeMark.x + placeMark.implicitWidth + Theme.space.rung.mark
                y: held.headBase - baselineOffset
                text: "Notices"
                color: Theme.text
                font.family: Theme.level.title.family
                font.pointSize: Theme.level.title.pointSize
                font.weight: Theme.level.title.weight
            }

            // What's sought, typed or kept once its line is left, and how
            // many it found of how many, a mark each side of the dot, as
            // the hints are joined; the count stays whole, the sought words
            // giving way before it.
            Text {
                id: sought
                visible: held.pattern !== ""
                x: place.x + place.implicitWidth + Theme.space.rung.items
                width: Math.max(0, Math.min(implicitWidth, parent.width - x
                    - Theme.space.rung.inset - 2 * Theme.space.rung.mark - dot.implicitWidth
                    - count.implicitWidth))
                y: held.headBase - baselineOffset
                text: held.pattern
                textFormat: Text.PlainText
                elide: Text.ElideLeft
                // Strong while it types, as every line being typed is; the
                // text colour once kept, a fact like the count.
                color: held.mode === "find" ? Theme.strong : Theme.text
                font: figure.font
            }

            Text {
                id: dot
                visible: sought.visible
                x: sought.x + sought.width + Theme.space.rung.mark
                y: held.headBase - baselineOffset
                text: "\u00b7"
                color: Theme.muted
                font: note.font
            }

            Text {
                id: count
                visible: sought.visible
                x: dot.x + dot.implicitWidth + Theme.space.rung.mark
                y: held.headBase - baselineOffset
                text: held.foundCount
                color: Theme.muted
                font: note.font
            }

            Rectangle {
                visible: held.mode === "find"
                opacity: held.caretLight
                x: sought.x + soughtBefore.advanceWidth
                y: held.headBase - figure.ascent
                width: Math.max(1, Math.round(figure.averageCharacterWidth / 4))
                height: figure.ascent + figure.descent
                color: Theme.strong
            }
        }

        // The head's hairline, inset to inset, spanning the ink: a rule
        // meeting the frame would cut the card into boxes (VKCF3Z's family
        // cut, 2026-11-05).
        Rectangle {
            x: Theme.space.rung.inset
            width: sheet.width - 2 * Theme.space.rung.inset
            height: Theme.border
            color: Theme.edge
        }

        // The lines in view, or Space ?'s list in their place.
        Repeater {
            model: held.listing ? [] : held.inView

            Column {
                id: line

                required property var modelData
                required property int index
                readonly property int at: held.shownTop + index
                readonly property var one: held.notice(modelData)
                readonly property bool atCursor: at === held.cursor
                readonly property bool chosenLine: held.selected.includes(modelData)
                    || atCursor && (held.mode === "normal" || held.pending)
                // What the command line would act on, which keeps a ground
                // while the menu's line takes the selection's.
                readonly property bool targeted: held.mode === "command"
                    && held.targets.includes(modelData)
                readonly property var from: one ? Notices.sender(one.appName) : null
                readonly property bool washed: held.pattern !== ""

                width: sheet.width

                Item {
                    width: parent.width
                    height: held.rowHeight

                    // Chosen in visual, or waiting on an operator, each row
                    // wears the selection's ground, as the launcher's tinted
                    // rows do; a command line's targets lie on raised. It's
                    // the launcher's ground, close round the row's ink: a
                    // mark round the glyph and the time, above the capitals
                    // and below the baseline, so more room stands outside
                    // it than in; the gutter stands outside it, as it
                    // numbers the row rather than being part of it.
                    Rectangle {
                        x: held.groundAt
                        y: held.rowLeaves - Theme.space.rung.mark
                        width: parent.width - Theme.space.rung.inset + Theme.space.rung.mark - x
                        height: held.rowCaps + 2 * Theme.space.rung.mark
                        radius: Theme.inner
                        visible: line.chosenLine || line.targeted
                        color: Qt.alpha(line.targeted ? Theme.raised : Theme.selection,
                            Theme.floating.selection)
                    }

                    // The gutter: the cursor line's own number in text, the
                    // rest how far from it, muted.
                    Text {
                        id: gutter
                        x: Theme.space.rung.inset
                        width: gutterFigures.advanceWidth
                        anchors.baseline: who.baseline
                        horizontalAlignment: Text.AlignRight
                        text: NoticeList.number(line.at, held.cursor, held.relative)
                        color: line.atCursor ? Theme.text : Theme.muted
                        font: note.font
                    }

                    Text {
                        id: mark
                        x: held.markAt
                        anchors.baseline: who.baseline
                        text: line.from ? String.fromCodePoint(line.from.glyph) : ""
                        color: line.from ? Theme[line.from.hue] : Theme.muted
                        font: note.font
                    }

                    // Its Name, its capitals centred in the row. Washed by
                    // the find, it's rich text, set on the same baseline.
                    Text {
                        id: who
                        x: held.nameAt
                        y: held.rowBase - baselineOffset
                        width: Math.min(implicitWidth, (time.x - x - Theme.space.rung.groups)
                            * (first.text === "" ? 1 : 0.6))
                        text: !line.one ? "" : line.washed
                            ? Search.washed(line.one.summary, held.pattern,
                                held.wash(line.chosenLine))
                            : line.one.summary
                        textFormat: line.washed ? Text.RichText : Text.PlainText
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        color: Theme.strong
                        font.family: Theme.level.name.family
                        font.pointSize: Theme.level.name.pointSize
                        font.weight: Theme.level.name.weight
                    }

                    // Its first words, unless it's the cursor card, whose
                    // words show whole beneath.
                    Text {
                        id: first
                        x: who.x + who.width + Theme.space.rung.items
                        // Giving way a group before the time.
                        width: Math.max(time.x - x - Theme.space.rung.groups, 0)
                        anchors.baseline: who.baseline
                        visible: text !== ""
                        text: !line.one || line.atCursor ? ""
                            : line.washed ? Search.washed(held.stack.plain(line.one.body)
                                .split("\n")[0], held.pattern, held.wash(line.chosenLine))
                            : held.stack.plain(line.one.body).split("\n")[0]
                        textFormat: line.washed ? Text.RichText : Text.PlainText
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        color: Theme.level.words.colour
                        font.family: Theme.level.words.family
                        font.pointSize: Theme.level.words.pointSize
                        font.weight: Theme.level.words.weight
                        font.variableAxes: Theme.level.words.variableAxes
                    }

                    Text {
                        id: time
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.space.rung.inset
                        anchors.baseline: who.baseline
                        readonly property real now: clock.date.getTime()
                        text: line.one ? held.stack.since(held.stack.arrived[line.one.id] ?? now,
                            now) : ""
                        color: Theme.level.note.colour
                        font: note.font
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            if (line.atCursor)
                                held.open();
                            else
                                held.cursorId = line.modelData;
                        }
                    }
                }

                // The cursor card whole under its line, on the Name's
                // column, each line set by its baseline, down in beats
                // (VKCF3Z, 2026-11-05): what it says a pitch under the
                // Name, as lines of one saying; who sent it Note's pitch and
                // a beat on, as the toast signs it; its acts as commands a
                // pitch under that, and a reply's field two beats under
                // them.
                Item {
                    id: card

                    // Its row's Name's baseline, down from its top, which
                    // is the row's bottom.
                    readonly property real nameBase: held.rowBase - held.rowHeight
                    readonly property real wordsBase: nameBase + Theme.level.words.pitch
                    readonly property real wordsEnd: body.visible
                        ? wordsBase + (body.lineCount - 1) * Theme.level.words.pitch
                        : nameBase
                    readonly property real senderBase: wordsEnd + Theme.level.note.pitch
                        + Theme.space.beat
                    readonly property real lastBase: says.visible
                        ? senderBase + Theme.level.note.pitch : senderBase

                    // Folded while the command line's menu is up, which
                    // takes its room.
                    visible: line.atCursor && line.one !== null && held.mode !== "command"
                    x: held.nameAt
                    width: parent.width - x - Theme.space.rung.inset
                    // A field ends it as a row does. Else it leaves what a
                    // row leaves under its last baseline, so the hairline
                    // under it stands as far off, less a beat where a row
                    // follows: the next Name a row less a beat under the
                    // card's last line, as VKCF3Z's family mocks set it.
                    height: tall - (line.index < held.inView.length - 1 ? Theme.space.beat : 0)
                    readonly property real tall: field.visible ? field.y + field.height
                        : lastBase + held.rowLeaves
                    // Told to the list, which gives it room on a short
                    // screen; set rather than bound, as the rows shown
                    // follow from it.
                    function told() {
                        if (card.visible)
                            Qt.callLater(held.cardSays, line.modelData, card.tall);
                    }
                    onTallChanged: told()
                    onVisibleChanged: told()
                    Component.onCompleted: told()

                    Text {
                        id: body
                        y: card.wordsBase - (line.washed ? held.richBase : baselineOffset)
                        width: parent.width
                        visible: text !== ""
                        text: !line.one || !line.atCursor ? "" : line.washed
                            ? Search.washed(held.stack.plain(line.one.body), held.pattern,
                                held.wash(line.chosenLine && held.mode === "visual"))
                            : line.one.body
                        textFormat: line.washed ? Text.RichText : Text.StyledText
                        wrapMode: Text.Wrap
                        maximumLineCount: 6
                        elide: Text.ElideRight
                        lineHeight: Theme.level.words.pitch
                        lineHeightMode: Text.FixedHeight
                        color: Theme.text
                        font.family: Theme.level.words.family
                        font.pointSize: Theme.level.words.pointSize
                        font.weight: Theme.level.words.weight
                        font.variableAxes: Theme.level.words.variableAxes
                    }

                    Text {
                        y: card.senderBase - baselineOffset
                        width: parent.width
                        text: line.from ? line.from.name : ""
                        elide: Text.ElideRight
                        color: Theme.level.note.colour
                        font: note.font
                    }

                    // Its own acts as the commands that do them, ":reply" and
                    // ":play", in the text's colour since they're typed; Return
                    // goes to its sender, as the foot says.
                    Text {
                        id: says
                        y: card.senderBase + Theme.level.note.pitch - baselineOffset
                        width: parent.width
                        visible: text !== ""
                        text: held.cardSays
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: Theme.text
                        font: note.font
                    }

                    // A reply's words, typed on the card it answers, in a
                    // prompt's field: a row, its words on the card's column
                    // and ending at its inset, as every line of the card,
                    // and its ground the inset out from them on each side,
                    // so it meets the frame's inside edge at the right
                    // (VKCF3Z, 2026-11-05).
                    Item {
                        id: field
                        visible: line.atCursor && held.replyTo === line.modelData
                        x: -Theme.space.rung.inset
                        y: card.lastBase + 2 * Theme.space.beat
                        width: parent.width + 2 * Theme.space.rung.inset
                        height: held.rowHeight

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.inner
                            color: Qt.alpha(Theme.raised, Theme.floating.selection)
                        }

                        // How far in the caret stands.
                        TextMetrics {
                            id: replyBefore
                            font: figure.font
                            text: held.promptLine.text.slice(0, held.promptLine.caret)
                        }

                        // Over the caret, so the block in normal sits
                        // behind the letter it's on.
                        Text {
                            id: words
                            z: 1
                            x: Theme.space.rung.inset
                            width: parent.width - 2 * x
                            anchors.verticalCenter: parent.verticalCenter
                            text: held.promptLine.text === "" && held.mode === "insert"
                                ? line.one?.inlineReplyPlaceholder ?? "" : held.promptLine.text
                            textFormat: Text.PlainText
                            elide: Text.ElideLeft
                            color: held.promptLine.text === "" ? Theme.muted : Theme.strong
                            font: figure.font
                        }

                        // The bar in insert, the block over a letter in
                        // normal, as Joel's guicursor.
                        Rectangle {
                            readonly property real before: replyBefore.advanceWidth
                            visible: held.mode === "insert" || held.mode === "prompt"
                            opacity: held.mode === "insert" ? held.caretLight : 1
                            x: words.x + before
                            y: words.y + words.baselineOffset - figure.ascent
                            width: held.mode === "insert"
                                ? Math.max(1, Math.round(figure.averageCharacterWidth / 4))
                                : figure.averageCharacterWidth
                            height: figure.ascent + figure.descent
                            color: held.mode === "insert" ? Theme.strong : Theme.selection
                        }
                    }
                }
            }
        }

        // Nothing left: said, so the list never looks broken.
        Text {
            visible: !held.listing && held.listed.length === 0
            x: Theme.space.rung.inset
            height: held.rowHeight
            verticalAlignment: Text.AlignVCenter
            text: held.pattern !== "" ? `nothing holds ${held.pattern}` : "no notices"
            color: Theme.muted
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
            font.variableAxes: Theme.level.words.variableAxes
        }

        // What arrived while the lines held their order.
        Text {
            visible: held.arrivals > 0
            x: Theme.space.rung.inset
            height: held.rowHeight
            verticalAlignment: Text.AlignVCenter
            text: `${held.arrivals} new`
            color: Theme.accent
            font: note.font
        }

        // Space ?: every key, by mode, each group under its heading, as the
        // drawer's key list sets them (VKCF3Z, 2026-11-05): a line a key
        // pitch, a heading a pitch and a beat over its first line and an
        // empty line under the one before; each chord apart from the next
        // and its keys a tie, and what they do a group past the keys.
        Item {
            id: keyLines

            // Each line's baseline, down from the list's top: the first's
            // on the first row's, a row under the head's, whatever its
            // level, so the key list keeps the rows' baselines.
            readonly property var bases: {
                const out = [];
                held.listShown.forEach((one, at) => {
                    out.push(at === 0 ? held.rowBase : out[at - 1]
                        + NoticeKeys.stepTo(held.listShown, at, held.listSteps));
                });
                return out;
            }

            visible: held.listing
            width: sheet.width
            height: bases.length === 0 ? 0 : bases[bases.length - 1] + held.rowLeaves

            Repeater {
                model: held.listing ? held.listShown : []

                Item {
                    id: listedKey

                    required property var modelData
                    required property int index
                    readonly property real base: keyLines.bases[index] ?? 0

                    width: sheet.width

                    Text {
                        x: Theme.space.rung.inset
                        y: listedKey.base - baselineOffset
                        visible: listedKey.modelData.heading !== ""
                        text: listedKey.modelData.heading
                        color: Theme.level.label.colour
                        font.family: Theme.level.label.family
                        font.pointSize: Theme.level.label.pointSize
                        font.weight: Theme.level.label.weight
                        font.capitalization: Theme.level.label.capitalization
                        font.letterSpacing: Theme.level.label.letterSpacing
                        font.features: Theme.level.label.features
                    }

                    Row {
                        x: Theme.space.rung.inset
                        y: listedKey.base - baselineOffset
                        spacing: Theme.space.rung.apart
                        visible: listedKey.modelData.heading === ""
                        baselineOffset: note.ascent

                        Repeater {
                            model: listedKey.modelData.chords

                            Row {
                                id: chord
                                required property string modelData
                                spacing: Theme.space.rung.tie

                                Repeater {
                                    model: chord.modelData.split(" ")

                                    Text {
                                        required property string modelData
                                        text: modelData
                                        textFormat: Text.PlainText
                                        color: Theme.text
                                        font: note.font
                                    }
                                }
                            }
                        }
                    }

                    Text {
                        x: Theme.space.rung.inset + held.keysWide + Theme.space.rung.groups
                        width: parent.width - x - Theme.space.rung.inset
                        y: listedKey.base - baselineOffset
                        visible: listedKey.modelData.heading === ""
                        text: listedKey.modelData.does
                        elide: Text.ElideRight
                        color: Theme.muted
                        font: note.font
                    }
                }
            }
        }

        // The menu, rising above the command line: the launcher's own
        // (CommandMenu.qml), one menu for every : line, as many lines as
        // the screen has room for.
        CommandMenu {
            id: commandMenu
            width: sheet.width
            all: held.entries
            lines: held.menuFound.shown
            chosen: held.menuChosen
            most: held.menuRoom
            visible: held.mode === "command" && !held.menuAway && showing > 0
        }

        // The command line, above the status line with its menu rising
        // from it, as Vim's: the accent's sigil, what's typed in the mono
        // at Figure, and the bar caret where it types.
        Item {
            visible: held.mode === "command"
            width: sheet.width
            height: held.rowHeight

            // How far in the caret stands.
            TextMetrics {
                id: lineBefore
                font: figure.font
                text: held.commandLine.text.slice(0, held.commandLine.caret)
            }

            Text {
                id: sigil
                x: Theme.space.rung.inset
                anchors.verticalCenter: parent.verticalCenter
                text: ":"
                color: Theme.accent
                font: figure.font
            }

            Text {
                id: typed
                x: sigil.x + sigil.width
                width: parent.width - x - Theme.space.rung.inset
                anchors.baseline: sigil.baseline
                text: held.commandLine.text
                textFormat: Text.PlainText
                elide: Text.ElideLeft
                color: Theme.strong
                font: figure.font
            }

            Rectangle {
                opacity: held.caretLight
                x: typed.x + lineBefore.advanceWidth
                y: typed.y + typed.baselineOffset - figure.ascent
                width: Math.max(1, Math.round(figure.averageCharacterWidth / 4))
                height: figure.ascent + figure.descent
                color: Theme.strong
            }
        }

        // The foot a region of its own, as the launcher's is: under one
        // hairline, inset to inset as the head's, which whatever is last
        // above meets with what a row leaves.
        Rectangle {
            x: Theme.space.rung.inset
            width: sheet.width - 2 * Theme.space.rung.inset
            height: Theme.border
            color: Theme.edge
        }

        // The foot: how things stand, the mode's word in its room, and the
        // hints, or what was just done until the next key.
        Item {
            id: foot

            width: sheet.width
            height: held.rowHeight

            // The mode word's room, kept for the widest it says often, so
            // the hints never shift as the mode changes; an operator's
            // word, naming its card, may take more where the hints leave
            // it. TextMetrics, since a binding follows a property and never
            // what a call measured before the face loaded.
            readonly property real wordRoom: widestWord.advanceWidth

            TextMetrics {
                id: widestWord
                font: note.font
                text: "999 chosen"
            }

            // The mode in three capitals, first, where the eye starts the
            // line, as his Neovim's status line starts with it (Joel,
            // 2026-11-05: "in a place where they are more obvious and not
            // lost with other things"). It's set as a label, the theme's
            // capitals, muted in normal and in the accent in every other
            // mode, as his status line names a mode but normal in it. Every
            // mark is three of the mono's capitals, so one's width is all
            // of theirs, and nothing beside it moves as the mode changes.
            Text {
                id: markText
                x: Theme.space.rung.inset
                anchors.verticalCenter: parent.verticalCenter
                text: held.modeMark
                color: held.modeMark === NoticeKeys.mark("normal") ? Theme.muted : Theme.accent
                font.family: Theme.level.label.family
                font.pointSize: Theme.level.label.pointSize
                font.weight: Theme.level.label.weight
                font.capitalization: Theme.level.label.capitalization
                font.letterSpacing: Theme.level.label.letterSpacing
                font.features: Theme.level.label.features
            }

            // Where the facts start: a group past the mark's ink, its last
            // capital's tracking, which a label draws after it, left off.
            readonly property real factsAt: markText.x + markText.implicitWidth
                - markText.font.letterSpacing + Theme.space.rung.groups

            // Whole, as the hints leave it its room as it says it now; it
            // gives way only where the hints that stay leave none.
            Text {
                id: facts
                x: foot.factsAt
                width: Math.min(implicitWidth, foot.width - foot.factsAt
                    - Theme.space.rung.inset - Theme.space.rung.groups - hinted.width)
                anchors.verticalCenter: parent.verticalCenter
                text: held.facts
                elide: Text.ElideRight
                color: Theme.muted
                font: note.font
            }

            Text {
                id: modeWord
                x: facts.x + facts.width + (facts.width > 0 ? Theme.space.rung.groups : 0)
                width: Math.max(foot.wordRoom, Math.min(implicitWidth, foot.width
                    - foot.factsAt - Theme.space.rung.inset - facts.width - hinted.width
                    - 2 * Theme.space.rung.groups))
                anchors.verticalCenter: parent.verticalCenter
                text: held.modeWord
                elide: Text.ElideRight
                color: held.standing.count !== "" && !held.pending ? Theme.strong : Theme.accent
                font: note.font
            }

            Text {
                id: hinted

                // The room the facts and the mode word leave as they say
                // it now, a group from each, so at rest, with no mode word,
                // the hints have it all, and none gives way while there's
                // room for it (VKCF3Z, 2026-11-05).
                readonly property real fits: foot.width - foot.factsAt
                    - Theme.space.rung.inset - factsNow.advanceWidth - Theme.space.rung.groups
                    - (held.modeWord === "" ? 0
                        : wordNow.advanceWidth + Theme.space.rung.groups)

                TextMetrics {
                    id: factsNow
                    font: note.font
                    text: held.facts
                }

                TextMetrics {
                    id: wordNow
                    font: note.font
                    text: held.modeWord
                }

                // The hints that fit: the middle ones give way first, then
                // the first, so Space ? and Return's at the right end stay,
                // and the last stays alone before a hint is cut mid-word.
                readonly property string all: {
                    if (held.said !== "")
                        return held.said;
                    // Read, so what's measured below follows the face.
                    const face = note.font;
                    if (face.family === "")
                        return "";
                    const said = held.hints.map(one => `${one.key} ${one.word}`);
                    let shown = said;
                    while (shown.length > 1 && note.advanceWidth(shown.join(" \u00b7 ")) > fits) {
                        const going = shown.length > 3 ? 1 : 0;
                        shown = shown.slice(0, going).concat(shown.slice(going + 1));
                    }
                    return shown.join(" \u00b7 ");
                }

                anchors.right: parent.right
                anchors.rightMargin: Theme.space.rung.inset
                anchors.verticalCenter: parent.verticalCenter
                width: Math.min(implicitWidth, fits)
                text: all
                elide: Text.ElideLeft
                color: held.saidWrong ? Theme.alert : held.said !== "" ? Theme.text : Theme.muted
                font: note.font
            }
        }
    }

    // How many menu lines show: ten at most, and no more than the height
    // the lines leave while the command line is up, so the line being
    // typed never falls off the screen; one at least.
    readonly property int menuRoom: Math.max(1, Math.min(10,
        Math.floor((commandTall - inView.length * rowHeight) / commandMenu.pitch) - 1))
    // How tall the lines and the menu may be while the command line is up:
    // the lines' height less the command line and the menu's hairline. A
    // menu line's pitch more is kept for the menu's room above and below
    // its lines.
    readonly property real commandTall: listTall - Theme.border - rowHeight
    // How tall the lines may be: the screen less its rung each side, the
    // frame's hairlines, the head and its rule, and the foot and its rule.
    readonly property real listTall: screenHeight - 2 * Theme.space.rung.screen
        - 4 * Theme.border - headTall - rowHeight - sheet.bottomPadding
    // The lines that leave the menu three where the screen has the room.
    readonly property int commandRows: Math.max(1, Math.min(room,
        Math.floor((commandTall - 4 * commandMenu.pitch) / rowHeight)))


    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }
}
