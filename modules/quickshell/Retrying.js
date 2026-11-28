// What Daemon.qml's askTillTaken decides of an ask that may be refused for now: whether an
// answer takes it, sends it again or gives it up, which asks wait for the next try, and which
// of them a try asks. Apart from Daemon.qml, so each is held in Qt's own engine in a moment
// (tests/retrying.qml) rather than over a real socket's four-second tries.
.pragma library

// What `said` asks now: one given as a function says its latest each time it's asked.
function asked(said) {
    return typeof said === "function" ? said() : said;
}

// What becomes of an ask once `answer` comes, `tries` its tries left counting the one
// answered. One taken, or refused by any code but `refused`, is done. One refused for now,
// or given no answer as the daemon went with a switch's restart, goes again, unless that was
// its last try: then it's given up, `said` for the journal and `why` for Joel, the daemon's
// refusal or "" where none came.
function after(answer, tries) {
    const unanswered = answer === null || answer === undefined;
    if (!unanswered && answer?.error?.code !== "refused") {
        return { next: "done" };
    }
    if (tries <= 1) {
        return {
            next: "givenUp",
            said: unanswered ? "no answer" : answer.error.message,
            why: unanswered ? "" : String(answer.error.message ?? "")
        };
    }
    return { next: "again" };
}

// The asks waiting for their next try once `said`, asking `ask`, is refused with `tries`
// left. One given as a function says its latest when asked, so one try of each such ask
// waits, the latest, rather than ten kept places each paying the daemon's wait (round 000011
// on #6796, F111); one given as words names what it acts on, so each waits. The one kept
// takes the most tries left of those it stands for, so a fresh ask isn't given up on with an
// older one's few (round 000036 on #6796, F139).
function waiting(waited, said, ask, tries) {
    const alike = one => typeof said === "function" && typeof one.said === "function"
        && one.ask === ask;
    const left = waited.filter(alike).reduce((most, one) => Math.max(most, one.tries), tries - 1);
    return waited.filter(one => !alike(one)).concat([{ said, ask, tries: left }]);
}

// Which of the `waited` a try falling due asks now, and which it keeps: none while nothing
// answers (`wired` false), so a try spends nothing and the connection's return starts it again.
function due(waited, wired) {
    return wired ? { now: waited, kept: [] } : { now: [], kept: waited };
}
