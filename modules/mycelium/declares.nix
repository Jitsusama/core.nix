# The shapes of what a machine says of itself to mycelium (home.nix): what
# its node can't measure (`declares`), with the taints it refuses work by,
# the fences its agents run in and the doors each opens, the machine it runs
# within or that run within it, and the one-way flows text crosses between
# realms by. They change only when what a machine can declare does, apart
# from how the node is run.
{
  lib,
  isLinux,
  name,
}:
rec {
  taint = lib.types.submodule {
    options = {
      key = lib.mkOption {
        type = name;
        description = "What work names to tolerate the taint.";
      };
      effect = lib.mkOption {
        type = lib.types.enum [
          "never"
          "prefer_not"
        ];
        description = ''
          How hard it refuses work that doesn't tolerate it: never, or only
          while anywhere else will do.
        '';
      };
      why = lib.mkOption {
        type = lib.types.str;
        description = "Why, for a person reading where work went.";
      };
    };
  };
  declares = lib.types.submodule {
    options = {
      reach = lib.mkOption {
        type = lib.types.listOf name;
        example = [
          "home"
          "internet"
        ];
        description = "The networks work on this machine always reaches, wherever it is.";
      };
      probes = lib.mkOption {
        type = lib.types.attrsOf (lib.types.either lib.types.str (lib.types.listOf lib.types.str));
        default = { };
        example = {
          home = "router.lan:53";
          office = [
            "/run/current-system/sw/bin/test"
            "-e"
            "/run/office"
          ];
        };
        # The node tries each every 30 seconds, giving it 3 to answer. A
        # command suits a network an address can't tell apart, such as one
        # known by its Wi-Fi or a tunnel's link; it runs with no shell.
        description = "Networks reached at times, each by a host:port or command answering only then.";
      };
      outlives = lib.mkOption {
        type = lib.types.listOf (
          lib.types.enum [
            "window_close"
            "node_restart"
            "sleep"
            "reboot"
          ]
        );
        # A held session's holder is a process of its own, apart from its
        # window and the node, which Linux keeps through a suspend.
        default = lib.optionals isLinux [
          "window_close"
          "node_restart"
          "sleep"
        ];
        description = "What a session held on this machine outlives.";
      };
      costsMoney = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether work on this machine costs money.";
      };
      # A Mac's pi in pi-fence's launcher can't reach its own state, so the
      # realm's agent program, pi unless `agents` names another, held there
      # for a fenced kind starts alone where the machine declares pi fences
      # itself, so it is fenced only as it fences each command it runs.
      piFencesItself = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Whether pi here lays a sandbox profile round each command it runs,
          as a Mac's pi does from its harness, so the realm's agent program,
          as `agents` names it or else pi, held fenced or strict, starts
          alone rather than within the fence's launcher, told which fence to
          keep. Then the fence rests on that program: only the commands it
          runs are fenced, and only as it fences them itself, as pi does,
          never its own reads and calls. Refused on Linux, whose pi fences
          each command with Landlock instead, and whose node holds it so
          unasked.
        '';
      };
      taints = lib.mkOption {
        type = lib.types.listOf taint;
        default = [ ];
        description = ''
          What this machine refuses, or would rather not take, each by a key
          work tolerates it by. The machine enforces its own.
        '';
      };
      within = lib.mkOption {
        type = lib.types.nullOr another;
        default = null;
        example = {
          machine = "host-machine";
          file = "/run/host/said.json";
        };
        description = ''
          The machine this one runs within, as a virtual machine runs in its
          host, and the file, as this machine reaches it, in which the host
          says its power and room, which this machine then has as its own.
        '';
      };
      hosts = lib.mkOption {
        type = lib.types.listOf another;
        default = [ ];
        description = ''
          The machines that run within this one, each with the file, as this
          machine reaches it, that its node writes its power and room to
          each minute.
        '';
      };
      # The node adds these only to a sprout placement chooses for, so one
      # naming a machine, asking needs of its own or staying here starts as
      # asked; a mycelium that predates them ignores them.
      needsIn = lib.mkOption {
        type = lib.types.listOf needRule;
        # A rule naming neither folders nor words would name no work, so
        # it's refused rather than taken as meaning none.
        apply = map (
          rule:
          lib.throwIf (rule.folders == [ ] && rule.words == [ ])
            "jitsusama.mycelium.declares.needsIn: the rule needing ${lib.concatStringsSep ", " rule.needs} names no folder and no word"
            rule
        );
        default = [ ];
        example = [
          {
            folders = [ "src/inner" ];
            words = [ "forge.example" ];
            needs = [ "reach=inner" ];
          }
        ];
        description = ''
          What work sprouted here needs wherever placement sends it: work
          begun in one of a rule's folders or below, or whose words hold
          one of its words in any case, needs each of its needs. Work met
          only by a probe is refused while no machine's probe answers,
          rather than started where it can't work. It keeps work from a
          machine it can't work on, and is no fence: a sprout naming its
          own machine or needs, an agent's included, starts as it asks. A
          mycelium that predates this ignores it.
        '';
      };
      # A carry an orchestrator orders lands only in a kind at least as
      # strict as the session's own, behind no door it lacked, judged from
      # what each machine says here; a kind left unsaid is one no ordered carry lands in, so the
      # default refuses them all.
      fence = lib.mkOption {
        type = lib.types.submodule {
          options = lib.genAttrs [ "none" "fence" "strict" ] (
            kind:
            lib.mkOption {
              type = lib.types.nullOr doors;
              default = null;
              description = "The doors a session run ${kind} opens here, or null if none runs.";
            }
          );
        };
        default = { };
        example.strict = {
          builders = [ "remote" ];
          forges = [ "github" ];
          secrets = [ ];
        };
        description = ''
          The fences an agent session runs in here, by kind, unfenced (none),
          fenced (fence) or strictly fenced (strict), each with the doors it
          opens, so an orchestrator's carry never lands in a weaker kind, as
          each machine names it, or behind a door it lacked. The kinds aren't
          the same on each system: a Mac's strict reads the home folder but
          what pi's fence hides, where Linux's reads only what it lists, and
          it writes the account's temporary folder, yet both say strict. So a carry from Linux to a
          Mac lands looser in what its commands may read and write. Where
          the realm's agent program is held alone, on Linux and on a
          Mac declaring piFencesItself, the kind fences only the commands
          it runs, as it fences them itself, while its own reads, writes
          and calls stand outside it. So a carry from a Mac that doesn't
          declare it, to Linux or to a Mac that does, lands looser for the
          agent itself, which the carry's judge, reading kinds, can't see.
        '';
      };
    };
  };
  # Work begun in some folders, or naming some words, and what it needs,
  # each need as `mycelium sprout --need` takes it.
  needRule = lib.types.submodule {
    options = {
      # Mycelium compares folders part by part, expanding nothing, so a
      # folder written from `~` or `$`, with an empty, `.` or `..` part
      # (home itself, the root alone or a trailing `/` among them), or
      # with space at an end, would match nothing, or everything.
      folders = lib.mkOption {
        type = lib.types.listOf (
          lib.types.addCheck lib.types.str (
            folder:
            builtins.match "[~$].*|[[:space:]].*|.*[[:space:]]|.*/[$].*" folder == null
            && lib.all (part: part != "" && part != "." && part != "..") (
              lib.splitString "/" (lib.removePrefix "/" folder)
            )
          )
          // {
            description = "folder within home or whole, of plain parts";
          }
        );
        default = [ ];
        example = [ "src/inner" ];
        description = "The folders, each within home unless whole, of plain parts never from ~ or $, whose work or work below needs these.";
      };
      # A word with space at an end would miss the work it stands in.
      words = lib.mkOption {
        type = lib.types.listOf (
          lib.types.addCheck lib.types.str (word: word != "" && lib.trim word == word)
          // {
            description = "word with no space at either end";
          }
        );
        default = [ ];
        example = [ "forge.example" ];
        description = "The words, such as a link's host, whose naming anywhere in work's words, in any case, needs these.";
      };
      # Each value in the shape a machine says it: a machine as a realm
      # names one, an accelerator of a kind mycelium knows, and a reach,
      # always or by a probe, system, architecture or tool as a lowercase
      # name. Only the shape is judged here; which machine says it is
      # placement's to find.
      needs = lib.mkOption {
        type = lib.types.nonEmptyListOf (
          lib.types.strMatching (
            lib.concatStringsSep "|" [
              "cloud"
              "machine=[a-z0-9][a-z0-9-]*"
              "accelerator=(gpu|npu)"
              "(reach|system|arch|tool)=[a-z0-9][a-z0-9_.-]*"
            ]
          )
        );
        example = [ "reach=inner" ];
        description = "What such work needs: cloud, or kind=value, as `mycelium sprout --need` takes it.";
      };
    };
  };
  # What a fence opens, each door named as the fence's settings name it.
  # Each is said, since a door left out would read as one shut.
  doors = lib.types.submodule {
    options = {
      builders = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        description = "The builders a fenced command may build on.";
      };
      forges = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        description = "The forges a fenced command may reach.";
      };
      secrets = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        description = "The secrets released to a fenced command.";
      };
      # A carry lands only where no door is wider than here's, but the
      # person may allow one machine a wider door for sessions carried
      # from here. The machine there still declares every door it opens,
      # so the gain is said beside the allowance, never hidden. A name no
      # realm could give a machine fails here, rather than leaving the
      # carry refused with nothing said; a well-shaped name for a machine
      # that isn't there can't be caught here, since realms are read live.
      # Every realm's node reads the one declaration, so a name holds in
      # each realm with a machine of that name, and a mycelium older than
      # accepted doors ignores them, so the description says both.
      accepts = lib.mkOption {
        type = byMachine accepted;
        default = { };
        example.other-machine.secrets = [ "tokens" ];
        description = "The doors each other machine, by name, may open beyond these for a session carried from here. A name holds in every realm this machine joins, and a mycelium that predates accepted doors ignores them, refusing the carry as before.";
      };
    };
  };
  # An attribute set of `type`, keyed by machines as the realm names them.
  byMachine = type: namingMachines (lib.types.attrsOf type);
  # `machines`, whose merge refuses a key no realm could name a machine,
  # saying which. `addCheck` can't: the module system rebuilds a set of
  # submodules from its functor to give them their file, dropping any
  # check added beside it, so the rebuilt set is wrapped again too. The
  # names are read from the merged set, so one under a false `mkIf` is
  # dropped first, as attrsOf drops it. The functor is still attrsOf's, so
  # two declarations of this option merged by type would lose the check;
  # declares declares it once.
  namingMachines =
    machines:
    machines
    // {
      merge =
        loc: defs:
        let
          merged = machines.merge loc defs;
          misnamed = lib.filter (machine: !name.check machine) (lib.attrNames merged);
        in
        if misnamed == [ ] then
          merged
        else
          throw "${lib.showOption loc} names ${lib.concatStringsSep ", " misnamed}, which no realm could name a machine: ${name.description}.";
      substSubModules = modules: namingMachines (machines.substSubModules modules);
    };
  # The doors of each kind a session carried from here may gain there.
  accepted = lib.types.submodule {
    options = lib.genAttrs [ "builders" "forges" "secrets" ] (
      door:
      lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "The ${door} a carried session may gain there.";
      }
    );
  };
  # One machine within another, as either names it.
  another = lib.types.submodule {
    options = {
      machine = lib.mkOption {
        type = name;
        description = "The other machine's name in the realm.";
      };
      file = lib.mkOption {
        type = lib.types.str;
        description = "The file the two share, as this machine reaches it.";
      };
    };
  };
  flow = lib.types.submodule {
    options = {
      from = lib.mkOption {
        type = name;
        description = "The realm whose text may cross.";
      };
      to = lib.mkOption {
        type = name;
        description = "The realm it may cross into.";
      };
    };
  };
}
