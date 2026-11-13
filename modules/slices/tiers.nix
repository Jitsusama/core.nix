# Agents' work in the order Joel ranks it. Each tier names the quests that
# head it, and a quest's work goes in its slice inside the tier of its
# nearest ancestor a tier names, as the harness's quest tree says; any other
# quest's, and work for none, goes in the unranked tier, last
# (jobs-sorter/src/tiers.rs). The tiers are slices side by side inside
# jobs-work.slice, so CPU goes by tier rather than by how many quests each
# has open: a tier with one quest gets as much as one with ten.
#
# Each tier weighs twice the one after it, the unranked one half the last,
# so while work waits for CPU the first tier's runs about twice as fast as
# the second's, and every tier still moves. jobs.slice stays idle beside
# Joel's own programs, so none of this takes anything from them.
#
# The map is a file the sorter reads again when it changes, so a switch
# that moves a quest between tiers moves its work without restarting the
# sorter, and the journal says each move once.
#
# When memory runs out, the kernel ends the last tier's first: work goes
# 10 lower for each tier above the unranked one, and an agent's pi 25
# higher for each tier below the first, so work always goes before any pi
# (jobs-sorter/src/oom.rs, keep.rs). Past eight tiers, the first's work
# waiting elsewhere would fall under the last's pi, so eight is the most.
# systemd-oomd stays at its 80% everywhere: under its own ceiling, jobs'
# worst minute stalled 55% with memory free, so a lower limit for a lower
# tier would end builds that were only slowed.
{ config, lib, ... }:
let
  tiers = config.jitsusama.slices.tiers;
  names = map (tier: tier.name) tiers;
  # Halved once per rank, from 640 down, never under 1.
  weightAt = rank: lib.max 1 (lib.foldl' (weight: _: weight / 2) 640 (lib.range 1 rank));
  slice = rank: {
    CPUWeight = weightAt rank;
    IOWeight = weightAt rank;
  };
  heads = lib.listToAttrs (
    lib.concatMap (tier: map (quest: lib.nameValuePair quest tier.name) tier.quests) tiers
  );
in
{
  options.jitsusama.slices.tiers = lib.mkOption {
    description = ''
      Agents' work by tier, first to last, each with the quests that head
      it: a quest's work goes in the tier of the nearest of itself and its
      ancestors named here, and every other quest's in a last, unranked
      one. Empty, every quest's work sits in jobs-work.slice side by side.
    '';
    default = [ ];
    example = [
      {
        name = "first";
        quests = [ "QEST-20261024-ABC123" ];
      }
      {
        name = "second";
        quests = [ "QEST-20261001-DEF456" ];
      }
    ];
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          name = lib.mkOption {
            # A dash in a slice's name nests it, so a tier's has none.
            type = lib.types.strMatching "[a-z0-9]+";
            description = "The tier's name, which names its slice: jobs-work-<name>.slice.";
          };
          quests = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            description = "The quests that head the tier, by id.";
          };
        };
      }
    );
  };

  config = lib.mkIf (tiers != [ ]) {
    assertions = [
      {
        assertion = lib.allUnique names && !(lib.elem "unranked" names);
        message = "jitsusama.slices.tiers needs names of its own, none of them unranked.";
      }
      {
        assertion = lib.length names <= 8;
        message = "jitsusama.slices.tiers holds at most 8, so every tier's work goes before any pi.";
      }
    ];

    environment.etc."jobs-sorter/tiers.json".text = builtins.toJSON heads;

    systemd.user.slices =
      lib.listToAttrs (
        lib.imap0 (rank: name: lib.nameValuePair "jobs-work-${name}" { sliceConfig = slice rank; }) names
      )
      // {
        jobs-work-unranked.sliceConfig = slice (lib.length names);
      };
  };
}
