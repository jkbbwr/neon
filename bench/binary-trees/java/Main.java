// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
// plain per-node object allocation left to the GC — no pools or arenas.

public class Main {
    static final class Node {
        Node left;
        Node right;
    }

    static Node make(int depth) {
        Node n = new Node();
        if (depth > 0) {
            n.left = make(depth - 1);
            n.right = make(depth - 1);
        }
        return n;
    }

    static long check(Node n) {
        if (n == null) return 0;
        return 1 + check(n.left) + check(n.right);
    }

    public static void main(String[] args) {
        final int maxDepth = 18;
        long total = 0;

        Node stretch = make(maxDepth + 1);
        long sc = check(stretch);
        stretch = null;
        System.out.println("stretch tree of depth " + (maxDepth + 1) + " check: " + sc);
        total += sc;

        Node longLived = make(maxDepth);

        for (int depth = 4; depth <= maxDepth; depth += 2) {
            long iterations = 1L << (maxDepth - depth + 4);
            long sum = 0;
            for (long i = 0; i < iterations; i++) {
                Node t = make(depth);
                sum += check(t);
            }
            System.out.println(iterations + " trees of depth " + depth + " check: " + sum);
            total += sum;
        }

        long ll = check(longLived);
        System.out.println("long lived tree of depth " + maxDepth + " check: " + ll);
        total += ll;

        System.out.println("Result: " + total);
    }
}
