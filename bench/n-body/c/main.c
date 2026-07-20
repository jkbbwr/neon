// N-body: the benchmarks-game gravitational integrator, single-threaded. Pure f64
// arithmetic over five bodies — no allocation, no strings, no containers in the hot
// loop. The energy is printed before and after at nine decimals; identical operation
// order makes those digits reproducible across languages, and the Result line pins
// both.
#include <stdio.h>
#include <math.h>

#define PI 3.141592653589793
#define SOLAR_MASS (4 * PI * PI)
#define DAYS_PER_YEAR 365.24
#define N_BODIES 5

typedef struct {
    double x, y, z, vx, vy, vz, mass;
} Body;

static Body bodies[N_BODIES] = {
    { // sun
        0, 0, 0, 0, 0, 0, SOLAR_MASS,
    },
    { // jupiter
        4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01,
        1.66007664274403694e-03 * DAYS_PER_YEAR,
        7.69901118419740425e-03 * DAYS_PER_YEAR,
        -6.90460016972063023e-05 * DAYS_PER_YEAR,
        9.54791938424326609e-04 * SOLAR_MASS,
    },
    { // saturn
        8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01,
        -2.76742510726862411e-03 * DAYS_PER_YEAR,
        4.99852801234917238e-03 * DAYS_PER_YEAR,
        2.30417297573763929e-05 * DAYS_PER_YEAR,
        2.85885980666130812e-04 * SOLAR_MASS,
    },
    { // uranus
        1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01,
        2.96460137564761618e-03 * DAYS_PER_YEAR,
        2.37847173959480950e-03 * DAYS_PER_YEAR,
        -2.96589568540237556e-05 * DAYS_PER_YEAR,
        4.36624404335156298e-05 * SOLAR_MASS,
    },
    { // neptune
        1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01,
        2.68067772490389322e-03 * DAYS_PER_YEAR,
        1.62824170038242295e-03 * DAYS_PER_YEAR,
        -9.51592254519715870e-05 * DAYS_PER_YEAR,
        5.15138902046611451e-05 * SOLAR_MASS,
    },
};

static void offset_momentum(void) {
    double px = 0, py = 0, pz = 0;
    for (int i = 0; i < N_BODIES; i++) {
        px += bodies[i].vx * bodies[i].mass;
        py += bodies[i].vy * bodies[i].mass;
        pz += bodies[i].vz * bodies[i].mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

static void advance(double dt) {
    for (int i = 0; i < N_BODIES; i++) {
        for (int j = i + 1; j < N_BODIES; j++) {
            double dx = bodies[i].x - bodies[j].x;
            double dy = bodies[i].y - bodies[j].y;
            double dz = bodies[i].z - bodies[j].z;
            double d2 = dx * dx + dy * dy + dz * dz;
            double mag = dt / (d2 * sqrt(d2));
            bodies[i].vx -= dx * bodies[j].mass * mag;
            bodies[i].vy -= dy * bodies[j].mass * mag;
            bodies[i].vz -= dz * bodies[j].mass * mag;
            bodies[j].vx += dx * bodies[i].mass * mag;
            bodies[j].vy += dy * bodies[i].mass * mag;
            bodies[j].vz += dz * bodies[i].mass * mag;
        }
    }
    for (int i = 0; i < N_BODIES; i++) {
        bodies[i].x += dt * bodies[i].vx;
        bodies[i].y += dt * bodies[i].vy;
        bodies[i].z += dt * bodies[i].vz;
    }
}

static double energy(void) {
    double e = 0;
    for (int i = 0; i < N_BODIES; i++) {
        e += 0.5 * bodies[i].mass *
             (bodies[i].vx * bodies[i].vx + bodies[i].vy * bodies[i].vy +
              bodies[i].vz * bodies[i].vz);
        for (int j = i + 1; j < N_BODIES; j++) {
            double dx = bodies[i].x - bodies[j].x;
            double dy = bodies[i].y - bodies[j].y;
            double dz = bodies[i].z - bodies[j].z;
            e -= bodies[i].mass * bodies[j].mass /
                 sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return e;
}

int main(void) {
    const int n = 20000000;
    offset_momentum();
    char before[32], after[32];
    snprintf(before, sizeof before, "%.9f", energy());
    printf("%s\n", before);
    for (int i = 0; i < n; i++) advance(0.01);
    snprintf(after, sizeof after, "%.9f", energy());
    printf("%s\n", after);
    printf("Result: %s %s\n", before, after);
    return 0;
}
