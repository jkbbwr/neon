# Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
# plain immutable per-node tuples left to the GC — no pools or arenas.

defmodule BinaryTrees do
  def make(0), do: {nil, nil}
  def make(depth), do: {make(depth - 1), make(depth - 1)}

  def check(nil), do: 0
  def check({left, right}), do: 1 + check(left) + check(right)

  def run do
    max_depth = 18

    stretch = make(max_depth + 1)
    sc = check(stretch)
    IO.puts("stretch tree of depth #{max_depth + 1} check: #{sc}")
    total = sc

    long_lived = make(max_depth)

    total =
      Enum.reduce(Enum.take_every(4..max_depth, 2), total, fn depth, acc ->
        iterations = Bitwise.bsl(1, max_depth - depth + 4)

        sum =
          Enum.reduce(1..iterations, 0, fn _, s ->
            t = make(depth)
            s + check(t)
          end)

        IO.puts("#{iterations} trees of depth #{depth} check: #{sum}")
        acc + sum
      end)

    ll = check(long_lived)
    IO.puts("long lived tree of depth #{max_depth} check: #{ll}")
    total = total + ll

    IO.puts("Result: #{total}")
  end
end

BinaryTrees.run()
