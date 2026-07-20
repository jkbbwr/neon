# Word-frequency: string building, string hashing, and hash-map upserts using the
# native Hash with string keys.
counts = Hash.new(0)

x = 42
n = 10_000_000

n.times do
  x = (x * 48271) % 2147483647
  w = "w#{x % 10000}"
  counts[w] += 1
end

distinct = counts.size
max = 0
counts.each_value { |c| max = c if c > max }

puts "Result: #{distinct} #{n} #{max}"
