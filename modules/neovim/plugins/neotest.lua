-- neotest runs the tests that Space t's keys (keys.nix) ask for.
local neotest = require('neotest')

neotest.setup({
  adapters = {
    require('neotest-minitest')({}),
  },
  quickfix = {
    open = true,
  },
})
