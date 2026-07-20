// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
// plain per-node object allocation left to the GC — no pools or arenas.

class Node
{
    public Node? Left;
    public Node? Right;
}

static class BinaryTrees
{
    static Node Make(int depth)
    {
        var n = new Node();
        if (depth > 0)
        {
            n.Left = Make(depth - 1);
            n.Right = Make(depth - 1);
        }
        return n;
    }

    static long Check(Node? n)
    {
        if (n == null) return 0;
        return 1 + Check(n.Left) + Check(n.Right);
    }

    static void Main()
    {
        const int maxDepth = 18;
        long total = 0;

        var stretch = Make(maxDepth + 1);
        long sc = Check(stretch);
        stretch = null;
        Console.WriteLine($"stretch tree of depth {maxDepth + 1} check: {sc}");
        total += sc;

        var longLived = Make(maxDepth);

        for (int depth = 4; depth <= maxDepth; depth += 2)
        {
            long iterations = 1L << (maxDepth - depth + 4);
            long sum = 0;
            for (long i = 0; i < iterations; i++)
            {
                var t = Make(depth);
                sum += Check(t);
            }
            Console.WriteLine($"{iterations} trees of depth {depth} check: {sum}");
            total += sum;
        }

        long ll = Check(longLived);
        Console.WriteLine($"long lived tree of depth {maxDepth} check: {ll}");
        total += ll;

        Console.WriteLine($"Result: {total}");
    }
}
