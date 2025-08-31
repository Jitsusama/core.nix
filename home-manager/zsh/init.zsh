# Disable oh-my-zsh auto-updates (declarative now)
zstyle ':omz:update' mode disabled

# Homebrew Environment (common on macOS)
[[ -x /opt/homebrew/bin/brew ]] && eval $(/opt/homebrew/bin/brew shellenv)

# Oh-my-zsh terminal title configuration (set after OMZ loads)
ZSH_THEME_TERM_TAB_TITLE_IDLE="🐚 %~"
ZSH_THEME_TERM_TITLE_IDLE="🐚 %~"