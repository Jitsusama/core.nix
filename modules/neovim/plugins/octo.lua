-- Octo edits GitHub's issues and pull requests in Neovim. Its keys, in its
-- own buffers as everywhere else, are keys.nix's, and none of its own.
local keys = require('keys')

-- A window's keys from keys.nix, by the octo action each runs.
local function mappings(scope)
  local made = {}
  for lhs, key in
    pairs(keys.window(scope, function(key)
      return key
    end))
  do
    made[key.plugin] = { lhs = lhs, desc = key.desc, mode = key.modes }
  end
  return made
end

require('octo').setup({
  -- Its lists open in fzf-lua, as every other list does.
  picker = 'fzf-lua',
  mappings_disable_default = true,
  mappings = {
    issue = mappings('octo-issue'),
    pull_request = mappings('octo-pull'),
    review_thread = mappings('octo-thread'),
    submit_win = mappings('octo-submit'),
    review_diff = mappings('octo-change'),
    file_panel = mappings('octo-files'),
    runs = mappings('octo-runs'),
    notification = mappings('octo-notices'),
  },
})
