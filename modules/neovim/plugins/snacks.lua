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
    -- A notice is its mark and then its words, on the floats' shade with a
    -- step of it all round and a line of air between two; the mark in its
    -- level's colour, and no box round it.
    padding = false,
    gap = 1,
    icons = {
      error = require('theme').marks.failed,
      warn = require('theme').marks.caution,
      info = require('theme').marks.note,
      debug = require('theme').marks.suggestion,
      trace = require('theme').marks.suggestion,
    },
    style = function(buf, notif, ctx)
      ctx.opts.border = 'solid'
      ctx.opts.wo.winhighlight = 'Normal:NormalFloat,FloatBorder:FloatBorder'
      local mark = notif.icon .. ' '
      local hanging = string.rep(' ', vim.fn.strdisplaywidth(mark))
      local lines = vim.split(notif.msg, '\n')
      for index, line in ipairs(lines) do
        lines[index] = (index == 1 and mark or hanging) .. line
      end
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
      vim.api.nvim_buf_set_extmark(
        buf,
        ctx.ns,
        0,
        0,
        { end_col = #notif.icon, hl_group = ctx.hl.icon }
      )
    end,
  },

  -- Scratch buffers for temporary work
  scratch = {
    win = {
      width = 0.6,
      height = 0.6,
      border = 'solid',
    },
  },

  -- Terminal management
  terminal = {
    win = {
      position = 'float',
      border = 'solid',
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

  -- Pictures drawn in the terminal through kitty's graphics protocol: a
  -- picture file opened shows itself, and a markdown file's pictures show
  -- under the lines that name them, a file's own path resolved beside it.
  -- Formulas would need a TeX the account doesn't carry, so they stay text.
  image = {
    enabled = true,
    math = { enabled = false },
    icons = { image = require('theme').marks.picture .. ' ' },
  },
})

-- Notices go to Snacks' notifier, which loads with the first one.
vim.notify = function(...)
  return Snacks.notifier.notify(...)
end
