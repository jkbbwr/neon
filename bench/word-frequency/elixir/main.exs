# Word-frequency: string building, string hashing, and hash-map upserts using the
# native Map with string keys.
defmodule WordFreq do
  @n 10_000_000

  def run do
    counts = loop(@n, 42, %{})

    distinct = map_size(counts)
    max = counts |> Map.values() |> Enum.max()

    IO.puts("Result: #{distinct} #{@n} #{max}")
  end

  defp loop(0, _x, counts), do: counts

  defp loop(i, x, counts) do
    x = rem(x * 48271, 2_147_483_647)
    w = "w" <> Integer.to_string(rem(x, 10_000))
    counts = Map.update(counts, w, 1, &(&1 + 1))
    loop(i - 1, x, counts)
  end
end

WordFreq.run()
