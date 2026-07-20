// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
// plain per-node object allocation left to the GC — no pools or arenas.

interface Node_ {
  left: Node_ | null;
  right: Node_ | null;
}

function make(depth: number): Node_ {
  if (depth === 0) {
    return { left: null, right: null };
  }
  return { left: make(depth - 1), right: make(depth - 1) };
}

function check(n: Node_ | null): number {
  if (n === null) return 0;
  return 1 + check(n.left) + check(n.right);
}

function main(): void {
  const maxDepth = 18;
  let total = 0;

  let stretch: Node_ | null = make(maxDepth + 1);
  const sc = check(stretch);
  stretch = null;
  console.log(`stretch tree of depth ${maxDepth + 1} check: ${sc}`);
  total += sc;

  const longLived = make(maxDepth);

  for (let depth = 4; depth <= maxDepth; depth += 2) {
    const iterations = 2 ** (maxDepth - depth + 4);
    let sum = 0;
    for (let i = 0; i < iterations; i++) {
      const t = make(depth);
      sum += check(t);
    }
    console.log(`${iterations} trees of depth ${depth} check: ${sum}`);
    total += sum;
  }

  const ll = check(longLived);
  console.log(`long lived tree of depth ${maxDepth} check: ${ll}`);
  total += ll;

  console.log(`Result: ${total}`);
}

main();
