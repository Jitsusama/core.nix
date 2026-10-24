# The checks of onepassword, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  ...
}:
{
  # 1Password's Chrome extension talks to the desktop app through a helper in
  # the onepassword group, and every account may unlock the app with its own
  # password.
  onepassword-works-with-chrome =
    holds "onepassword-works-with-chrome" "1Password lacks its browser helper or its polkit owners."
      (
        graphical.config.security.wrappers."1Password-BrowserSupport".group or null == "onepassword"
        && graphical.config.programs._1password-gui.polkitPolicyOwners == [ "someone" ]
      );
}
