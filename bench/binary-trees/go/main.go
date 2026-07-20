// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
// plain per-node allocation left to the GC — no pools or arenas.
package main

import "fmt"

type Node struct {
	left, right *Node
}

func make_(depth int) *Node {
	n := &Node{}
	if depth > 0 {
		n.left = make_(depth - 1)
		n.right = make_(depth - 1)
	}
	return n
}

func check(n *Node) int64 {
	if n == nil {
		return 0
	}
	return 1 + check(n.left) + check(n.right)
}

func main() {
	const maxDepth = 18
	var total int64 = 0

	stretch := make_(maxDepth + 1)
	sc := check(stretch)
	stretch = nil
	fmt.Printf("stretch tree of depth %d check: %d\n", maxDepth+1, sc)
	total += sc

	longLived := make_(maxDepth)

	for depth := 4; depth <= maxDepth; depth += 2 {
		iterations := int64(1) << uint(maxDepth-depth+4)
		var sum int64 = 0
		for i := int64(0); i < iterations; i++ {
			t := make_(depth)
			sum += check(t)
		}
		fmt.Printf("%d trees of depth %d check: %d\n", iterations, depth, sum)
		total += sum
	}

	ll := check(longLived)
	fmt.Printf("long lived tree of depth %d check: %d\n", maxDepth, ll)
	total += ll

	fmt.Printf("Result: %d\n", total)
}
