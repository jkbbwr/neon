# Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
# plain per-node object allocation left to the GC — no pools or arenas.

class Node
  attr_accessor :left, :right

  def initialize(left = nil, right = nil)
    @left = left
    @right = right
  end
end

def make(depth)
  return Node.new if depth == 0
  Node.new(make(depth - 1), make(depth - 1))
end

def check(n)
  return 0 if n.nil?
  1 + check(n.left) + check(n.right)
end

max_depth = 18
total = 0

stretch = make(max_depth + 1)
sc = check(stretch)
stretch = nil
puts "stretch tree of depth #{max_depth + 1} check: #{sc}"
total += sc

long_lived = make(max_depth)

4.step(max_depth, 2) do |depth|
  iterations = 1 << (max_depth - depth + 4)
  sum = 0
  iterations.times do
    t = make(depth)
    sum += check(t)
  end
  puts "#{iterations} trees of depth #{depth} check: #{sum}"
  total += sum
end

ll = check(long_lived)
puts "long lived tree of depth #{max_depth} check: #{ll}"
total += ll

puts "Result: #{total}"
