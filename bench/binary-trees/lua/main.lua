-- Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
-- plain per-node table allocation left to the GC — no pools or arenas.
-- Works under both lua and luajit.

local function make(depth)
  if depth == 0 then
    return {}
  end
  return { left = make(depth - 1), right = make(depth - 1) }
end

local function check(n)
  if n == nil then
    return 0
  end
  return 1 + check(n.left) + check(n.right)
end

local max_depth = 18
local total = 0

local stretch = make(max_depth + 1)
local sc = check(stretch)
stretch = nil
print(string.format("stretch tree of depth %d check: %d", max_depth + 1, sc))
total = total + sc

local long_lived = make(max_depth)

for depth = 4, max_depth, 2 do
  local iterations = 2 ^ (max_depth - depth + 4)
  local sum = 0
  for _ = 1, iterations do
    local t = make(depth)
    sum = sum + check(t)
  end
  print(string.format("%d trees of depth %d check: %d", iterations, depth, sum))
  total = total + sum
end

local ll = check(long_lived)
long_lived = nil
print(string.format("long lived tree of depth %d check: %d", max_depth, ll))
total = total + ll

print(string.format("Result: %d", total))
