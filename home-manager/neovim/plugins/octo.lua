-- GitHub Integration Plugin Configuration
-- Comprehensive keybinding setup following our design patterns

require('octo').setup({
  mappings = {
    issue = {
      -- Buffer operations
      close_issue = { lhs = '<leader>ghbc', desc = 'Buffer close (close issue)' },
      reopen_issue = { lhs = '<leader>ghbR', desc = 'Buffer Reopen (reopen issue)' },
      reload = { lhs = '<leader>ghbr', desc = 'Buffer reload' },
      open_in_browser = { lhs = '<leader>ghbo', desc = 'Buffer open browser' },
      copy_url = { lhs = '<leader>ghby', desc = 'Buffer yank url' },

      -- Action operations
      add_assignee = { lhs = '<leader>ghaa', desc = 'Action assignee add' },
      remove_assignee = { lhs = '<leader>ghad', desc = 'Action assignee delete' },

      -- Label operations
      create_label = { lhs = '<leader>ghlc', desc = 'Label create' },
      add_label = { lhs = '<leader>ghla', desc = 'Label add' },
      remove_label = { lhs = '<leader>ghld', desc = 'Label delete' },

      -- Comment operations
      add_comment = { lhs = '<leader>ghca', desc = 'Comment add' },
      add_reply = { lhs = '<leader>ghcr', desc = 'Comment reply' },
      delete_comment = { lhs = '<leader>ghcd', desc = 'Comment delete' },

      -- Navigation
      next_comment = { lhs = ']c', desc = 'go to next comment' },
      prev_comment = { lhs = '[c', desc = 'go to previous comment' },

      -- Reactions (using new ght* prefix)
      react_hooray = { lhs = '<leader>ghtp', desc = 'Party 🎉 reaction' },
      react_heart = { lhs = '<leader>ghth', desc = 'Heart ❤️ reaction' },
      react_eyes = { lhs = '<leader>ghte', desc = 'Eyes 👀 reaction' },
      react_thumbs_up = { lhs = '<leader>ght+', desc = 'Thumbs up 👍 reaction' },
      react_thumbs_down = { lhs = '<leader>ght-', desc = 'Thumbs down 👎 reaction' },
      react_rocket = { lhs = '<leader>ghtr', desc = 'Rocket 🚀 reaction' },
      react_laugh = { lhs = '<leader>ghtl', desc = 'Laugh 😄 reaction' },
      react_confused = { lhs = '<leader>ghtc', desc = 'Confused 😕 reaction' },
    },
    pull_request = {
      -- Buffer operations
      close_issue = { lhs = '<leader>ghbc', desc = 'Buffer close (close PR)' },
      reopen_issue = { lhs = '<leader>ghbR', desc = 'Buffer Reopen (reopen PR)' },
      reload = { lhs = '<leader>ghbr', desc = 'Buffer reload' },
      open_in_browser = { lhs = '<leader>ghbo', desc = 'Buffer open browser' },
      copy_url = { lhs = '<leader>ghby', desc = 'Buffer yank url' },

      -- Merge operations
      checkout_pr = { lhs = '<leader>ghmo', desc = 'Merge checkout (checkout PR)' },
      merge_pr = { lhs = '<leader>ghmm', desc = 'Merge commit' },
      squash_and_merge_pr = { lhs = '<leader>ghms', desc = 'Merge squash' },
      rebase_and_merge_pr = { lhs = '<leader>ghmr', desc = 'Merge rebase' },

      -- Review operations
      add_reviewer = { lhs = '<leader>ghaR', desc = 'Action Reviewer add' },
      remove_reviewer = { lhs = '<leader>ghar', desc = 'Action reviewer remove' },
      list_commits = { lhs = '<leader>ghvc', desc = 'reView commits' },
      list_changed_files = { lhs = '<leader>ghvf', desc = 'reView files' },
      show_pr_diff = { lhs = '<leader>ghvD', desc = 'reView Diff' },
      review_start = { lhs = '<leader>ghvs', desc = 'reView start' },
      review_resume = { lhs = '<leader>ghvr', desc = 'reView resume' },

      -- Action operations
      add_assignee = { lhs = '<leader>ghaa', desc = 'Action assignee add' },
      remove_assignee = { lhs = '<leader>ghad', desc = 'Action assignee delete' },

      -- Label operations
      create_label = { lhs = '<leader>ghlc', desc = 'Label create' },
      add_label = { lhs = '<leader>ghla', desc = 'Label add' },
      remove_label = { lhs = '<leader>ghld', desc = 'Label delete' },

      -- Comment operations
      add_comment = { lhs = '<leader>ghca', desc = 'Comment add' },
      add_reply = { lhs = '<leader>ghcr', desc = 'Comment reply' },
      delete_comment = { lhs = '<leader>ghcd', desc = 'Comment delete' },

      -- Navigation
      next_comment = { lhs = ']c', desc = 'go to next comment' },
      prev_comment = { lhs = '[c', desc = 'go to previous comment' },
      goto_file = { lhs = 'gf', desc = 'go to file' },

      -- Thread operations
      resolve_thread = { lhs = '<leader>ghTr', desc = 'Thread resolve' },
      unresolve_thread = { lhs = '<leader>ghTu', desc = 'Thread unresolve' },

      -- Reactions (using new ght* prefix)
      react_hooray = { lhs = '<leader>ghtp', desc = 'Party 🎉 reaction' },
      react_heart = { lhs = '<leader>ghth', desc = 'Heart ❤️ reaction' },
      react_eyes = { lhs = '<leader>ghte', desc = 'Eyes 👀 reaction' },
      react_thumbs_up = { lhs = '<leader>ght+', desc = 'Thumbs up 👍 reaction' },
      react_thumbs_down = { lhs = '<leader>ght-', desc = 'Thumbs down 👎 reaction' },
      react_rocket = { lhs = '<leader>ghtr', desc = 'Rocket 🚀 reaction' },
      react_laugh = { lhs = '<leader>ghtl', desc = 'Laugh 😄 reaction' },
      react_confused = { lhs = '<leader>ghtc', desc = 'Confused 😕 reaction' },
    },
    review_thread = {
      -- Comment operations
      add_comment = { lhs = '<leader>ghca', desc = 'Comment add' },
      add_reply = { lhs = '<leader>ghcr', desc = 'Comment reply' },
      add_suggestion = { lhs = '<leader>ghcs', desc = 'Comment suggest' },
      delete_comment = { lhs = '<leader>ghcd', desc = 'Comment delete' },

      -- Navigation
      next_comment = { lhs = ']c', desc = 'go to next comment' },
      prev_comment = { lhs = '[c', desc = 'go to previous comment' },
      select_next_entry = { lhs = ']q', desc = 'move to next changed file' },
      select_prev_entry = { lhs = '[q', desc = 'move to previous changed file' },
      select_first_entry = { lhs = '[Q', desc = 'move to first changed file' },
      select_last_entry = { lhs = ']Q', desc = 'move to last changed file' },
      close_review_tab = { lhs = '<C-c>', desc = 'close review tab' },

      -- Thread operations
      resolve_thread = { lhs = '<leader>ghTr', desc = 'Thread resolve' },
      unresolve_thread = { lhs = '<leader>ghTu', desc = 'Thread unresolve' },

      -- Reactions (using new ght* prefix)
      react_hooray = { lhs = '<leader>ghtp', desc = 'Party 🎉 reaction' },
      react_heart = { lhs = '<leader>ghth', desc = 'Heart ❤️ reaction' },
      react_eyes = { lhs = '<leader>ghte', desc = 'Eyes 👀 reaction' },
      react_thumbs_up = { lhs = '<leader>ght+', desc = 'Thumbs up 👍 reaction' },
      react_thumbs_down = { lhs = '<leader>ght-', desc = 'Thumbs down 👎 reaction' },
      react_rocket = { lhs = '<leader>ghtr', desc = 'Rocket 🚀 reaction' },
      react_laugh = { lhs = '<leader>ghtl', desc = 'Laugh 😄 reaction' },
      react_confused = { lhs = '<leader>ghtc', desc = 'Confused 😕 reaction' },
    },
    submit_win = {
      approve_review = { lhs = '<C-a>', desc = 'approve review', mode = { 'n', 'i' } },
      comment_review = { lhs = '<C-m>', desc = 'comment review', mode = { 'n', 'i' } },
      request_changes = { lhs = '<C-r>', desc = 'request changes review', mode = { 'n', 'i' } },
      close_review_tab = { lhs = '<C-c>', desc = 'close review tab', mode = { 'n', 'i' } },
    },
    review_diff = {
      -- Review operations
      submit_review = { lhs = '<leader>ghvS', desc = 'reView Submit' },
      discard_review = { lhs = '<leader>ghvd', desc = 'reView discard' },

      -- Comment operations
      add_review_comment = { lhs = '<leader>ghca', desc = 'Comment add', mode = { 'n', 'x' } },
      add_review_suggestion = { lhs = '<leader>ghcs', desc = 'Comment suggest', mode = { 'n', 'x' } },

      -- Navigation
      focus_files = { lhs = '<leader>ghve', desc = 'reView focus files' },
      toggle_files = { lhs = '<leader>ghvb', desc = 'reView toggle files' },
      next_thread = { lhs = ']t', desc = 'move to next thread' },
      prev_thread = { lhs = '[t', desc = 'move to previous thread' },
      select_next_entry = { lhs = ']q', desc = 'move to next changed file' },
      select_prev_entry = { lhs = '[q', desc = 'move to previous changed file' },
      select_first_entry = { lhs = '[Q', desc = 'move to first changed file' },
      select_last_entry = { lhs = ']Q', desc = 'move to last changed file' },
      close_review_tab = { lhs = '<C-c>', desc = 'close review tab' },
      toggle_viewed = { lhs = '<leader>ghvt', desc = 'reView toggle viewed' },
      goto_file = { lhs = 'gf', desc = 'go to file' },
    },
    file_panel = {
      -- Review operations
      submit_review = { lhs = '<leader>ghvS', desc = 'reView Submit' },
      discard_review = { lhs = '<leader>ghvd', desc = 'reView discard' },

      -- Panel navigation
      next_entry = { lhs = 'j', desc = 'move to next changed file' },
      prev_entry = { lhs = 'k', desc = 'move to previous changed file' },
      select_entry = { lhs = '<cr>', desc = 'show selected changed file diffs' },
      refresh_files = { lhs = 'R', desc = 'refresh changed files panel' },
      focus_files = { lhs = '<leader>ghve', desc = 'reView focus files' },
      toggle_files = { lhs = '<leader>ghvb', desc = 'reView toggle files' },
      select_next_entry = { lhs = ']q', desc = 'move to next changed file' },
      select_prev_entry = { lhs = '[q', desc = 'move to previous changed file' },
      select_first_entry = { lhs = '[Q', desc = 'move to first changed file' },
      select_last_entry = { lhs = ']Q', desc = 'move to last changed file' },
      close_review_tab = { lhs = '<C-c>', desc = 'close review tab' },
      toggle_viewed = { lhs = '<leader>ghvt', desc = 'reView toggle viewed' },
    },
    notification = {
      read = { lhs = '<leader>ghnr', desc = 'mark notification as read' },
      done = { lhs = '<leader>ghnd', desc = 'mark notification as done' },
      unsubscribe = { lhs = '<leader>ghnu', desc = 'unsubscribe from notifications' },
    },
  },
})

