.pragma library
// How a Spend row reads now while Spend's rows aren't wanted, as Now's
// Spend and Tokens and Agents' Budgets ask it (Spend.qml's rowNow). Said
// apart from Spend.qml so tests/spend-now.qml holds it in Qt's own engine.

// The record the place serves a row of `key` as.
function recordOf(place, key) {
    return `row:${place}:${key.slice("spend:".length)}`;
}

// How a row of `key` reads now, from how Spend stands (`spend`): "unsure"
// while its rows are wanted, when they're as they read now and the row is
// looked for where it's shown, and for a key not Spend's. While
// spend-watch isn't answering, Spend lists only its down row, so Start
// reads as it is ("down") and every other as none. Then "served" for a
// record the place holds, and "none" for one it doesn't once the place has
// answered on this connection, even with no rows, or while the screen is
// shared, before the quiet place's first answer too, since it holds no
// Budgets. Said as unsure, a row is looked for among the rows last made
// while wanted, and Agents' Budgets said the amounts those held, on a
// shared screen too (core.nix#6073). So "unsure" is left only for the
// moment before the place first answers, the screen unshared.
function standing(key, spend) {
    if (spend.wanted || !key.startsWith("spend:")) {
        return "unsure";
    }
    if (spend.down) {
        return key === "spend:start" ? "down" : "none";
    }
    if (spend.serves(recordOf(spend.place, key))) {
        return "served";
    }
    return spend.sharing || spend.answered ? "none" : "unsure";
}
