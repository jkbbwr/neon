# Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
# plain per-node array-ref allocation left to refcounting — no pools or arenas.
use strict;
use warnings;

sub make {
    my ($depth) = @_;
    return [undef, undef] if $depth == 0;
    return [make($depth - 1), make($depth - 1)];
}

sub check {
    my ($n) = @_;
    return 0 unless defined $n;
    return 1 + check($n->[0]) + check($n->[1]);
}

my $max_depth = 18;
my $total = 0;

my $stretch = make($max_depth + 1);
my $sc = check($stretch);
undef $stretch;
printf "stretch tree of depth %d check: %d\n", $max_depth + 1, $sc;
$total += $sc;

my $long_lived = make($max_depth);

for (my $depth = 4; $depth <= $max_depth; $depth += 2) {
    my $iterations = 1 << ($max_depth - $depth + 4);
    my $sum = 0;
    for (my $i = 0; $i < $iterations; $i++) {
        my $t = make($depth);
        $sum += check($t);
    }
    printf "%d trees of depth %d check: %d\n", $iterations, $depth, $sum;
    $total += $sum;
}

my $ll = check($long_lived);
printf "long lived tree of depth %d check: %d\n", $max_depth, $ll;
$total += $ll;

printf "Result: %d\n", $total;
