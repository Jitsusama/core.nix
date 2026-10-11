# A mark from the icon font by its code point, written in hex ("f313",
# "f075d"), since a Nix string can't spell one. JSON's \u takes exactly four
# hex digits, so a code point past U+FFFF, where most of the icon font's
# marks live, is written as its surrogate pair; written as one escape, it
# would read as the first four digits and a stray letter after them.
{ lib }:
code:
let
  point = lib.fromHexString code;
  escape = n: "\\u" + lib.fixedWidthString 4 "0" (lib.toHexString n);
  beyond = point - 65536;
in
builtins.fromJSON ''"${
  if point < 65536 then
    escape point
  else
    escape (55296 + beyond / 1024) + escape (56320 + lib.mod beyond 1024)
}"''
