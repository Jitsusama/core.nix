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

# Emit OSC 7 (current working directory for terminal)
function _emit_working_directory() {
  printf '\033]7;file://%s%s\007' "$HOST" "$PWD"
}

# Register hooks in execution order
# precmd: runs before each prompt is displayed
precmd_functions+=(
  _emit_command_finished    # Must be first to capture exit code
  _emit_prompt_start        # Mark prompt beginning
  _emit_working_directory   # Update terminal's CWD
)

# preexec: runs after command is entered, before it executes
preexec_functions+=(
  _emit_command_start       # Mark command beginning
)
