// The margin's puppy, visiting the cat once Joel has left the keys (Joel,
// 2026-11-12: "bat balls of yarn and stuff? And maybe throw in a little
// puppy too?"): it trots in from behind the edge of the cat's room, stands
// a moment as the cat notices it, bows to play as the cat eyes it, sits
// as the cat answers with a hop where it sits and flicks an ear, then
// turns and trots back behind the edge as the cat watches it go. The cat
// never leaves its seat. Its beats are the cat's way (MarginCatWays.js),
// so they keep the rules every play keeps, and the puppy is on a track of
// its own beside them, as the yarn is (MarginCatGames.js), drawn only
// within the cat's room, so it never appears or goes in open space.
// MarginCatPlays.js says when it comes. Imported as Puppy.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatGames.js" as Games
.import "MarginCatWays.js" as Ways

// How long each beat it holds lasts, in ms at the theme's pace: the cat's
// ears going up as it stands, the cat's eyes on it as it bows (as long as
// the cat thinks, besides), the cat's hop, and its ear after.
const NOTICES = 160;
const BOWS = 650;
const HOP = { crouches: 220, lands: 160 };
const LOOKS = 500;
// How many of the cat's cells it stops short of the cat.
const NEAR = 2;
// Its strides, in turn, as the cat's are, each pair of legs apart two and
// gathered two; and how it stands, as the cat's walking B is its standing.
const TROTS = ["puppyWalkingA", "puppyWalkingC", "puppyWalkingB", "puppyWalkingD"];
const STANDS = "puppyWalkingB";

// Its trot over `cells` of the cat's: at the pace the cat walks a way that
// long (MarginCat.js), so a short way is an amble and a long one never
// past a hurry, in `ms`; and its `stride`, the cells each drawing covers,
// two to three as the cat's are, so its paws are always seen moving: the
// cells its pace covers in STRIDE_SECONDS, rounded up as the cat's walk
// rounds them (MarginCatWays.js), so no drawing shows for less.
function trot(cells) {
    const pace = Math.min(Math.max(cells / (MarginCat.CRUISES / MarginCat.MS), MarginCat.AMBLES),
        MarginCat.HURRIES);
    const stride = Math.min(Math.max(Math.ceil(pace * MarginCat.STRIDE_SECONDS - 1e-6),
        MarginCat.SHORTEST_STRIDE), MarginCat.LONGEST_STRIDE);
    return { ms: (cells / pace) * MarginCat.MS, stride };
}

// The visit from where the cat sits, `at`, as Games.play takes it. It
// comes from the side with more room, which must hold the puppy and the
// gap it stops short by; null where it has too little, so the scene moves
// on. `puppy` is its track by the cat's beats, from `gone`, its left just
// behind the room's edge, to `near` the cat, facing it, and back.
function visit(at) {
    const spare = Games.room(at);
    if (spare === null) {
        return null;
    }
    const c = at.cell;
    const dir = spare.right >= spare.left ? 1 : -1;
    const ahead = dir > 0 ? spare.right : spare.left;
    if (ahead < (NEAR + MarginCat.CELLS) * c) {
        return null;
    }
    const x = at.seat.x;
    const past = cells => (dir > 0 ? x + at.wide + cells * c : x - at.wide - cells * c);
    const far = Math.floor(ahead / c) + 1;
    const gone = past(far);
    const near = past(NEAR);
    const trots = trot(far - NEAR);
    const still = pose => ({ from: near, to: near, pose, facing: -dir });
    const sits = still("puppySitting");
    const puppy = {
        within: { left: x - spare.left, right: x + at.wide + spare.right },
        stride: trots.stride,
        near,
        tracks: {
            waits: { from: gone, to: near, facing: -dir }, notices: still(STANDS),
            eyes: still("puppyBowing"), crouches: sits, hops: sits, lands: sits, looks: sits,
            watches: { from: near, to: gone, facing: dir }
        }
    };
    const way = Ways.begun(Object.assign({}, at, { facing: 0 }), at.seat);
    Ways.holds(way, "waits", trots.ms);
    way.facing = dir;
    Ways.holds(way, "notices", NOTICES);
    Ways.holds(way, "eyes", BOWS + (at.pause ?? 0));
    Ways.holds(way, "crouches", HOP.crouches);
    Ways.leap(way, at.seat, 1, "hops");
    Ways.holds(way, "lands", HOP.lands);
    Ways.holds(way, "looks", LOOKS);
    Ways.holds(way, "watches", trots.ms);
    return Object.assign(Ways.timed(way), { kind: "puppy", way: dir, yarn: null, puppy });
}

// Where a play has the puppy `t` ms in: its left `x` and `y`, its drawing
// by the theme's name, the way it faces and the room it's drawn `within`,
// as the yarn is said (Games.frame); null where the play has none, or is
// done. Trotting, it steps in whole cells at a steady pace, its drawing
// the next of its strides each `stride` cells it covers. They're counted
// from where it stands near the cat, the cell before it coming in, since
// that last cell is the notice's standing: so a way in that isn't whole
// strides leaves its short one behind the room's edge, not where it
// arrives in plain sight.
function frame(playing, t) {
    const beat = playing.puppy ? playing.beats.find(one => t < one.ends) : null;
    const track = beat ? playing.puppy.tracks[beat.name] : null;
    if (!track) {
        return null;
    }
    const c = playing.cell;
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const x = Ways.onCells(track.from + (track.to - track.from) * p, track.from, track.to, c);
    const near = playing.puppy.near;
    const coming = track.to === near && track.from !== near;
    const strides = cells => Math.floor(Math.max(cells, 0) / playing.puppy.stride + 1e-6);
    const seen = Math.abs(x - near) / c - (coming ? 1 : 0);
    // Coming in, its strides run on from the first, which it takes at the edge.
    const taken = coming ? strides(Math.abs(track.from - near) / c - 1) - strides(seen)
        : strides(seen);
    return {
        x,
        y: playing.from.y,
        pose: track.pose ?? TROTS[taken % TROTS.length],
        facing: track.facing,
        within: playing.puppy.within
    };
}
