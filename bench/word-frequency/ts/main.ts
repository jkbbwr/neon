// Word-frequency: string building, string hashing, and hash-map upserts using the
// native Map with string keys.
const counts: Map<string, number> = new Map();

let x: number = 42;
const n: number = 10000000;

for (let i = 0; i < n; i++) {
    x = (x * 48271) % 2147483647;
    const w: string = "w" + (x % 10000);
    counts.set(w, (counts.get(w) ?? 0) + 1);
}

let max: number = 0;
let distinct: number = 0;
for (const c of counts.values()) {
    distinct++;
    if (c > max) max = c;
}

console.log(`Result: ${distinct} ${n} ${max}`);
