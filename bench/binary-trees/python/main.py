# Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
# plain per-node class-instance allocation left to the GC — no pools or arenas.


class Node:
    __slots__ = ("left", "right")

    def __init__(self, left=None, right=None):
        self.left = left
        self.right = right


def make(depth):
    if depth == 0:
        return Node()
    return Node(make(depth - 1), make(depth - 1))


def check(n):
    if n is None:
        return 0
    return 1 + check(n.left) + check(n.right)


def main():
    max_depth = 18
    total = 0

    stretch = make(max_depth + 1)
    sc = check(stretch)
    del stretch
    print(f"stretch tree of depth {max_depth + 1} check: {sc}")
    total += sc

    long_lived = make(max_depth)

    for depth in range(4, max_depth + 1, 2):
        iterations = 1 << (max_depth - depth + 4)
        sum_ = 0
        for _ in range(iterations):
            t = make(depth)
            sum_ += check(t)
        print(f"{iterations} trees of depth {depth} check: {sum_}")
        total += sum_

    ll = check(long_lived)
    print(f"long lived tree of depth {max_depth} check: {ll}")
    total += ll

    print(f"Result: {total}")


if __name__ == "__main__":
    main()
