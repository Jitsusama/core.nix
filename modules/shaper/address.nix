# The address a house is named by, as the firewall and Rust's parse read
# it alike, for the shaper's houses (nixos.nix) and for house-report's
# shaped machine (../house-report/system.nix): IPv4's four octets, or
# IPv6's eight groups with at most one :: for a run of them. An IPv4 one
# written as IPv6, mapped or in the old compatible form, in any spelling,
# is refused: the firewall would match it as IPv6, which no IPv4 report
# is. Rust's to_ipv4 refuses the same.
lib:
let
  octet = "(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])";
  group = "[0-9a-fA-F]{1,4}";
  isIpv4 = text: builtins.match "${octet}([.]${octet}){3}" text != null;
  isIpv6 =
    text:
    let
      halves = lib.splitString "::" text;
      groups = lib.concatMap (half: lib.optionals (half != "") (lib.splitString ":" half)) halves;
      each = lib.all (part: builtins.match group part != null) groups;
      counted =
        if lib.length halves == 1 then
          lib.length groups == 8
        else
          lib.length halves == 2 && lib.length groups <= 7;
    in
    each && counted;
  # A well-formed IPv6 address's eight groups, its :: written out.
  groupsOf =
    text:
    let
      halves = lib.splitString "::" (lib.toLower text);
      parts = half: lib.optionals (half != "") (lib.splitString ":" half);
      head = parts (lib.head halves);
      tail = lib.optionals (lib.length halves == 2) (parts (lib.last halves));
      run = lib.optionals (lib.length halves == 2) (
        lib.genList (_: "0") (8 - lib.length head - lib.length tail)
      );
    in
    head ++ run ++ tail;
  isIpv4In6 =
    text:
    let
      groups = groupsOf text;
      zero = part: builtins.match "0+" part != null;
      sixth = lib.elemAt groups 5;
    in
    lib.all zero (lib.take 5 groups) && (zero sixth || sixth == "ffff");
in
lib.types.addCheck lib.types.str (text: isIpv4 text || isIpv6 text && !isIpv4In6 text)
// {
  description = "IPv4 or IPv6 address";
}
