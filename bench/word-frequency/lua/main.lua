-- Word-frequency: string building, string hashing, and hash-map upserts using the
-- native table with string keys. Works under both lua and luajit.
local counts = {}

local x = 42
local n = 10000000

for _ = 1, n do
    x = (x * 48271) % 2147483647
    local w = string.format("w%d", x % 10000)
    counts[w] = (counts[w] or 0) + 1
end

local max = 0
local distinct = 0
for _, c in pairs(counts) do
    distinct = distinct + 1
    if c > max then max = c end
end

print(string.format("Result: %d %d %d", distinct, n, max))
