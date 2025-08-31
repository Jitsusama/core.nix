-- Load smart-open as a Telescope extension
local telescope = require('telescope')

-- Smart-open learns from your usage patterns automatically
telescope.load_extension('smart_open')

-- Configure smart-open settings
telescope.setup({
  extensions = {
    smart_open = {
      show_scores = false, -- Don't show the scoring numbers
      ignore_patterns = { "*.git/*", "*.svn/*", "node_modules/*", "vendor/*" },
      match_algorithm = "fzf", -- Use fzf-style matching
      disable_devicons = false, -- Show file icons
      open_buffer_indicators = { previous = "👀", others = "📝" }, -- Indicate already open buffers
      
      -- Prioritization settings
      cwd_only = false, -- Don't restrict to current working directory
      filename_first = true, -- Prioritize filename matches over path matches
      
      -- Custom mappings in the picker
      mappings = {
        i = {
          ["<C-w>"] = function(prompt_bufnr)
            -- Delete word in insert mode
            local action_state = require('telescope.actions.state')
            local current_line = action_state.get_current_line()
            local new_line = current_line:gsub("%w+%s*$", "")
            vim.api.nvim_put({ new_line }, "c", false, true)
          end,
        },
      },
    },
  },
})

-- FILES DOMAIN: Smart file finding
vim.keymap.set("n", "<leader>fo", function()
  require('telescope').extensions.smart_open.smart_open({
    cwd_only = false,
    filename_first = true,
  })
end, { desc = "Files open (smart)" })

-- Make it the default for <leader>ff (replacing basic find_files)
vim.keymap.set("n", "<leader>ff", function()
  require('telescope').extensions.smart_open.smart_open({
    cwd_only = true, -- For ff, restrict to project
    filename_first = true,
  })
end, { desc = "Find files (smart)" })


-- Register with which-key
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { '<leader>fo', desc = 'Files open (smart)' },
    { '<leader>ff', desc = 'Find files (smart)' },
  })
end