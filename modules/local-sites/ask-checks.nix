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
  # on a look, in place or by an entry after core's, naming the act's old
  # and new word, and takes core's own words.
  local-sites-refuses-renamed-look-acts =
    let
      clear = "verb:on:look:clear";
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
      says = said: refusal: refused said != null && lib.hasInfix refusal (refused said).message;
      inPlace = coreWords // {
        things = map (
          thing: if (thing.id or null) == clear then thing // { name = "Drop It"; } else thing
        ) coreWords.things;
      };
      appended = coreWords // {
        things = coreWords.things ++ [
          {
            id = clear;
            name = "Dismiss";
          }
        ];
      };
    in
    holds "local-sites-refuses-renamed-look-acts"
      "local-sites' home module lets a machine rename one of the launcher's acts on a look, in place or by a later entry, refuses core's own words, or doesn't name the act's old and new word."
      (
        refused coreWords == null
        && says inPlace "${clear} from Clear It to Drop It"
        && says appended "${clear} from Clear It to Dismiss"
      );
}
