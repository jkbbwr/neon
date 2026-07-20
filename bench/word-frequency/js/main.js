// Word-frequency: string building, string hashing, and hash-map upserts using the
// native Map with string keys.
"use strict";

const counts = new Map();

let x = 42;
const n = 10000000;

for (let i = 0; i < n; i++) {
    x = (x * 48271) % 2147483647;
    const w = "w" + (x % 10000);
    counts.set(w, (counts.get(w) || 0) + 1);
}

let max = 0;
let distinct = 0;
for (const c of counts.values()) {
    distinct++;
    if (c > max) max = c;
}

console.log(`Result: ${distinct} ${n} ${max}`);
