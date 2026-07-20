# Word-frequency: string building, string hashing, and hash-map upserts using the
# native hash with string keys.
use strict;
use warnings;

my %counts;

my $x = 42;
my $n = 10000000;

for (my $i = 0; $i < $n; $i++) {
    $x = ($x * 48271) % 2147483647;
    my $w = "w" . ($x % 10000);
    $counts{$w}++;
}

my $distinct = scalar keys %counts;
my $max = 0;
for my $c (values %counts) {
    $max = $c if $c > $max;
}

print "Result: $distinct $n $max\n";
