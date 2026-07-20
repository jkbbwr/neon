// Word-frequency: string building, string hashing, and hash-map upserts using the
// native map[string]int64.
package main

import (
	"fmt"
	"strconv"
)

func main() {
	counts := make(map[string]int64)

	var x int64 = 42
	const n int64 = 10000000

	for i := int64(0); i < n; i++ {
		x = (x * 48271) % 2147483647
		w := "w" + strconv.FormatInt(x%10000, 10)
		counts[w]++
	}

	var max int64 = 0
	distinct := 0
	for _, c := range counts {
		distinct++
		if c > max {
			max = c
		}
	}

	fmt.Printf("Result: %d %d %d\n", distinct, n, max)
}
