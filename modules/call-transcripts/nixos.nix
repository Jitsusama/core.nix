# Where the machine sums its calls up, for the account's call transcripts
# (home.nix): its hardware's module says, since only that knows whether its
# graphics can hold the summarizer's model, about 16 GB through OpenVINO.
# With none, calls are written down but not summed up, and the model's
# 15 GB is never fetched.
{ lib, ... }:
{
  options.jitsusama.call-transcripts.summarizerDevice = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "GPU";
    description = ''
      The OpenVINO device calls are summed up on, or null where none can
      run the summarizer's model, which leaves each call written down with
      no summary.
    '';
  };
}
