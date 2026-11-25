local stock = { loaves = 5, fishes = 2 }
local M = {}

function M.count(item) return stock[item] end

function M.multiply(item, by)
  stock[item] = stock[item] * by
  return stock[item]
end

function M.baskets() return { "barley", "wheat", "figs" } end

return M
