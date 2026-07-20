// Word-frequency: string building, string hashing, and hash-map upserts using the
// native Dictionary<string, long>.
var counts = new Dictionary<string, long>();

long x = 42;
const long n = 10000000;

for (long i = 0; i < n; i++)
{
    x = (x * 48271) % 2147483647;
    string w = "w" + (x % 10000);
    counts.TryGetValue(w, out long c);
    counts[w] = c + 1;
}

long max = 0;
long distinct = 0;
foreach (long c in counts.Values)
{
    distinct++;
    if (c > max) max = c;
}

Console.WriteLine($"Result: {distinct} {n} {max}");
