// Word-frequency: string building, string hashing, and hash-map upserts using the
// standard std::unordered_map with std::string keys.
#include <cstdint>
#include <cstdio>
#include <string>
#include <unordered_map>

int main() {
    std::unordered_map<std::string, int64_t> counts;

    int64_t x = 42;
    const int64_t n = 10000000;
    char buf[32];

    for (int64_t i = 0; i < n; i++) {
        x = (x * 48271) % 2147483647;
        int len = std::snprintf(buf, sizeof buf, "w%lld", (long long)(x % 10000));
        counts[std::string(buf, len)]++;
    }

    int64_t max = 0;
    size_t distinct = 0;
    for (const auto &kv : counts) {
        distinct++;
        if (kv.second > max) max = kv.second;
    }

    std::printf("Result: %zu %lld %lld\n", distinct, (long long)n, (long long)max);
    return 0;
}
