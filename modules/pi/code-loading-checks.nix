# The check that a realm giving a variable which would run code as its pi
# host is refused by name, which host-checks.nix gathers: kept apart from the
# hosts' other checks since it changes with that refusal's words alone.
{
  account,
  bareNixos,
  holds,
  lib,
  loadsCode,
  pkgs,
  proofOf,
  self,
}:
{
  # A realm's own NODE_OPTIONS, or any other variable a host's unit clears
  # since it would run code as the host, fails evaluation, naming the realm
  # and each variable, rather than being dropped unseen.
  pi-hosts-refuse-code-loading-variables =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      realm = environment: {
        pi = "${runtime}/lib/node_modules/pi";
        node = "${pkgs.emptyDirectory}/node";
        proof = proofOf { inherit runtime; };
        inherit environment;
      };
      # A realm giving each alone, named for it: node_options and so on.
      realmOf = name: lib.toLower name;
      checks =
        (account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.pi.hosts =
              lib.listToAttrs (
                map (
                  name:
                  lib.nameValuePair (realmOf name) (realm {
                    ${name} = "/x";
                  })
                ) loadsCode
              )
              // {
                plain = realm { GREETING = "hi"; };
                # Two, beside one that loads nothing, which the
                # refusal leaves out.
                both = realm {
                  OPENSSL_CONF = "/x.cnf";
                  LD_PRELOAD = "/x.so";
                  GREETING = "hi";
                };
                three = realm {
                  OPENSSL_MODULES = "/x";
                  NODE_PATH = "/x";
                  LD_AUDIT = "/x.so";
                };
              };
          }
        ])).assertions;
      refusals = map (check: check.message) (lib.filter (check: !check.assertion) checks);
      # The one refusal naming a realm, or none.
      prefixOf = realm: "jitsusama.pi.hosts.${realm}.environment gives ";
      refusalOf = realm: lib.findFirst (lib.hasPrefix (prefixOf realm)) null refusals;
      # Whether a realm's refusal names just these variables, each once in
      # whatever order host.nix lists them, as "A", "A and B" or "A, B and
      # C", and speaks of one as it and of several as each; then names each
      # of loadsCode again where it says how they load code, the
      # conversations' own node options a realm sets instead, and where a
      # command gets one. How each loads code is the message's to word, so
      # only its names are pinned.
      names =
        realm: given:
        let
          refused = refusalOf realm;
          one = lib.length given == 1;
          clears = ", which a pi host's unit clears: ${if one then "it" else "each"} could make";
          parts = lib.splitString clears (lib.removePrefix (prefixOf realm) refused);
          named = lib.head parts;
          said = lib.splitString ", " (lib.replaceStrings [ " and " ] [ ", " ] named);
          # The names said, laid out again as a list of them is written.
          written =
            if lib.length said < 2 then
              lib.concatStrings said
            else
              "${lib.concatStringsSep ", " (lib.init said)} and ${lib.last said}";
          explained = lib.concatStringsSep clears (lib.tail parts);
        in
        refused != null
        && lib.hasPrefix (prefixOf realm) refused
        && lib.length parts == 2
        && named == written
        && lib.sort lib.lessThan said == lib.sort lib.lessThan given
        && lib.all (name: lib.hasInfix " ${name}" explained) loadsCode
        && lib.hasInfix (if one then " Drop it. " else " Drop each one. ") refused
        && lib.hasInfix "conversationHeapMb" refused
        && lib.hasInfix "conversationHeapGrowingPercent" refused
        && lib.hasSuffix " a command that needs one sets it itself." refused;
    in
    holds "pi-hosts-refuse-code-loading-variables"
      "A realm giving NODE_OPTIONS, NODE_PATH, NODE_COMPILE_CACHE, LD_PRELOAD, LD_AUDIT, LD_LIBRARY_PATH, OPENSSL_CONF or OPENSSL_MODULES, which its pi host's unit clears, was taken unseen, or the refusal didn't name the realm and only the ones it gave, speaking of one as it and of several as each, then name each of the eight again where it says how they load code, name the options a conversation's own node takes instead, and say that a command needing one sets it itself."
      (
        refusalOf "plain" == null
        && lib.all (name: names (realmOf name) [ name ]) loadsCode
        && names "both" [
          "LD_PRELOAD"
          "OPENSSL_CONF"
        ]
        && names "three" [
          "OPENSSL_MODULES"
          "NODE_PATH"
          "LD_AUDIT"
        ]
      );
}
