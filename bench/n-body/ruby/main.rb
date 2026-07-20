# N-body: the benchmarks-game gravitational integrator, single-threaded.
# Faithful port of ../c/main.c — identical constants and operation order.

PI = 3.141592653589793
SOLAR_MASS = 4 * PI * PI
DAYS_PER_YEAR = 365.24
N_BODIES = 5

# Each body is [x, y, z, vx, vy, vz, mass].
BODIES = [
  [ # sun
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS,
  ],
  [ # jupiter
    4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01,
    1.66007664274403694e-03 * DAYS_PER_YEAR,
    7.69901118419740425e-03 * DAYS_PER_YEAR,
    -6.90460016972063023e-05 * DAYS_PER_YEAR,
    9.54791938424326609e-04 * SOLAR_MASS,
  ],
  [ # saturn
    8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01,
    -2.76742510726862411e-03 * DAYS_PER_YEAR,
    4.99852801234917238e-03 * DAYS_PER_YEAR,
    2.30417297573763929e-05 * DAYS_PER_YEAR,
    2.85885980666130812e-04 * SOLAR_MASS,
  ],
  [ # uranus
    1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01,
    2.96460137564761618e-03 * DAYS_PER_YEAR,
    2.37847173959480950e-03 * DAYS_PER_YEAR,
    -2.96589568540237556e-05 * DAYS_PER_YEAR,
    4.36624404335156298e-05 * SOLAR_MASS,
  ],
  [ # neptune
    1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01,
    2.68067772490389322e-03 * DAYS_PER_YEAR,
    1.62824170038242295e-03 * DAYS_PER_YEAR,
    -9.51592254519715870e-05 * DAYS_PER_YEAR,
    5.15138902046611451e-05 * SOLAR_MASS,
  ],
]

# Pairs in the same (i, j) order as the C double loop: i < j.
PAIRS = []
(0...N_BODIES).each do |i|
  ((i + 1)...N_BODIES).each do |j|
    PAIRS << [BODIES[i], BODIES[j]]
  end
end

def offset_momentum
  px = 0.0
  py = 0.0
  pz = 0.0
  BODIES.each do |b|
    px += b[3] * b[6]
    py += b[4] * b[6]
    pz += b[5] * b[6]
  end
  BODIES[0][3] = -px / SOLAR_MASS
  BODIES[0][4] = -py / SOLAR_MASS
  BODIES[0][5] = -pz / SOLAR_MASS
end

def advance(dt)
  PAIRS.each do |bi, bj|
    dx = bi[0] - bj[0]
    dy = bi[1] - bj[1]
    dz = bi[2] - bj[2]
    d2 = dx * dx + dy * dy + dz * dz
    mag = dt / (d2 * Math.sqrt(d2))
    mi = bi[6]
    mj = bj[6]
    bi[3] -= dx * mj * mag
    bi[4] -= dy * mj * mag
    bi[5] -= dz * mj * mag
    bj[3] += dx * mi * mag
    bj[4] += dy * mi * mag
    bj[5] += dz * mi * mag
  end
  BODIES.each do |b|
    b[0] += dt * b[3]
    b[1] += dt * b[4]
    b[2] += dt * b[5]
  end
end

def energy
  e = 0.0
  (0...N_BODIES).each do |i|
    bi = BODIES[i]
    e += 0.5 * bi[6] * (bi[3] * bi[3] + bi[4] * bi[4] + bi[5] * bi[5])
    ((i + 1)...N_BODIES).each do |j|
      bj = BODIES[j]
      dx = bi[0] - bj[0]
      dy = bi[1] - bj[1]
      dz = bi[2] - bj[2]
      e -= bi[6] * bj[6] / Math.sqrt(dx * dx + dy * dy + dz * dz)
    end
  end
  e
end

n = 20000000
offset_momentum
before = format("%.9f", energy)
puts before
n.times { advance(0.01) }
after = format("%.9f", energy)
puts after
puts "Result: #{before} #{after}"
