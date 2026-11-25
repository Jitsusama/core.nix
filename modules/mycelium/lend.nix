# The shape of what the home realm's node lends the worker machines it made
# (home.nix's `lend`), in the names of the file mycelium reads. It changes
# only when what mycelium's lender reads, or what it may be trusted with,
# does, apart from how the node is run.
{ lib }:
let
  # A lend's upstream socket, relative to the node's runtime folder: no
  # part of it starts with a dot, so `..` can't climb out of it.
  socket = lib.types.strMatching "([A-Za-z0-9_-][A-Za-z0-9._-]*/)*[A-Za-z0-9_-][A-Za-z0-9._-]*";
in
lib.types.submodule {
  options = {
    vms = lib.mkOption {
      type = lib.types.nonEmptyListOf (
        lib.types.submodule {
          options = {
            prefix = lib.mkOption {
              type = lib.types.strMatching "[a-z0-9-]+";
              description = ''
                The start of the names of the machines this lend is for. A
                machine whose name starts as none does, or as two, is
                refused.
              '';
            };
            budget = lib.mkOption {
              type = lib.types.str;
              description = ''
                The budget, in `budgets`, their requests are carried through
                where they name none.
              '';
            };
            also = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = ''
                The other budgets, in `budgets`, they may name, as a forge
                beside their models.
              '';
            };
            requests_a_day = lib.mkOption {
              type = lib.types.ints.positive;
              description = ''
                How many requests each machine may make a day, through all
                its budgets, past which each is refused.
              '';
            };
          };
        }
      );
      description = "The machines lent to, each by the start of its name.";
    };
    budgets = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            upstream = lib.mkOption {
              # Whatever answers at the upstream reads every lent
              # machine's requests, its sessions' prompts, is handed
              # the key, and is taken as the model; and mycelium
              # doesn't ask who owns the socket. So it is only ever
              # within the node's runtime folder, XDG_RUNTIME_DIR, the
              # account's own at 0700, where fenced agents have one of
              # their own; never a whole path, which could lie in a
              # folder another account binds first, as /tmp. mycelium's
              # lender joins a relative upstream onto the folder
              # place::login_runtime finds, and refuses the request when
              # there is none.
              type = socket;
              example = "model-relay/agents.sock";
              description = ''
                The socket each request goes on to, relative to the node's
                runtime folder, and nowhere else.
              '';
            };
            host = lib.mkOption {
              type = lib.types.strMatching "[A-Za-z0-9.-]+(:[0-9]{1,5})?";
              description = "The host the requests are for, with no scheme.";
            };
            kind = lib.mkOption {
              type = lib.types.enum [
                "models"
                "forge"
              ];
              default = "models";
              description = ''
                What answers at the upstream: a model relay, told nothing of
                who asked, or a forge, told which machine asked, since it
                signs and pushes for each by name.
              '';
            };
            paths = lib.mkOption {
              # mycelium matches each exactly, or everything below it
              # when it ends `/*`, and only over plain bytes, refusing
              # a `.` or `..` part. So each names at least one plain
              # part: never `/` or `/*` alone, which would lend every
              # path the host has, nor a path no request could match.
              type = lib.types.nonEmptyListOf (
                lib.types.addCheck (lib.types.strMatching "(/[A-Za-z0-9_.-]+)+(/[*])?") (
                  path: !lib.any (part: part == "." || part == "..") (lib.splitString "/" path)
                )
              );
              description = ''
                The paths a request may ask for, each exactly, or with all
                below it where it ends `/*`; any other is refused.
              '';
            };
            auth_header = lib.mkOption {
              type = lib.types.strMatching "[A-Za-z0-9-]+";
              description = ''
                The header the key goes in. The machine's own of that name
                is dropped.
              '';
            };
            key = lib.mkOption {
              # Its program in the Nix store, with no `.` or `..` part
              # that could climb out of it, and carrying the context of
              # a store path, so the lend file keeps it from the garbage
              # collector and a bare string is refused. That shows the
              # word was spliced from a store path, not which package's,
              # nor that the program can't be a link to a file the
              # account writes, or that the words after it don't name
              # one: the account, trusted as home.nix's lendArgs says,
              # may do either.
              type = lib.types.nullOr (
                lib.types.addCheck (lib.types.nonEmptyListOf lib.types.str) (
                  words:
                  let
                    program = lib.head words;
                  in
                  lib.hasPrefix "${builtins.storeDir}/" program
                  && builtins.hasContext program
                  && !lib.any (part: part == "." || part == "..") (lib.splitString "/" program)
                )
              );
              default = null;
              description = ''
                The command, its program by its path in the Nix store, that prints
                the header's whole value on one line, which the node never
                logs; or null where the upstream adds its own.
              '';
            };
          };
        }
      );
      description = "The budgets a machine may be lent through, by name.";
    };
  };
}
