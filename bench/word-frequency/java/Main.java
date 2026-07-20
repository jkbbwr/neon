// Word-frequency: string building, string hashing, and hash-map upserts using the
// native HashMap<String, Long>.
import java.util.HashMap;

public class Main {
    public static void main(String[] args) {
        HashMap<String, Long> counts = new HashMap<>();

        long x = 42;
        final long n = 10000000;

        for (long i = 0; i < n; i++) {
            x = (x * 48271) % 2147483647L;
            String w = "w" + (x % 10000);
            counts.merge(w, 1L, Long::sum);
        }

        long max = 0;
        long distinct = 0;
        for (long c : counts.values()) {
            distinct++;
            if (c > max) max = c;
        }

        System.out.println("Result: " + distinct + " " + n + " " + max);
    }
}