-- Global GitHub operations (available everywhere)

-- GitHub home (quick access)
vim.keymap.set('n', '<leader>ghh', ':Octo pr list<CR>', { desc = 'GitHub home (PR list)' })

-- Search operations
vim.keymap.set('n', '<leader>ghs', ':Octo search<CR>', { desc = 'GitHub search (current repo)' })
vim.keymap.set('n', '<leader>ghS', ':Octo search<CR>', { desc = 'GitHub Search (global)' })

-- Issues (<leader>ghi)
vim.keymap.set('n', '<leader>ghii', ':Octo issue list<CR>', { desc = 'Issue picker' })
vim.keymap.set('n', '<leader>ghin', ':Octo issue create<CR>', { desc = 'Issue new' })
vim.keymap.set('n', '<leader>ghis', ':Octo issue search<CR>', { desc = 'Issue search' })

-- Pull Requests (<leader>ghp)
vim.keymap.set('n', '<leader>ghpp', ':Octo pr list<CR>', { desc = 'PR picker' })
vim.keymap.set('n', '<leader>ghpn', ':Octo pr create<CR>', { desc = 'PR new' })
vim.keymap.set('n', '<leader>ghpc', ':Octo pr list author:@me<CR>', { desc = 'PR current (your PRs)' })
vim.keymap.set('n', '<leader>ghps', ':Octo pr search<CR>', { desc = 'PR search' })
vim.keymap.set('n', '<leader>ghpC', ':Octo pr checks<CR>', { desc = 'PR Checks' })

