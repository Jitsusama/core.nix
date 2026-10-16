-- Oil shows a folder as a file to edit, for Space f e and - (keys.nix), and
-- its keys inside are in keys.nix too.
local oil = require('oil')
local keys = require('keys')

oil.setup({
  default_file_explorer = true,
  delete_to_trash = true,
  view_options = {
    show_hidden = true,
    is_always_hidden = function(name, _)
      return name == '..' or name == '.git'
    end,
  },
  -- The file browser's keys, and none of oil's own.
  use_default_keymaps = false,
  keymaps = keys.window('files', function(key)
    return { key.plugin, opts = key.options, mode = key.modes, desc = key.desc }
  end),
})
