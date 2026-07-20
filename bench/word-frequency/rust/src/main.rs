// Word-frequency: string building, string hashing, and hash-map upserts using the
// standard std::collections::HashMap with String keys.
use std::collections::HashMap;

fn main() {
    let mut counts: HashMap<String, i64> = HashMap::new();

    let mut x: i64 = 42;
    let n: i64 = 10_000_000;

    for _ in 0..n {
        x = (x * 48271) % 2147483647;
        let word = format!("w{}", x % 10000);
        *counts.entry(word).or_insert(0) += 1;
    }

    let distinct = counts.len();
    let max = counts.values().copied().max().unwrap_or(0);

    println!("Result: {} {} {}", distinct, n, max);
}
