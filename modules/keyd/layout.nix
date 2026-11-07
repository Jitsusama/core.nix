# The built-in keyboard's layout, for the shell to draw on Super+K while no
# board of Joel's own is plugged in (modules/quickshell/KeyboardLayout.qml):
# an ANSI laptop's keys, where each sits (laptop.json), each capped with what
# keyd's Colemak (colemak-dh.conf) makes it type, so the map is read from the
# file that lays the keys out and can't say other than they do. It's said in
# the shape the ErgoDox's is (modules/ergodox/describe.py), as one layer, in
# /etc/keyboard/layout.json, outside keyd's own folder, whose files it
# reads as its settings.
{ lib, ... }:
let
  geometry = lib.importJSON ./laptop.json;

  # Which US key each key sends instead, from keyd's lines such as "e = f".
  sends = lib.listToAttrs (
    map (pair: lib.nameValuePair (lib.elemAt pair 0) (lib.elemAt pair 1)) (
      lib.filter (pair: pair != null) (
        map (builtins.match "([a-z0-9]+) = ([a-z0-9]+)") (
          lib.splitString "\n" (builtins.readFile ./colemak-dh.conf)
        )
      )
    )
  );

  # Each US key by its cap and its words, for those the geometry doesn't
  # name: the letters, the digits and the function keys.
  letters = lib.genAttrs (lib.stringToCharacters "abcdefghijklmnopqrstuvwxyz") (letter: [
    (lib.toUpper letter)
    letter
  ]);
  digits = lib.genAttrs (map toString (lib.range 0 9)) (digit: [
    digit
    digit
  ]);
  functions = lib.listToAttrs (
    map (
      n:
      lib.nameValuePair "f${toString n}" [
        "F${toString n}"
        "f${toString n}, function key ${toString n}"
      ]
    ) (lib.range 1 12)
  );
  names = letters // digits // functions // geometry.names;

  # A row's keys, left to right, each starting where the one before ends.
  keysOf =
    y: row:
    let
      placed =
        made: key:
        let
          width = lib.elemAt key 1;
          named = names.${sends.${lib.elemAt key 0} or (lib.elemAt key 0)};
        in
        {
          x = made.x + width;
          keys = made.keys ++ [
            {
              inherit (made) x;
              inherit y;
              w = width;
              h = 1;
              cap = lib.elemAt named 0;
              words = lib.elemAt named 1;
              inherit (row) where;
            }
          ];
        };
      empty = {
        x = 0;
        keys = [ ];
      };
    in
    (lib.foldl' placed empty row.keys).keys;
in
{
  environment.etc."keyboard/layout.json".text = builtins.toJSON {
    layers = [
      {
        name = "Colemak";
        keys = lib.concatLists (lib.imap0 keysOf geometry.rows);
      }
    ];
  };
}
