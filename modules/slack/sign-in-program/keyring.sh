#!/bin/sh
# Prints pi's Slack sign-in from the desktop's keyring, with the
# secret-tool and busctl ../keyring.nix fills in, or $SECRET_TOOL and
# $BUSCTL, which tests/keyring.rs gives stand-ins as. It ends 0 having
# printed it, 75 (lib's LOCKED) when the keyring holds it locked, and 1
# when it holds none. secret-tool ends 1 saying nothing whether the item is
# locked or lacking, so the keyring's SearchItems, which never prompts,
# is asked which: it answers "aoao", the unlocked items' count and
# paths, then the locked items' count and paths.
SECRET_TOOL=${SECRET_TOOL:-@secretTool@} BUSCTL=${BUSCTL:-@busctl@}
"${SECRET_TOOL}" lookup service agentic-harness username slack-credentials && exit 0
found=$("${BUSCTL}" --user call org.freedesktop.secrets /org/freedesktop/secrets \
  org.freedesktop.Secret.Service SearchItems 'a{ss}' 2 \
  service agentic-harness username slack-credentials 2>/dev/null)
# shellcheck disable=SC2086 # split on purpose: the answer's words
set -- ${found}
[ "$#" -ge 3 ] && [ "$1" = aoao ] || exit 1
case $2 in
  '' | *[!0-9]*) exit 1 ;;
  *) [ "$#" -ge $(($2 + 3)) ] || exit 1 ;;
esac
shift $(($2 + 2))
case $1 in
  '' | *[!0-9]* | 0) exit 1 ;;
  *) exit 75 ;;
esac
