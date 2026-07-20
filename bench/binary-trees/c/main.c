// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded, plain
// malloc/free — the baseline is the allocator, not a pool trick.
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

typedef struct Node {
    struct Node *left, *right;
} Node;

static Node *make(int depth) {
    Node *n = malloc(sizeof(Node));
    if (depth == 0) {
        n->left = n->right = NULL;
    } else {
        n->left = make(depth - 1);
        n->right = make(depth - 1);
    }
    return n;
}

static int64_t check(Node *n) {
    if (!n) return 0;
    int64_t c = 1 + check(n->left) + check(n->right);
    return c;
}

static void drop(Node *n) {
    if (!n) return;
    drop(n->left);
    drop(n->right);
    free(n);
}

int main(void) {
    const int max_depth = 18;
    int64_t total = 0;

    Node *stretch = make(max_depth + 1);
    int64_t sc = check(stretch);
    drop(stretch);
    printf("stretch tree of depth %d check: %lld\n", max_depth + 1, (long long)sc);
    total += sc;

    Node *long_lived = make(max_depth);

    for (int depth = 4; depth <= max_depth; depth += 2) {
        int64_t iterations = 1LL << (max_depth - depth + 4);
        int64_t sum = 0;
        for (int64_t i = 0; i < iterations; i++) {
            Node *t = make(depth);
            sum += check(t);
            drop(t);
        }
        printf("%lld trees of depth %d check: %lld\n", (long long)iterations, depth,
               (long long)sum);
        total += sum;
    }

    int64_t ll = check(long_lived);
    drop(long_lived);
    printf("long lived tree of depth %d check: %lld\n", max_depth, (long long)ll);
    total += ll;

    printf("Result: %lld\n", (long long)total);
    return 0;
}
