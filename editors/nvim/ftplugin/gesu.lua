-- Settings for Gesù buffers, and the language server when `gesu` is installed.
vim.bo.commentstring = "-- %s"
vim.bo.comments = ":--"
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

local command = vim.g.gesu_command or "gesu"
if vim.fn.executable(command) == 1 then
  vim.lsp.start({
    name = "gesu",
    cmd = { command, "lsp" },
    root_dir = vim.fs.dirname(vim.api.nvim_buf_get_name(0)),
  })
end
