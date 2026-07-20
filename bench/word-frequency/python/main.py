# Word-frequency: string building, string hashing, and hash-map upserts using the
# native dict with string keys.

def main():
    counts = {}
    get = counts.get

    x = 42
    n = 10000000

    for _ in range(n):
        x = (x * 48271) % 2147483647
        w = "w%d" % (x % 10000)
        counts[w] = get(w, 0) + 1

    distinct = len(counts)
    mx = max(counts.values())

    print("Result: %d %d %d" % (distinct, n, mx))


main()