-- Repository (<leader>ghr)
vim.keymap.set('n', '<leader>ghrr', ':Octo repo list<CR>', { desc = 'Repo picker' })
vim.keymap.set('n', '<leader>ghrf', ':Octo repo fork<CR>', { desc = 'Repo fork' })
vim.keymap.set('n', '<leader>ghro', ':Octo repo browser<CR>', { desc = 'Repo open browser' })

-- Gists (<leader>ghg)
vim.keymap.set('n', '<leader>ghgg', ':Octo gist list<CR>', { desc = 'Gist picker' })
-- Note: ghgn (gist new) is handled by rhubarb.lua to support both normal and visual modes

-- Notifications (<leader>ghn)
vim.keymap.set('n', '<leader>ghnn', ':Octo notification list<CR>', { desc = 'Notification picker' })

-- Register specific keymaps with which-key
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    -- Global GitHub operation descriptions
    { '<leader>ghh', desc = 'GitHub home (PR list)', icon = '🏠' },
    { '<leader>ghs', desc = 'GitHub search (current repo)', icon = '🔍' },
    { '<leader>ghS', desc = 'GitHub Search (global)', icon = '🌍' },

    -- Issue operations
    { '<leader>ghii', desc = 'Issue picker' },
    { '<leader>ghin', desc = 'Issue new' },
    { '<leader>ghis', desc = 'Issue search' },

    -- Pull Request operations
    { '<leader>ghpp', desc = 'PR picker' },
    { '<leader>ghpn', desc = 'PR new' },
    { '<leader>ghpc', desc = 'PR current' },
    { '<leader>ghps', desc = 'PR search' },
    { '<leader>ghpC', desc = 'PR Checks' },

    -- Repository operations
    { '<leader>ghrr', desc = 'Repo picker' },
    { '<leader>ghrf', desc = 'Repo fork' },
    { '<leader>ghro', desc = 'Repo open browser' },

    -- Gist operations
    { '<leader>ghgg', desc = 'Gist picker' },

    -- Notification operations
    { '<leader>ghnn', desc = 'Notification picker' },
  })
end

