// What Spotify's program in the launcher works out from what spotify-recent
// prints (SpotifyProgram.qml): its lines read into records, what each says
// beside it, the link it copies, and how Spotify stands by the command's
// exit status. Plain JavaScript, so tests/spotify-program.qml reaches it
// in the shell's own engine.
.pragma library
.import "Ago.js" as Ago

// spotify-recent's exit statuses (src/spotify.rs's Refused), by how
// Spotify stands for each.
const STANDS = { 0: "ok", 2: "signedOut", 3: "nowhere", 4: "unreachable" };

// The fields of a line: its URI, kind, name, who made it and when.
const FIELDS = 5;

// How Spotify stands after the command ended with `code`: "ok",
// "signedOut", "nowhere" for nothing to play on, "unreachable", or
// "failed" for anything else.
function standing(code) {
    return STANDS[code] ?? "failed";
}

// How Spotify stands after several reads ended with `codes`: the worst
// any said, signed out over out of reach over failed over well, since
// what mends the worst comes first.
function worst(codes) {
    const order = ["signedOut", "unreachable", "failed", "nowhere", "ok"];
    const stood = codes.map(standing);
    return order.find(stands => stood.includes(stands)) ?? "";
}

// The records in what spotify-recent printed, a line each: { uri, kind,
// name, detail, at }, where `at` is when it was last played, or `playing`
// or `paused`, and for a playlist listed whole, how many songs it holds.
// A line without its five fields is left out, as a cut-off one would be.
function records(text) {
    return text.split("\n")
        .map(line => line.split("\t"))
        .filter(fields => fields.length === FIELDS && fields[0] !== "")
        .map(([uri, kind, name, detail, at]) => ({ uri, kind, name, detail, at }));
}

// Whether a record is what Spotify has now, playing or paused.
function current(record) {
    return record.at === "playing" || record.at === "paused";
}

// What a row of Recent says beside it: whether it plays now, and once
// over, its kind and how long ago it last played, in the words `said`
// gives for each, as "playlist · 2 h ago"; Liked Songs says only when,
// its name saying what it is. What plays now says so by `playing`,
// Spotify's own player's live state where it has one, so a pause after
// it was read still shows.
function beside(record, playing, now, said) {
    if (current(record)) {
        const live = playing === null ? record.at === "playing" : playing;
        return said[live ? "playing" : "paused"] ?? "";
    }
    const at = Date.parse(record.at);
    const kind = record.kind === "collection" ? "" : said[record.kind] ?? record.kind;
    const when = Number.isNaN(at) ? "" : Ago.ago(at, now);
    return [kind, when].filter(part => part !== "").join(" \u00b7 ");
}

// A count as it's said, grouped by thousands, and `one` or `many` with it
// in place of {count}.
function counted(count, one, many) {
    const grouped = String(count).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    return (count === 1 ? one : many).replace("{count}", grouped);
}

// The link anyone can open for a Spotify URI: open.spotify.com's, as
// Spotify's own Share copies it. Liked Songs is the account's own, so its
// link opens whoever's are signed in.
function link(uri) {
    const parts = uri.split(":");
    if (parts[0] !== "spotify") {
        return "";
    }
    if (parts[1] === "collection" || uri.endsWith(":collection")) {
        return "https://open.spotify.com/collection/tracks";
    }
    return `https://open.spotify.com/${parts.slice(1).join("/")}`;
}

// What Spotify's program row says while Spotify can't be read, in the
// words `said` gives: signed out, or out of reach; nothing while it
// answers or hasn't been asked.
function note(stands, said) {
    if (stands === "signedOut") {
        return said.signedOut ?? "";
    }
    if (stands === "unreachable") {
        return said.unreachable ?? "";
    }
    return stands === "failed" ? said.failed ?? "" : "";
}
