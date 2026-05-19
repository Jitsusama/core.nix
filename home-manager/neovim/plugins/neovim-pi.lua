require("neovim-pi").setup({
  -- Socket pi looks for first when XDG_RUNTIME_DIR is unset (macOS).
  listen = vim.fn.expand("~/.local/state/pi/neovim-pi.sock"),
  -- Register the `pi://` BufReadCmd so pi can open buffers in this nvim.
  buffers = { enable = true },
  -- Register `:PiStatus` and `:PiDetach` for quick inspection.
  commands = { enable = true },
})
