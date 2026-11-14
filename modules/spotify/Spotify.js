// What Spotify's program in the launcher works out from what spotify-recent
// prints (SpotifyProgram.qml): its lines read into records, what each says
// beside it and as its note, when it played, the link it copies, and how
// Spotify stands by the command's exit status. Plain JavaScript, so
// tests/spotify-program.qml reaches it in the shell's own engine.
.pragma library
.import "Ago.js" as Ago

// spotify-recent's exit statuses (src/spotify.rs's Refused), from the
// table every command of ours keeps (docs/commands.md), by how Spotify
// stands for each: 78 a sign-in to make, 66 nothing to play on, and 69
// out of reach or 75 too busy, both mended by waiting.
const STANDS = { 0: "ok", 78: "signedOut", 66: "nowhere", 69: "unreachable", 75: "unreachable" };

// The fields of a line: its URI, kind, name, who made it, when, and how
// many it holds.
const FIELDS = 6;

// The lists Joel keeps, each an inside of Spotify's program, in the order
// they're listed, by the words spotify-recent is asked them by.
const LISTS = ["playlists", "albums", "artists", "shows"];

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
// name, detail, at, size }, where `at` is when it was last played, or
// `playing` or `paused`, and `size` how many songs or episodes it holds,
// each empty where Spotify doesn't say. A line without its six fields is
// left out, as a cut-off one would be.
function records(text) {
    return text.split("\n")
        .map(line => line.split("\t"))
        .filter(fields => fields.length === FIELDS && fields[0] !== "")
        .map(([uri, kind, name, detail, at, size]) => ({ uri, kind, name, detail, at, size }));
}

// When a record was last played, in milliseconds, as Recent lays rows out
// by: what plays now as `now`, since it's newer than anything played, and
// NaN for one that says no time.
function when(record, now) {
    return current(record) ? now : Date.parse(record.at);
}

// Whether a record is what Spotify has now, playing or paused.
function current(record) {
    return record.at === "playing" || record.at === "paused";
}

// What a row of Recent says beside it: whether it plays now, and once
// over, how long ago it last played, as "2 h ago", its note saying what
// it is. What plays now says so by `playing`, Spotify's own player's live
// state where it has one, so a pause after it was read still shows.
function beside(record, playing, now, said) {
    if (current(record)) {
        const live = playing === null ? record.at === "playing" : playing;
        return said[live ? "playing" : "paused"] ?? "";
    }
    const at = Date.parse(record.at);
    return Number.isNaN(at) ? "" : Ago.ago(at, now);
}

// What a row says as its note, in the words `said` gives: its kind where
// `kinded`, how many songs it holds, or episodes for a show, and who made
// it, as "playlist · 46 songs · Joel", each left out where Spotify
// doesn't say.
function noted(record, kinded, said) {
    const size = (record.size ?? "") === "" ? ""
        : record.kind === "show"
            ? counted(Number(record.size), said.episode ?? "", said.episodes ?? "")
            : counted(Number(record.size), said.song ?? "", said.songs ?? "");
    const kind = kinded ? said[record.kind] ?? record.kind : "";
    return [kind, size, record.detail ?? ""].filter(part => part !== "").join(" \u00b7 ");
}

// A count grouped by thousands, as "1,234".
function grouped(count) {
    return String(count).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

// A count as it's said, grouped by thousands, and `one` or `many` with it
// in place of {count}.
function counted(count, one, many) {
    return (count === 1 ? one : many).replace("{count}", grouped(count));
}

// What Spotify's drawer in Programs says it holds, by each place's id in
// the launcher's words (VKCF3Z, 2026-11-08): how many songs are liked,
// then how many of each list Joel keeps, each once it's read and left out
// at none, and nothing while signed out, since what was read is no longer
// his to say.
function held(liked, lists, stands) {
    if (stands === "signedOut") {
        return [];
    }
    const counts = [["spotify:liked", liked]]
        .concat(LISTS.filter(name => name in lists)
            .map(name => [`spotify:${name}`, lists[name].length]));
    return counts.filter(count => count[1] > 0)
        .map(count => ({ id: count[0], value: grouped(count[1]) }));
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
