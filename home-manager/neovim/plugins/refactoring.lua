local refactoring = require('refactoring')

-- Configure refactoring.nvim for treesitter-based code extraction
refactoring.setup({
  -- Prompt for function name when extracting
  prompt_func_return_type = {
    go = true,
    java = true,
    cpp = true,
    c = true,
  },
  -- Prompt for function name
  prompt_func_param_type = {
    go = true,
    java = true,
    cpp = true,
    c = true,
  },
  -- Prompt for variable name when extracting
  printf_statements = {},
  print_var_statements = {},
  -- Show success notifications
  show_success_message = true,
})

-- CODE DOMAIN: Refactoring operations

-- Extract function (works in visual mode)
vim.keymap.set("x", "<leader>cre", function()
  refactoring.refactor('Extract Function')
end, { desc = "Code refactor extract function" })

-- Extract function to file (works in visual mode)
vim.keymap.set("x", "<leader>crf", function()
  refactoring.refactor('Extract Function To File')
end, { desc = "Code refactor extract to file" })

-- Extract variable (works in visual mode)
vim.keymap.set("x", "<leader>crv", function()
  refactoring.refactor('Extract Variable')
end, { desc = "Code refactor extract variable" })

-- Inline variable (works in normal and visual mode)
vim.keymap.set({ "n", "x" }, "<leader>cri", function()
  refactoring.refactor('Inline Variable')
end, { desc = "Code refactor inline variable" })

-- Extract block to if statement
vim.keymap.set("x", "<leader>crb", function()
  refactoring.refactor('Extract Block')
end, { desc = "Code refactor extract block" })

-- Extract block to if statement (from current line)
vim.keymap.set("n", "<leader>crB", function()
  refactoring.refactor('Extract Block To File')
end, { desc = "Code refactor extract Block to file" })


-- Register with which-key
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- Refactor group
    { '<leader>cre', desc = 'Code refactor extract function', mode = 'x' },
    { '<leader>crf', desc = 'Code refactor extract to file', mode = 'x' },
    { '<leader>crv', desc = 'Code refactor extract variable', mode = 'x' },
    { '<leader>cri', desc = 'Code refactor inline variable', mode = { 'n', 'x' } },
    { '<leader>crb', desc = 'Code refactor extract block', mode = 'x' },
    { '<leader>crB', desc = 'Code refactor extract Block to file', mode = 'n' },
  })
end