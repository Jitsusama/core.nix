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
// gone: found by what it is, or by its connector when it was kept before
// screens were named so.
function connectorOf(at, screens) {
    if (at.screen) {
        return Object.keys(screens).find(name => screens[name] === at.screen);
    }
    return at.output in screens ? at.output : undefined;
}

// Where each kept place goes now, in the order given, null for none: on
// its own screen, wherever it's connected now. One whose screen is gone
// goes to the screen in use, its workspaces after every workspace the
// places there use, in their order, as niri moves a gone screen's
// workspaces, rather than mixed into the same numbers. `screens` maps each
// connector there is to its description; with none known, as when niri
// didn't answer, or no screen in use, a gone one is left as it was kept.
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
            workspace: after[one.gone] + one.at.workspace
        });
    });
}
