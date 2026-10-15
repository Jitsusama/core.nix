# How zsh behaves at the keyboard, set by zsh alone so a shell starts no
# program for it. home.nix gives _zsh_selected, the selected completion's
# colours as the escape's numbers.

# Emacs's keys, as a terminal's line editing has always had them, and each
# of the keys a keyboard has beyond them, in both the forms a terminal may
# send: the cursor's own and the keypad's.
bindkey -e
() {
  local key widget
  local -A keys=(
    '^[[A' up-line-or-beginning-search    '^[OA' up-line-or-beginning-search
    '^[[B' down-line-or-beginning-search  '^[OB' down-line-or-beginning-search
    '^[[H' beginning-of-line              '^[OH' beginning-of-line
    '^[[F' end-of-line                    '^[OF' end-of-line
    '^[[1;5C' forward-word                '^[[1;5D' backward-word
    '^[[3~' delete-char                   '^[[3;5~' kill-word
    '^[[5~' up-line-or-history            '^[[6~' down-line-or-history
    '^[[Z' reverse-menu-complete          '^?' backward-delete-char
    '^X^E' edit-command-line              ' ' magic-space
  )
  autoload -Uz up-line-or-beginning-search down-line-or-beginning-search edit-command-line
  zle -N up-line-or-beginning-search
  zle -N down-line-or-beginning-search
  zle -N edit-command-line
  for key widget in ${(kv)keys}; do
    bindkey -M emacs $key $widget
  done
}
# A word ends at any mark, so Ctrl+W takes one folder of a path at a time.
WORDCHARS=''

# A pasted URL is quoted as it arrives, and a typed one as it's typed.
autoload -Uz bracketed-paste-magic url-quote-magic
zle -N bracketed-paste bracketed-paste-magic
zle -N self-insert url-quote-magic

# A folder's name alone goes there, and each folder left is remembered, once,
# for cd -<Tab> to go back to.
setopt auto_cd auto_pushd pushd_ignore_dups pushd_minus

# A history expansion is shown to be run rather than run, the history file is
# locked by the kernel while shells share it, and a line can carry a comment.
setopt hist_verify hist_fcntl_lock interactive_comments long_list_jobs
unsetopt flow_control

# Completion offers a menu on the second Tab, chosen from by the arrows,
# matches in either case and from either end of what's typed, and lists
# files in the colours ls gives them, the selected one as a launcher row.
zmodload -i zsh/complist
setopt auto_menu complete_in_word always_to_end
unsetopt menu_complete
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{[:lower:][:upper:]}={[:upper:][:lower:]}' \
  'r:|=*' 'l:|=* r:|=*'
zstyle ':completion:*' special-dirs true
zstyle ':completion:*' list-colors "ma=$_zsh_selected" ${(s.:.)LS_COLORS}
zstyle ':completion:*:cd:*' tag-order local-directories directory-stack path-directories
zstyle ':completion:*' use-cache yes
zstyle ':completion:*' cache-path ${XDG_CACHE_HOME:-$HOME/.cache}/zsh/completion
zstyle '*' single-ignored show
unset _zsh_selected
