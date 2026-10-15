# Terminal Integration Features
# =============================
# Provides semantic escape sequences for modern terminal emulators
# to better understand shell state and improve user experience.

# Track exit status and emit OSC 133;D (command finished with status)
# Must run FIRST in precmd to capture the real exit code
function _emit_command_finished() {
  local ret=$?
  printf '\033]133;D;%s\007' "$ret"
}

# Emit OSC 133;A (prompt is starting)
function _emit_prompt_start() {
  printf '\033]133;A\007'
}

# Emit OSC 133;B (prompt done, command starting)
function _emit_command_start() {
  printf '\033]133;B\007'
}

# Emit OSC 7 (current working directory for terminal), the folder as a URL:
# a byte outside a URL path's own characters is percent-encoded, which most
# folders need none of.
function _emit_working_directory() {
  emulate -L zsh -o no_multibyte -o extended_glob
  local where=$PWD byte
  if [[ $where == *[^A-Za-z0-9/._~-]* ]]; then
    where=""
    for byte in ${(s::)PWD}; do
      if [[ $byte == [A-Za-z0-9/._~-] ]]; then
        where+=$byte
      else
        where+=%${(l:2::0:)$(( [##16] #byte ))}
      fi
    done
  fi
  printf '\033]7;file://%s%s\007' "$HOST" "$where"
}

# oh-my-zsh tells the terminal the folder too, the same way and slower, so
# only the one above does.
(( $+functions[omz_termsupport_cwd] )) && add-zsh-hook -d precmd omz_termsupport_cwd

# Register hooks in execution order
# precmd: runs before each prompt is displayed
# CRITICAL: _emit_command_finished must be FIRST to capture exit code
precmd_functions=(
  _emit_command_finished    # Must be first to capture exit code
  ${precmd_functions[@]}    # Preserve existing functions
)

# Add remaining precmd hooks (order doesn't matter for these)
precmd_functions+=(
  _emit_prompt_start        # Mark prompt beginning
  _emit_working_directory   # Update terminal's CWD
)

# preexec: runs after command is entered, before it executes
preexec_functions+=(
  _emit_command_start       # Mark command beginning
)
