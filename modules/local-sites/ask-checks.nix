# The checks of the sites' ask row, which local-sites' checks.nix gathers
# beside its own, kept apart since they change with the kit's ask.ts and
# the launcher's look code rather than with any site.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  launcher = ../quickshell/launcher;
  # Core's words for the launcher, and the one words.json Words.qml reads.
  words = import ../quickshell/launcher/words.nix;
  coreWords = words.value;

  # The kit and one of the row's tests in tests/, run in Node with the
  # launcher's files it reads handed in by the names tests/launcher.ts reads.
  rowTest =
    name: test: files:
    pkgs.runCommandLocal name { } ''
      cp -r ${./kit} kit
      mkdir tests
      cp ${./tests/launcher.ts} tests/launcher.ts
      cp ${test} tests/${baseNameOf test}
      ${lib.concatStrings (lib.mapAttrsToList (file: path: "${file}=${path} ") files)}\
        ${lib.getExe pkgs.nodejs} --test --test-reporter=spec tests/${baseNameOf test}
      touch $out
    '';
in
{
  # The kit's ask row reads a look as the launcher's For You does, against
  # the launcher's own scripts run in Node, so either side's change fails it
  # (tests/ask-as-for-you.test.ts). It's a check of its own so the kit, and
  # every site built on it, isn't built again at each launcher edit.
  the-ask-row-reads-as-for-you =
    rowTest "the-ask-row-reads-as-for-you" ./tests/ask-as-for-you.test.ts
      {
        LOOK_FORM = launcher + "/LookForm.js";
        LOOKS = launcher + "/Looks.js";
        FOR_YOU = launcher + "/ForYou.js";
        ORDER = launcher + "/Order.js";
        NOW_QML = launcher + "/Now.qml";
        WORDS = words.file;
        LOOK_ACTS = pkgs.writeText "look-acts.json" (builtins.toJSON (import ./look-acts.nix));
      };

  # The row reads and judges every character as LookForm.js does
  # (tests/ask-reads-every-character.test.ts), apart from the check above
  # since only LookForm.js and ask.ts decide it and its sweep takes longest,
  # so an edit to Now.qml or the words doesn't run it again.
  the-ask-row-reads-each-character-as-for-you =
    rowTest "the-ask-row-reads-each-character-as-for-you" ./tests/ask-reads-every-character.test.ts
      { LOOK_FORM = launcher + "/LookForm.js"; };
}
# They evaluate the graphical machine, whose launcher has the words.
// onLinux {
  # The home module refuses a machine whose launcher renames one of its acts
  # on a look, in place or by an entry after core's (its id a list of one
  # set words.json writes as the id, as the shell's Words.qml keys it all
  # the same), naming only the act's
  # old and new word, and takes core's own words. Each case's machine is
  # evaluated once, since an evaluation is most of what this costs.
  local-sites-refuses-renamed-look-acts =
    let
      clear = "verb:on:look:clear";
      talk = "verb:agent:talk";
      # The graphical machine's account with the sites, its launcher saying `words`.
      sites =
        said:
        account (
          graphical.extendModules {
            modules = [
              {
                home-manager.users.someone = {
                  imports = [ self.homeModules.local-sites ];
                  jitsusama.quickshell.launcher.words = said;
                };
              }
            ];
          }
        );
      # What the home module says refusing renamed acts, or none.
      refused =
        said:
        lib.findFirst (
          check: !check.assertion && lib.hasInfix "renames the launcher's acts" check.message
        ) null (sites said).assertions;
      # A word core has that's no act on a look, renamed beside one.
      other = (lib.findFirst (thing: !(lib.elem thing.id lookActs)) null coreWords.things).id;
      inPlace = coreWords // {
        things = map (
          thing: if (thing.id or null) == clear then thing // { name = "Drop It"; } else thing
        ) coreWords.things;
      };
      appended = coreWords // {
        things = coreWords.things ++ [
          {
            # Written to words.json as the id itself, as toJSON writes a set
            # with an outPath.
            id = [ { outPath = clear; } ];
            name = "Dismiss";
          }
          {
            # Written as the string its __toString gives.
            id = {
              __toString = _: talk;
            };
            name = "Chat";
          }
          {
            id = other;
            name = "Something Else";
          }
        ];
      };
      # Ids Words.qml keys as no act: written as objects, as a list of
      # several or none, or as a path in the store.
      odd = coreWords // {
        things = coreWords.things ++ [
          {
            id = {
              a = 1;
            };
            name = "Odd";
          }
          {
            id = [ { a = 1; } ];
            name = "Odder";
          }
          {
            id = {
              outPath = {
                a = 1;
              };
            };
            name = "Oddest";
          }
          {
            id = [
              clear
              "x"
            ];
            name = "Two";
          }
          {
            id = [ ];
            name = "None";
          }
          {
            id = {
              outPath = ./look-acts.nix;
            };
            name = "Stored";
          }
        ];
      };
      # A rename to a name that isn't words, still refused in words.
      unnamed = coreWords // {
        things = coreWords.things ++ [
          {
            id = clear;
            name = null;
          }
        ];
      };
      refusals = lib.mapAttrs (_: refused) {
        core = coreWords;
        inherit
          inPlace
          appended
          odd
          unnamed
          ;
      };
      says = name: refusal: refusals.${name} != null && lib.hasInfix refusal refusals.${name}.message;
      lookActs = import ./look-acts.nix;
    in
    holds "local-sites-refuses-renamed-look-acts"
      "local-sites' home module lets a machine rename one of the launcher's acts on a look, in place or by a later entry keyed by a list of one set or a set words.json writes as the id, refuses core's own words or ids written as objects, or doesn't name only the act's old and new word."
      (
        refusals.core == null
        && refusals.odd == null
        && says "inPlace" "acts on a look (${clear} from Clear It to Drop It),"
        && says "appended" "acts on a look (${clear} from Clear It to Dismiss, ${talk} from Talk to Chat),"
        && says "unnamed" "acts on a look (${clear} from Clear It to null),"
      );
}
