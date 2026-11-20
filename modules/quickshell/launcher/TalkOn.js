// Which orchestrator Ask's Talk to row talks a question over with
// (Ask.qml's talkOn), apart from the launcher so the answers' cases hold it
// in Qt's own engine (tests/answers/core.js).
.pragma library
.import "Order.js" as Order

// Of the orchestrators running, each `{ one, session }` in the order the
// Orchestrators place lists them, the first in the one order of how
// sessions stand, as the agents daemon ranks each (modules/agents/src/
// standing.rs) and `rankOf` reads it from its row (SessionRow.rankOf), so
// one waiting on Joel comes before one working; ties keep the order
// listed. One the daemon hasn't ranked yet, as when the shell is newer than
// the daemon, is ranked after every one it has, never ahead of one needing
// Joel. Null with none running.
function firstToTalk(running, rankOf) {
    const ranked = (running ?? []).map(each => Object.assign({ rank: rankOf(each.session) },
        each));
    return Order.sorted(ranked, Order.smaller(each => each.rank))[0] ?? null;
}
