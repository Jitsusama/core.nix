// Which orchestrator Ask's Talk to row talks a question over with
// (Ask.qml's talkOn), apart from the launcher so the answers' cases hold it
// in Qt's own engine (tests/answers/core.js).
.pragma library
.import "Order.js" as Order

// Of the orchestrators running, each `{ one, session }` in the order the
// Orchestrators place lists them, the first in the one order of how
// sessions stand, as the agents daemon ranks each (`rankOf`, from its row's
// rank, modules/agents/src/standing.rs), so one waiting on Joel comes before
// one working; ties keep the order listed. One the daemon hasn't ranked
// yet, as when the shell is newer than the daemon, goes after every one it
// has, never ahead of one needing Joel. Null with none running.
function firstToTalk(running, rankOf) {
    const ranked = (running ?? []).map(each => {
        // Only a number is a rank: Number(null) would be 0, the first place.
        const rank = rankOf(each.session);
        return Object.assign({ rank: Number.isFinite(rank) ? rank : Number.MAX_SAFE_INTEGER },
            each);
    });
    return Order.sorted(ranked, Order.smaller(each => each.rank))[0] ?? null;
}
