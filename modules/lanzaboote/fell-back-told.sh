# shellcheck shell=bash
# Tells the session what the boot said in $FELL_BACK, once something shows
# notices: the account's manager starts before anything takes the name.
gdbus wait --session org.freedesktop.Notifications
notify-send --urgency=critical --app-name=Boot "Booted the last good generation" "$(< "$FELL_BACK")"
