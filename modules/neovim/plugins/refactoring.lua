-- refactoring.nvim does Space c r's refactors (keys.nix), from Tree-sitter.
local refactoring = require('refactoring')

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
