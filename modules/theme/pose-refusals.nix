# The margin's animals' pose rules, and the reader of the cat's walk, shown
# refusing what they're for as well as passing, for two of
# modules/theme/checks.nix's checks; its own file, as it changes only with
# the faults planted for them.
{
  lib,
  poseFaults,
  posesOf,
  walkRead,
}:
let
  walkIn = file: (walkRead file).walk;
  # The cat's sitting and strides as they're drawn today, copied and read
  # from nothing else, so a cat redrawn or renamed fails its own pose check
  # rather than these plants, which are made from them.
  theirs = lib.mapAttrs (_: pixels: { inherit pixels; }) {
    nothingHere = [
      "#...#..."
      "#####..."
      "#.#.#..."
      "#####..."
      ".###...."
      "#####..#"
      "#####..#"
      "#######."
    ];
    walkingA = [
      ".......#"
      "#.#...#."
      "###...#."
      "#.#####."
      "#######."
      ".######."
      ".#...#.."
      "#.#.#.#."
    ];
    walkingB = [
      "......#."
      "#.#...#."
      "###...#."
      "#.#####."
      "#######."
      ".######."
      ".#...#.."
      "##..##.."
    ];
    walkingC = [
      "......#."
      "#.#...#."
      "###...#."
      "#.#####."
      "#######."
      ".######."
      ".#...#.."
      "##..#.#."
    ];
    walkingD = [
      ".......#"
      "#.#...#."
      "###...#."
      "#.#####."
      "#######."
      ".######."
      ".#...#.."
      "#.#.##.."
    ];
  };
  # The cat's walk as it's drawn, so the planted poses are held to it
  # whether or not its code is read.
  drawnWalk = [
    "walkingA"
    "walkingC"
    "walkingB"
    "walkingD"
  ];
  rows = theirs.nothingHere.pixels;
  # The sitting cat, 37 pixels, with a pixel lit alone at its top right;
  # with six of its bottom row's seven gone, over 15% fewer, or five,
  # under it; and with its top four rows filled and a pixel by them, six
  # more, over 15% more, or its top three, five more, under it.
  filled = lib.genList (_: "#####...") 5;
  # A row with column `x` lit if it wasn't, or unlit if it was.
  flip =
    x: row:
    builtins.substring 0 x row
    + (if builtins.substring x 1 row == "#" then "." else "#")
    + builtins.substring (x + 1) 8 row;
  # A stride with the pixel at row `y`, column `x` flipped.
  flipped =
    pose: y: x:
    let
      moved = at: row: if at == y then flip x row else row;
    in
    lib.imap0 moved theirs.${pose}.pixels;
  small = lib.take 4 filled;
  # Stride A with its feet's row drawn as `row`.
  feet = row: lib.init theirs.walkingA.pixels ++ [ row ];
  # Stride A cut to an odd width, seven columns.
  odd = map (row: builtins.substring 0 7 row) theirs.walkingA.pixels;
  planted = theirs // {
    # Stride C with a pixel of its body moved: on its top row by the
    # tail's tip, on the row below, high in it, or just above its feet.
    topped.pixels = flipped "walkingC" 0 5;
    necked.pixels = flipped "walkingC" 1 3;
    shifted.pixels = flipped "walkingC" 3 1;
    edged.pixels = flipped "walkingC" 3 7;
    raised.pixels = flipped "walkingC" 6 2;
    # Stride C with only its tail's tip moved, a pixel along the top
    # row's last two columns, which no body rule reads.
    tippedC.pixels = flipped "walkingC" 0 7;
    # Stride A with one foot moved, either side of the feet's middle,
    # and on an odd width with its middle column's or its last moved,
    # both the hind half's.
    foreFoot.pixels = flipped "walkingA" 7 3;
    hindFoot.pixels = flipped "walkingA" 7 4;
    # Stride A's feet stepping fore, hind, fore, hind and fore again,
    # back to A, so the walk shuffles only across its end.
    wrapB.pixels = feet "#.###.#.";
    wrapC.pixels = feet "#.##..#.";
    wrapD.pixels = feet "####..#.";
    wrapE.pixels = feet "#####.#.";
    oddA.pixels = odd;
    oddB.pixels = lib.init odd ++ [ (flip 3 (lib.last odd)) ];
    oddC.pixels = lib.init odd ++ [ (flip 6 (lib.last odd)) ];
    # The sitting with a pixel joined to the rest by a corner alone.
    cornered.pixels = lib.imap0 (y: row: if y == 4 then flip 5 row else row) rows;
    # A sitting of 20, and 3 more or fewer, 15% on the line, or 4; and
    # one of 13, and 2 more or fewer, just past it.
    small.pixels = small;
    atLine.pixels = small ++ [ "###....." ];
    pastLine.pixels = small ++ [ "####...." ];
    underAt.pixels = lib.take 3 filled ++ [ "##......" ];
    underPast.pixels = lib.take 3 filled ++ [ "#......." ];
    tight.pixels = lib.take 2 filled ++ [ "###....." ];
    justPast.pixels = lib.take 3 filled;
    justShort.pixels = lib.take 2 filled ++ [ "#......." ];
    strayed.pixels = [ "${builtins.substring 0 7 (lib.head rows)}#" ] ++ lib.tail rows;
    # A pixel two cells from the rest, no nearer.
    reached.pixels = [ "${builtins.substring 0 6 (lib.head rows)}#." ] ++ lib.tail rows;
    shrunk.pixels = lib.init rows ++ [ "#......." ];
    trimmed.pixels = lib.init rows ++ [ "##......" ];
    grown.pixels = lib.take 4 filled ++ [ "####...." ] ++ lib.drop 5 rows;
    fuller.pixels = lib.take 3 filled ++ lib.drop 3 rows;
  };
  plantedFaults =
    pose:
    poseFaults {
      sitting = "nothingHere";
      poses = [ pose ];
      strides = drawnWalk;
      meant = null;
      glyphs = planted;
    };
  shown = [
    {
      pose = "strayed";
      reason = "stray [\"strayed\"]";
    }
    {
      pose = "reached";
      reason = "stray [\"reached\"]";
    }
    {
      pose = "shrunk";
      reason = "unlike [\"shrunk\"]";
    }
    {
      pose = "grown";
      reason = "unlike [\"grown\"]";
    }
  ];
  unshown =
    lib.filter (
      one: (plantedFaults one.pose).ok || !lib.hasInfix one.reason (plantedFaults one.pose).said
    ) shown
    ++ lib.optional (!(plantedFaults "trimmed").ok) {
      pose = "trimmed";
      reason = "trimmed, just under 15% fewer, held as one";
    }
    ++ lib.optional (!(plantedFaults "fuller").ok) {
      pose = "fuller";
      reason = "fuller, just under 15% more, held as one";
    };
  said =
    strides:
    poseFaults {
      sitting = "nothingHere";
      poses = [ "nothingHere" ];
      walkedFrom = "here";
      meant = null;
      glyphs = planted;
      inherit strides;
    };
  # Each rule at its edge, so a rule loosened or tightened by a pixel,
  # or a name it reads dropped, fails: on a sitting of 20, a pose 15%
  # more held and one past it refused, and on one of 13, a pose 15.4%
  # more refused, as no 16% would; a misnamed sitting said undrawn,
  # rather than every pose unlike it, and a pose listed but undrawn, as a
  # leap's tuck taken out would be, said so; a stride no pose lists
  # still held to the stray and size rules; and a stride meant otherwise
  # than the words its animal's poses are picked by refused for it alone,
  # the same strides all meant so held, and an undrawn one said undrawn
  # rather than meant otherwise.
  sized = poseFaults {
    sitting = "small";
    poses = [
      "atLine"
      "pastLine"
      "underAt"
      "underPast"
    ];
    strides = drawnWalk;
    meant = null;
    glyphs = planted;
  };
  sizedTight = poseFaults {
    sitting = "tight";
    poses = [
      "justPast"
      "justShort"
    ];
    strides = drawnWalk;
    meant = null;
    glyphs = planted;
  };
  misnamed = poseFaults {
    sitting = "nothingHer";
    poses = [ "walkingA" ];
    strides = drawnWalk;
    meant = null;
    glyphs = planted;
  };
  untucked = poseFaults {
    sitting = "nothingHere";
    poses = [
      "nothingHere"
      "tucked"
    ];
    strides = drawnWalk;
    meant = null;
    glyphs = planted;
  };
  unlisted = poseFaults {
    sitting = "nothingHere";
    poses = [ "nothingHere" ];
    strides = drawnWalk ++ [
      "strayed"
      "shrunk"
    ];
    meant = null;
    glyphs = planted;
  };
  meantAs =
    { otherwise, strides }:
    poseFaults {
      sitting = "nothingHere";
      poses = [ "nothingHere" ];
      inherit strides;
      meant = "its own pose";
      glyphs = lib.mapAttrs (
        name: mark: mark // { means = if name == otherwise then "another's" else "its own pose"; }
      ) planted;
    };
  meaning = meantAs {
    otherwise = "walkingC";
    strides = drawnWalk;
  };
  allMeant = meantAs {
    otherwise = null;
    strides = drawnWalk;
  };
  meantUndrawn = meantAs {
    otherwise = null;
    strides = drawnWalk ++ [ "unseen" ];
  };
  meantOtherwise = strides: ''meant otherwise than "its own pose" ${builtins.toJSON strides}'';
  edges = [
    {
      faults = meaning;
      reason = "walkingC, a stride meant otherwise, refused for it alone";
      holds = !meaning.ok && lib.hasInfix (meantOtherwise [ "walkingC" ]) meaning.said;
    }
    {
      faults = allMeant;
      reason = "the same strides, every one meant as its poses are, held";
      holds = allMeant.ok && lib.hasInfix (meantOtherwise [ ]) allMeant.said;
    }
    {
      faults = meantUndrawn;
      reason = "an undrawn stride said undrawn, not meant otherwise";
      holds =
        !meantUndrawn.ok
        && lib.hasInfix ''undrawn ["unseen"]'' meantUndrawn.said
        && lib.hasInfix (meantOtherwise [ ]) meantUndrawn.said;
    }
    {
      faults = sized;
      reason = "atLine and underAt, 15% off, held, and pastLine and underPast refused";
      holds =
        let
          refusing = builtins.match ''.*unlike \["pastLine","underPast"[],].*'' sized.said != null;
          holding = !lib.hasInfix ''"atLine"'' sized.said && !lib.hasInfix "underAt" sized.said;
        in
        refusing && holding;
    }
    {
      faults = sizedTight;
      reason = "justPast and justShort, 2 more and fewer than a sitting of 13, refused";
      holds = builtins.match ''.*unlike \["justPast","justShort"[],].*'' sizedTight.said != null;
    }
    {
      faults = misnamed;
      reason = "a misnamed sitting said undrawn, and nothing unlike it";
      holds =
        let
          saying = text: lib.hasInfix text misnamed.said;
        in
        saying ''undrawn ["nothingHer"]'' && saying "unlike []";
    }
    {
      faults = untucked;
      reason = "tucked, listed but undrawn, said undrawn";
      holds = lib.hasInfix ''undrawn ["tucked"]'' untucked.said;
    }
    {
      faults = unlisted;
      reason = "a stride no pose lists held to the stray and size rules";
      holds =
        lib.hasInfix ''stray ["strayed"]'' unlisted.said
        && builtins.match ''.*unlike \[[^]]*"shrunk".*'' unlisted.said != null;
    }
    {
      faults = plantedFaults "cornered";
      reason = "cornered, a pixel joined by a corner, held as one";
      holds = (plantedFaults "cornered").ok;
    }
    {
      faults = said tipped;
      reason = "tippedC, its tail's tip alone moved, held as stepping";
      holds = (said tipped).ok;
    }
  ];
  # A sitting, a stride and marks drawn as either's glyph, named for the
  # animal or meant as its poses are its poses, and no other mark: not one
  # with the name inside its own, nor one meant otherwise in another
  # glyph, nor one drawn as no glyph but a sitting drawn so, even when the
  # sitting or a stride isn't drawn at all.
  markedAs = glyph: { inherit glyph; };
  marks = {
    sat = markedAs "s";
    strode = markedAs "w";
    asSitting = markedAs "s";
    asStride = markedAs "w";
    petNamed = markedAs "x";
    other = markedAs "x";
    carpet = markedAs "x";
    bare = { };
    unmarked = { };
    ownMeant = {
      glyph = "m";
      means = "its own pose";
    };
    otherMeant = {
      glyph = "m";
      means = "another's";
    };
  };
  picked =
    sitting: named:
    posesOf {
      inherit sitting;
      strides = [ "strode" ];
      glyphs = marks;
      meant = null;
      inherit named;
    };
  picks = [
    {
      found = posesOf {
        sitting = "sat";
        strides = [ "strode" ];
        glyphs = marks;
        named = null;
        meant = "its own pose";
      };
      wanted = [
        "asSitting"
        "asStride"
        "ownMeant"
        "sat"
        "strode"
      ];
    }
    {
      found = picked "sat" "pet";
      wanted = [
        "asSitting"
        "asStride"
        "petNamed"
        "sat"
        "strode"
      ];
    }
    {
      found = picked "sat" null;
      wanted = [
        "asSitting"
        "asStride"
        "sat"
        "strode"
      ];
    }
    {
      found = picked "bare" null;
      wanted = [
        "asStride"
        "bare"
        "strode"
      ];
    }
    {
      found = posesOf {
        sitting = "gone";
        strides = [
          "strode"
          "absent"
        ];
        glyphs = marks;
        named = null;
        meant = null;
      };
      wanted = [
        "asStride"
        "strode"
      ];
    }
  ];
  tipped = [
    "walkingA"
    "tippedC"
    "walkingB"
    "walkingD"
  ];
  # Code read as no walk is judged by the reader's own faults alone, and
  # code read as a walk by the walk it reads, so a fault in the pose rules
  # isn't blamed on it.
  refused =
    walk:
    if walk ? read then
      walk.strides == walk.read
    else if walk ? wanted then
      walk.strides == [ ]
      && lib.elem walk.wanted walk.faults
      && (!(walk.first or false) || lib.head walk.faults == walk.wanted)
    else
      !(said walk.strides).ok && lib.hasInfix walk.reason (said walk.strides).said;
  # The cat's two declarations, as its code has them.
  drawnSaid = ''["walkingA", "walkingC", "walkingB", "walkingD"]'';
  walksSaid = walksOf drawnSaid;
  stridingSaid = "const STRIDING = Object.freeze({ goes: WALKS, climbs: WALKS });";
  # A walk unlike the drawn one, so reading a copy of that instead fails.
  skidded = walksOf ''["walkingA", "walkingB"]'';
  # Either declared as a constant of its own, frozen.
  walksOf = list: "const WALKS = Object.freeze(${list});";
  stridingOf = strides: "const STRIDING = Object.freeze({ ${strides} });";
  # Code read as no walk, and the rule it's refused by.
  walkedOf = wanted: ways: {
    strides = walkIn ways;
    reason = "code to be read as no walk";
    inherit wanted;
    inherit (walkRead ways) faults;
  };
  quoted = walkedOf "WALKS frozen as quoted words";
  byWalks = walkedOf "STRIDING frozen as strides by WALKS";
  elsewhere = walkedOf "WALKS named nowhere else";
  once = walkedOf "WALKS and STRIDING each declared once";
  # Said first of all, as the cause of the rest.
  found =
    ways:
    let
      wanted = "WALKS and STRIDING each found outside a comment";
    in
    walkedOf wanted ways // { first = true; };
  strideWalks = [
    {
      strides = [
        "walkingA"
        "walkingB"
        "walkingC"
        "walkingD"
      ];
      reason = "walkingA to walkingB, both pairs moved";
    }
    {
      strides = [ "walkingA" ];
      reason = "walkingA to walkingA, no pair moved";
    }
    {
      strides = [
        "walkingA"
        "walkingC"
      ];
      reason = "never stepping [\"hind\"]";
    }
    {
      strides = [
        "nothingHere"
        "walkingA"
      ];
      reason = "nothingHere to walkingA, the body moved";
    }
    {
      strides = [
        "walkingA"
        "topped"
      ];
      reason = "walkingA to topped, the body moved";
    }
    {
      strides = [
        "walkingA"
        "necked"
      ];
      reason = "walkingA to necked, the body moved";
    }
    {
      strides = [
        "walkingA"
        "shifted"
      ];
      reason = "walkingA to shifted, the body moved";
    }
    # One foot moved each way, so the fore half is the first four
    # columns, and the hind the rest.
    {
      strides = [
        "walkingA"
        "foreFoot"
      ];
      reason = "never stepping [\"hind\"]";
    }
    {
      strides = [
        "walkingA"
        "hindFoot"
      ];
      reason = "never stepping [\"fore\"]";
    }
    {
      strides = [
        "walkingA"
        "edged"
      ];
      reason = "walkingA to edged, the body moved";
    }
    {
      strides = [
        "walkingA"
        "raised"
      ];
      reason = "walkingA to raised, the body moved";
    }
    {
      strides = [
        "walkingA"
        "walkingZ"
      ];
      reason = "undrawn [\"walkingZ\"]";
    }
    # The fore pair stepping twice, then the hind: each step moves one
    # pair, and each pair steps, but out of turn.
    {
      strides = [
        "walkingA"
        "walkingC"
        "walkingA"
        "walkingD"
      ];
      reason = "walkingC to walkingA, the fore pair again";
    }
    {
      strides = [
        "walkingA"
        "wrapB"
        "wrapC"
        "wrapD"
        "wrapE"
      ];
      reason = "walkingA to wrapB, the fore pair again";
    }
    {
      strides = [ ];
      reason = "no strides";
    }
    {
      strides = [
        "oddA"
        "oddB"
      ];
      reason = "never stepping [\"fore\"]";
    }
    {
      strides = [
        "oddA"
        "oddC"
      ];
      reason = "never stepping [\"fore\"]";
    }
  ];
  codeWalks = [
    # A copy commented out above is no walk of the cat's: the one
    # below it is read as its own, which the copy is not.
    {
      strides = walkIn belowCopy;
      inherit (walkRead belowCopy) faults;
      read = [
        "walkingA"
        "walkingB"
      ];
      reason = "the walk below a copy commented out, read as its own";
    }
    (quoted "")
    (found "")
    (found "/* ${walksSaid}\n${stridingSaid} */")
    (found "/* ${walksSaid} */\n${stridingSaid}")
    (found "${walksSaid}\nconst glob = \"src/*\";\n${stridingSaid}")
    (byWalks walksSaid)
    (quoted stridingSaid)
    (byWalks "${walksSaid}\n${stridingOf ''goes: WALKS, climbs: ["walkingA"]''}")
    (byWalks "${walksSaid}\n${stridingOf "goes: WALKS, climbs: WALKS.slice(1)"}")
    (byWalks "${walksSaid}\n${stridingOf "goes: WALKS, climbs: RUNS"}")
    (quoted "${walksOf ''["walkingA", ...RUNS]''}\n${stridingSaid}")
    (quoted "${walksOf ''["walkingA", "walkingC"].concat(RUNS)''}\n${stridingSaid}")
    (byWalks "${walksSaid}\nconst STRIDING = Object.freeze({ goes: WALKS }) || RUNS;")
    (quoted "${walksOf ''["walking A", "walkingC"]''}\n${stridingSaid}")
    (quoted "${walksOf ''["walkingA', "walkingC"]''}\n${stridingSaid}")
    # Left unfrozen, or frozen only in part, code could change it after.
    (quoted "const WALKS = ${drawnSaid};\n${stridingSaid}")
    (byWalks "${walksSaid}\nconst STRIDING = { goes: WALKS, climbs: WALKS };")
    (quoted "${walksOf ''["walkingA", "walkingC"]).slice(0''}\n${stridingSaid}")
    (quoted "${walksOf ''["walkingA"]).concat(["walkingC"]''}\n${stridingSaid}")
    (byWalks "${walksSaid}\nconst STRIDING = Object.freeze({ goes: WALKS }).climbs;")
    (byWalks "${walksSaid}\nconst STRIDING = Object.freeze({ goes: WALKS } || RUNS);")
    (elsewhere "${walksSaid}\n${stridingSaid}\nWALKS.reverse();")
    (elsewhere "${walksSaid}\n${stridingSaid}\nconst runs = [...WALKS];")
    # Declared again, in a block or as anything else, the name strides
    # by another walk there.
    (once "${walksSaid}\n${stridingSaid}\n{ const STRIDING = { goes: RUNS }; }")
    (once "${walksSaid}\n${stridingSaid}\n{ let WALKS = RUNS; }")
    (once "${walksSaid}\n${stridingSaid}\nfunction STRIDING() {}")
    (once "${walksSaid}\n${stridingSaid}\nclass STRIDING {}")
    (once "${walksSaid}\n${stridingSaid}\nfor (var STRIDING of RUNS) {}")
    (once "${walksSaid}\n${stridingOf "goes: WALKS"}\nconst\nSTRIDING = {};")
  ];
  # The same walk, spaced and quoted otherwise, still reads whole.
  long = "/* ${lib.concatStrings (lib.genList (_: "WALKS ") 20000)}*/";
  belowCopy = "// ${walksSaid}\n${skidded}\n${stridingSaid}";
  reread = rereadBy.walk;
  rereadSaid = faultsSaid rereadBy.faults;
  # A reader case says what was read and the reader's own faults, never
  # the pose rules' words, so a reader fault isn't blamed on them.
  wantedSaid =
    walk:
    "${walk.reason}: read ${builtins.toJSON walk.strides}, wanted none, for want of "
    + "${walk.wanted}${lib.optionalString (walk.first or false) " first"}, "
    + "though it wanted ${faultsSaid walk.faults}";
  faultsSaid = faults: if faults == [ ] then "nothing" else lib.concatStringsSep ", " faults;
  rereadBy = walkRead ''
    ${long}
    // WALKS is the cat's walk, which STRIDING strides by.
    /* STRIDING[kind] = WALKS reads alike, as a comment.
     * So does WALKS.reverse(); here,
       and STRIDING.goes = RUNS; on a line with no star. */
    const WALKSPEED = 3, OVERSTRIDING = 2, MYWALKS = 1;
    const early = kind => STRIDING[kind];
    const WALKS = Object
      .freeze( [
      'walkingA',
      'walkingC',
      'walkingB',
      'walkingD',
    ]);
    const  STRIDING=Object.freeze ({ goes: WALKS, climbs: WALKS });
    const strides = STRIDING[beat.kind];
    const joined = STRIDING[was.kind] === STRIDING[beat.kind];
    const pose = STRIDING[beat.kind][0]; // not WALKS.reverse()
    const kept = STRIDING[beat.kind] && (last[i] = pose);
    const before = STRIDING[beats[i - 1].kind] /* the last */ ?? none;
    const pace = Ledges.WALKS * Other.STRIDING;
    const gait = /* not WALKS.reverse() */ STRIDING[beat.kind];
    const length = pace
      * STRIDING[beat.kind].length;
    const all = [...STRIDING[beat.kind]];
    const deep = STRIDING[beats[order[i]].kind];
    const carried = STRIDING[beats[0]?.kind]?.includes(at.drawnIn) ? at.drawnInto : 0;
    const last = STRIDING[beat.kind].at(-1) === pose;
    const first = STRIDING[kind]?.[0] ?? STRIDING[kind].find(isOn);
    const right = (STRIDING[kind]) >= 3 ? STRIDING[kind].toSorted() : [];
  '';
  unrefusedOf =
    walks:
    map (
      walk:
      if walk ? read then
        "${walk.reason}: read ${builtins.toJSON walk.strides}, wanted "
        + "${builtins.toJSON walk.read}, for want of ${faultsSaid walk.faults}"
      else if walk ? wanted then
        wantedSaid walk
      else
        "${walk.reason}, said ${(said walk.strides).said}"
    ) (lib.filter (walk: !refused walk) walks);
in
{
  inherit drawnWalk reread rereadSaid;
  poses =
    unrefusedOf strideWalks
    ++ map (one: "${one.reason}, said ${(plantedFaults one.pose).said}") unshown
    ++ map (one: "${one.reason}, said ${one.faults.said}") (lib.filter (one: !one.holds) edges)
    ++ map (one: "poses ${builtins.toJSON one.wanted}, found ${builtins.toJSON one.found}") (
      lib.filter (one: one.found != one.wanted) picks
    );
  code = unrefusedOf codeWalks;
}
