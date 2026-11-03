.pragma library
// Where Reopen Them All puts each session back (Sessions.qml), from the
// places kept for it: tests/places.qml holds it to the cases below. A kept
// place names its screen by what the screen is, its maker, model and
// serial as niri describes it, since a dock or a boot can give the same
// screen another connector: on 2026-11-01 the same monitor came up as DP-5
// after a boot as DP-4, and every session kept on DP-4 opened on the
// laptop's own screen.

// A screen as niri describes it, as its config names one: "HP Inc. HP
// Z27k G3 CN41282GC1". niri's answer to Outputs gives each by connector.
function described(output) {
    return [output?.make, output?.model, output?.serial]
        .filter(part => typeof part === "string" && part !== "")
        .join(" ");
}

// Each connector's description, from niri's answer to Outputs.
function screensOf(outputs) {
    const screens = {};
    for (const [name, output] of Object.entries(outputs ?? {})) {
        screens[name] = described(output);
    }
    return screens;
}

// The connector a kept place's screen is on now, or undefined when it's
// gone: found by what it is, the one it was kept on when two screens are
// alike, as two monitors of one model without serials are, or by its
// connector when it was kept before screens were named so.
function connectorOf(at, screens) {
    if (at.screen) {
        const alike = Object.keys(screens).filter(name => screens[name] === at.screen);
        return alike.includes(at.output) ? at.output : alike[0];
    }
    return at.output in screens ? at.output : undefined;
}

// Where each kept place goes now, in the order given, null for none: on
// its own screen, wherever it's connected now. One whose screen is gone
// goes to the screen in use, its workspaces after every workspace the
// places there use, in their order, as niri moves a gone screen's
// workspaces, rather than mixed into the same numbers, and says where it
// belongs (`away`), so its workspace can go there once the screen comes.
// `screens` maps each connector there is to its description; with none
// known, as when niri didn't answer, or no screen in use, a gone one is
// left as it was kept.
function placed(places, screens, inUse) {
    const known = Object.keys(screens ?? {}).length > 0;
    const found = places.map(at => {
        if (!at) {
            return null;
        }
        const output = known ? connectorOf(at, screens) : at.output;
        return output === undefined ? { at, gone: at.screen || at.output } : { at, output };
    });
    let last = 0;
    for (const one of found) {
        if (one?.output !== undefined && one.output === inUse) {
            last = Math.max(last, one.at.workspace);
        }
    }
    const goneScreens = found
        .filter(one => one?.gone !== undefined)
        .map(one => one.gone)
        .filter((gone, at, all) => all.indexOf(gone) === at)
        .sort();
    const after = {};
    for (const gone of goneScreens) {
        after[gone] = last;
        for (const one of found) {
            if (one?.gone === gone) {
                last = Math.max(last, after[gone] + one.at.workspace);
            }
        }
    }
    return found.map(one => {
        if (one === null) {
            return null;
        }
        if (one.output !== undefined) {
            return Object.assign({}, one.at, { output: one.output });
        }
        if (inUse === null || inUse === undefined) {
            return Object.assign({}, one.at);
        }
        return Object.assign({}, one.at, {
            output: inUse,
            workspace: after[one.gone] + one.at.workspace,
            away: { screen: one.gone, workspace: one.at.workspace }
        });
    });
}

// The moves that take workspaces given in place of a gone screen to it once
// it's connected again. `away` holds, by each such screen's description (or
// its connector, for a place kept before screens were named so), the
// workspaces given in its place, by niri's id, each with the number it was
// kept at there. Each move carries the index it takes there (`index`, from
// 1), its rank by kept number among them, lowest first, to be set in that
// order once all are moved: niri puts a moved workspace after the screen's
// active one, and makes it the active one when it held the focus, so where
// each lands can't be foretold. One gone already isn't moved, and one on the
// screen already keeps its place. `home` names the screens connected again,
// which are done with.
function homeward(away, screens, workspaces) {
    const moves = [];
    const home = [];
    for (const screen of Object.keys(away ?? {}).sort()) {
        const output = Object.keys(screens ?? {})
            .find(name => screens[name] === screen || name === screen);
        if (output !== undefined) {
            home.push(screen);
            const ranked = away[screen]
                .filter(one => workspaces.some(each => each.id === one.id))
                .sort((a, b) => a.workspace - b.workspace);
            ranked.forEach((one, at) => {
                const now = workspaces.find(each => each.id === one.id);
                if (now.output !== output) {
                    moves.push({ output, id: one.id, index: at + 1 });
                }
            });
        }
    }
    return { moves, home };
}

// Which window kept, of a boot's windows other than its sessions, a window
// opening now is: the first not used yet of the same program with the same
// title, else the first of that program, since a page's title can change
// before its window shows. -1 for none. `used` lists the indices taken.
function matchOther(kept, used, app, title) {
    const free = (one, at) => !used.includes(at) && one.app === app;
    const same = kept.findIndex((one, at) => free(one, at) && one.title === title);
    return same >= 0 ? same : kept.findIndex(free);
}

// Where a window put back goes, beside the windows already back from the
// same boot, rather than by its workspace's number: a workspace that held no
// session isn't made again as sessions are reopened, so every number after
// it is one less. `beside` holds each window back: its id, its kept place,
// the workspace it's on now and its column there. It joins the workspace
// holding one kept on its screen at its number (`join`): just after the
// nearest kept left of it (`after`), else just before the nearest kept
// right of it (`before`), by window rather than by column number, since a
// window Joel opened since can sit among them. Else it's given a workspace
// of its own (`make`), just after the one holding those kept at the next
// number below on its screen (`output`, `index`), or first, on a screen
// unknown, when none is.
function whereBack(at, beside) {
    const screenOf = one => one.screen || one.output;
    const there = beside.filter(one => screenOf(one.at) === screenOf(at));
    const joined = there.find(one => one.at.workspace === at.workspace);
    if (joined !== undefined) {
        const into = joined.workspace;
        const kin = there
            .filter(one => one.workspace.id === into.id && one.at.workspace === at.workspace)
            .sort((a, b) => a.at.column - b.at.column || a.column - b.column);
        const left = kin.filter(one => one.at.column < at.column);
        if (left.length > 0) {
            return { join: into.id, after: left[left.length - 1].id };
        }
        return { join: into.id, before: kin[0].id };
    }
    const below = there
        .filter(one => one.at.workspace < at.workspace)
        .sort((a, b) => b.at.workspace - a.at.workspace)[0];
    if (below === undefined) {
        return { make: true, output: null, index: 1 };
    }
    return { make: true, output: below.workspace.output, index: below.workspace.idx + 1 };
}
