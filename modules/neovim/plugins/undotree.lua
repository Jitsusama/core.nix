-- Undotree configuration
-- Visualizes the undo history and makes it easy to browse and switch between different undo branches

-- Set configuration variables before plugin loads
-- Focus the undotree window when toggling (more intuitive)
vim.g.undotree_SetFocusWhenToggle = 1

-- Keep default window layout (tree on left, diff below)
-- vim.g.undotree_WindowLayout = 1

-- Set up the keybinding
vim.keymap.set('n', '<leader>U', vim.cmd.UndotreeToggle, {
  desc = 'Undo tree',
  silent = true
})

-- Register the icon with which-key separately
local ok, which_key = pcall(require, 'which-key')
if ok then
  which_key.add({
    { "<leader>U", icon = "🌳" }
  })
end

-- Optional: Enable persistent undo if not already set
-- This allows undo history to persist between sessions
if vim.o.undofile == false then
  vim.o.undofile = true
  -- Set a reasonable undo directory (Neovim default is usually fine)
  -- vim.o.undodir = vim.fn.expand('~/.local/share/nvim/undo')
end

-- Set reasonable undo levels (how many changes to remember)
if vim.o.undolevels < 1000 then
  vim.o.undolevels = 1000
end