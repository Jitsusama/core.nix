# sudo on a Mac takes a finger on Touch ID in place of the password, so a
# switch at the Mac is a touch. The password still works when Touch ID
# can't be used, as over SSH or with the lid shut. It's written to
# /etc/pam.d/sudo_local, which macOS updates leave alone.
{
  security.pam.services.sudo_local.touchIdAuth = true;
}
