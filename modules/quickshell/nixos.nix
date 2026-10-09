# What Quickshell's lock screen needs from the system: a PAM service of its
# own, which checks the account's password and nothing else. Its polkit
# prompt goes through polkit's own service.
{
  security.pam.services.quickshell = { };
}
