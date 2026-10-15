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
