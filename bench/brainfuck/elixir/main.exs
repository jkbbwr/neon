defmodule Brainfuck do
  def parse(source) do
    chars = String.graphemes(source)
    {ops, _} = parse_body(chars)
    ops
  end

  defp parse_body(chars) do
    parse_acc(chars, [])
  end

  defp parse_acc([], acc), do: {Enum.reverse(acc), []}
  defp parse_acc([c | rest] = chars, acc) do
    case c do
      c when c in ["+", "-"] ->
        {val, remaining} = parse_rle(chars, 0, ["+", "-"])
        if val != 0 do
          parse_acc(remaining, [{:add, val} | acc])
        else
          parse_acc(remaining, acc)
        end
      c when c in [">", "<"] ->
        {val, remaining} = parse_rle(chars, 0, [">", "<"])
        if val != 0 do
          parse_acc(remaining, [{:move, val} | acc])
        else
          parse_acc(remaining, acc)
        end
      "." -> parse_acc(rest, [{:out} | acc])
      "," -> parse_acc(rest, [{:in} | acc])
      "[" ->
        {body, remaining} = parse_body(rest)
        parse_acc(remaining, [{:loop, body} | acc])
      "]" ->
        {Enum.reverse(acc), rest}
      _ ->
        parse_acc(rest, acc)
    end
  end

  defp parse_rle([], val, _chars), do: {val, []}
  defp parse_rle([c | rest] = current, val, allowed) do
    if c in allowed do
      inc = if c in ["+", ">"], do: 1, else: -1
      parse_rle(rest, val + inc, allowed)
    else
      {val, current}
    end
  end

  # :atomics is 1-indexed, so we offset ptr by 1
  def execute(ops, tape, ptr) do
    Enum.reduce(ops, ptr, fn op, p ->
      case op do
        {:add, val} ->
          :atomics.add(tape, p + 1, val)
          p
        {:move, val} ->
          p + val
        {:out} ->
          IO.write(to_string(:atomics.get(tape, p + 1)))
          p
        {:in} ->
          p
        {:loop, body} ->
          loop_execute(body, tape, p)
      end
    end)
  end

  defp loop_execute(body, tape, ptr) do
    if :atomics.get(tape, ptr + 1) != 0 do
      p2 = execute(body, tape, ptr)
      loop_execute(body, tape, p2)
    else
      ptr
    end
  end

  def main do
    program = "++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>+<-]<-]<-]<-]<-]<-]<-]<-]"
    ops = parse(program)
    tape = :atomics.new(30000, signed: true)
    _ptr = execute(ops, tape, 0)
    val = :atomics.get(tape, 8 + 1)
    IO.puts("Result: #{val}")
  end
end

Brainfuck.main()
