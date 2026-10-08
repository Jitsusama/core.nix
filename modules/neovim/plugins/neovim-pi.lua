require("neovim-pi").setup({
  -- Omit `listen` so the plugin defaults to a per-pid socket at
  -- ~/.local/state/pi/nvim-<pid>.sock. Each nvim gets its own;
  -- pi discovers them all and the user picks which to pair with.
  buffers = { enable = true },
  commands = { enable = true },
})
