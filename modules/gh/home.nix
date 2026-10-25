# GitHub's command line, signed in through its own credential helper for
# git over HTTPS.
{
  programs.gh = {
    enable = true;
    gitCredentialHelper.enable = true;
    settings = {
      git_protocol = "https";
      prompt = "enabled";
      aliases.co = "pr checkout";
    };
  };
}
