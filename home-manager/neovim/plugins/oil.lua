local oil = require('oil')

-- Configure oil.nvim for file management as editable buffers
oil.setup({
  default_file_explorer = true,
  delete_to_trash = true,
  view_options = {
    show_hidden = true,
    is_always_hidden = function(name, _)
      return name == '..' or name == '.git'
    end,
  },
  -- Override conflicting keymaps to avoid conflicts with global mappings
  keymaps = {
    ["g?"] = { "actions.show_help", mode = "n" },
    ["<CR>"] = "actions.select",
    ["<C-s>"] = { "actions.select", opts = { vertical = true } },
    ["<C-h>"] = { "actions.select", opts = { horizontal = true } },
    ["<C-t>"] = { "actions.select", opts = { tab = true } },
    ["<C-p>"] = "actions.preview",
    ["<C-c>"] = { "actions.close", mode = "n" },
    ["<C-l>"] = "actions.refresh",
    ["-"] = { "actions.parent", mode = "n" },
    ["_"] = { "actions.open_cwd", mode = "n" },
    ["`"] = { "actions.cd", mode = "n" },
    ["~"] = { "actions.cd", opts = { scope = "tab" }, mode = "n" },
    -- Disable gs to avoid conflict with Flash navigation
    ["gs"] = false,
    -- Keep gx but document it
    ["gx"] = "actions.open_external",
    -- Remap toggle hidden to avoid g. conflict (use gh instead)
    ["gh"] = { "actions.toggle_hidden", mode = "n" },
    ["g."] = false,
    ["g\\"] = { "actions.toggle_trash", mode = "n" },
  },
})

-- NEW KEYBINDING SYSTEM

-- DIRECT ACCESS: Most common operation (parent directory)
vim.keymap.set('n', '-', '<CMD>Oil<CR>', { desc = 'Parent directory' })

-- FILES DOMAIN (<leader>f): File management operations
vim.keymap.set('n', '<leader>fe', '<CMD>Oil<CR>', { desc = 'Files explorer' })

-- SYSTEM CONTROLS (<C-*>): Quick file explorer access
vim.keymap.set('n', '<C-e>', '<CMD>Oil<CR>', { desc = 'Explorer' })

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '<leader>fe', desc = 'Files explorer' },
    { '<C-e>', desc = 'Explorer' },
    { '-', desc = 'Parent directory' },
  })
end