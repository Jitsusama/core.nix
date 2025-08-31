local Snacks = require('snacks')

-- Configure Snacks.nvim with essential utilities
Snacks.setup({
  -- Big file optimizations
  bigfile = { enabled = true },

  -- Quick file loading optimizations
  quickfile = { enabled = true },

  -- Dashboard configuration with custom ASCII art
  -- https://www.asciiart.eu/text-to-ascii-art
  dashboard = {
    enabled = true,
    preset = {
      header = [[
                                           ███                 
                                          ░░░                  
 ████████    ██████   ██████  █████ █████ ████  █████████████  
░░███░░███  ███░░███ ███░░███░░███ ░░███ ░░███ ░░███░░███░░███ 
 ░███ ░███ ░███████ ░███ ░███ ░███  ░███  ░███  ░███ ░███ ░███ 
 ░███ ░███ ░███░░░  ░███ ░███ ░░███ ███   ░███  ░███ ░███ ░███ 
 ████ █████░░██████ ░░██████   ░░█████    █████ █████░███ █████
░░░░ ░░░░░  ░░░░░░   ░░░░░░     ░░░░░    ░░░░░ ░░░░░ ░░░ ░░░░░ 

by

 ▄▄▄██▀▀▀██▓▄▄▄█████▓  ██████  █    ██   ██████  ▄▄▄       ███▄ ▄███▓ ▄▄▄      
   ▒██  ▓██▒▓  ██▒ ▓▒▒██    ▒  ██  ▓██▒▒██    ▒ ▒████▄    ▓██▒▀█▀ ██▒▒████▄    
   ░██  ▒██▒▒ ▓██░ ▒░░ ▓██▄   ▓██  ▒██░░ ▓██▄   ▒██  ▀█▄  ▓██    ▓██░▒██  ▀█▄  
▓██▄██▓ ░██░░ ▓██▓ ░   ▒   ██▒▓▓█  ░██░  ▒   ██▒░██▄▄▄▄██ ▒██    ▒██ ░██▄▄▄▄██ 
 ▓███▒  ░██░  ▒██▒ ░ ▒██████▒▒▒▒█████▓ ▒██████▒▒ ▓█   ▓██▒▒██▒   ░██▒ ▓█   ▓██▒
 ▒▓▒▒░  ░▓    ▒ ░░   ▒ ▒▓▒ ▒ ░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░ ▒▒   ▓▒█░░ ▒░   ░  ░ ▒▒   ▓▒█░
 ▒ ░▒░   ▒ ░    ░    ░ ░▒  ░ ░░░▒░ ░ ░ ░ ░▒  ░ ░  ▒   ▒▒ ░░  ░      ░  ▒   ▒▒ ░
 ░ ░ ░   ▒ ░  ░      ░  ░  ░   ░░░ ░ ░ ░  ░  ░    ░   ▒   ░      ░     ░   ▒   
 ░   ░   ░                 ░     ░           ░        ░  ░       ░         ░  ░
]],
    },
    sections = {
      { section = 'header' },
    },
  },

  -- Enhanced notifications (replacing nvim-notify)
  notifier = {
    enabled = true,
    timeout = 3000,
    width = { min = 40, max = 80 },
    height = { min = 1, max = 20 },
    top_down = true,
    icons = {
      error = '✖',
      warn = '⚠',
      info = 'ℹ',
      debug = '⚙',
      trace = '✎',
    },
    style = 'compact',
  },

  -- Scratch buffers for temporary work
  scratch = {
    win = {
      width = 0.6,
      height = 0.6,
      border = 'rounded',
    },
  },

  -- Terminal management
  terminal = {
    win = {
      position = 'float',
      border = 'rounded',
      width = 0.8,
      height = 0.8,
    },
  },

  -- Buffer delete without closing windows
  bufdelete = {
    enabled = true,
  },

  -- Git integration utilities
  gitbrowse = {
    enabled = true,
  },
})

-- Replace vim.notify with Snacks notifier
vim.notify = Snacks.notifier.notify

-- NEW KEYBINDING SYSTEM

-- BUFFER OPERATIONS (<leader>b)
vim.keymap.set('n', '<leader>bd', function()
  Snacks.bufdelete()
end, { desc = 'Buffer delete' })

vim.keymap.set('n', '<leader>bD', function()
  Snacks.bufdelete({ force = true })
end, { desc = 'Buffer Delete (force)' })

vim.keymap.set('n', '<leader>bo', function()
  Snacks.bufdelete.other()
end, { desc = 'Buffer only' })

vim.keymap.set('n', '<leader>bs', function()
  Snacks.scratch()
end, { desc = 'Buffer scratch' })

vim.keymap.set('n', '<leader>bS', function()
  Snacks.scratch.select()
end, { desc = 'Buffer Scratch (select)' })

vim.keymap.set('n', '<leader>bw', function()
  vim.cmd('write')
end, { desc = 'Buffer write' })

-- INTROSPECTION (<leader>i)
vim.keymap.set('n', '<leader>in', function()
  Snacks.notifier.show_history()
end, { desc = 'Introspect notifications' })

vim.keymap.set('n', '<leader>iN', function()
  Snacks.notifier.hide()
end, { desc = 'Introspect Notifications clear' })


-- SYSTEM CONTROLS (<C-*>)
vim.keymap.set('n', '<C-/>', function()
  Snacks.terminal()
end, { desc = 'Terminal' })

-- WINDOW OPERATIONS (<C-w> extensions)
vim.keymap.set('n', '<C-w>d', function()
  Snacks.bufdelete()
end, { desc = 'Delete buffer' })

vim.keymap.set('n', '<C-w>D', function()
  Snacks.bufdelete({ force = true })
end, { desc = 'Delete buffer!' })

-- DIRECT ACCESS KEYS
vim.keymap.set('n', '<BS>', function()
  vim.cmd('buffer #')
end, { desc = 'Alternate buffer' })

vim.keymap.set('n', 'ga', function()
  vim.cmd('buffer #')
end, { desc = 'Go to alternate buffer' })