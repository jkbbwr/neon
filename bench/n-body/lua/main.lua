-- N-body: the benchmarks-game gravitational integrator, single-threaded.
-- Faithful port of ../c/main.c — identical constants and operation order.
-- Works under both lua and luajit.

local sqrt = math.sqrt

local PI = 3.141592653589793
local SOLAR_MASS = 4 * PI * PI
local DAYS_PER_YEAR = 365.24
local N_BODIES = 5

local bodies = {
  { -- sun
    x = 0, y = 0, z = 0, vx = 0, vy = 0, vz = 0, mass = SOLAR_MASS,
  },
  { -- jupiter
    x = 4.84143144246472090e+00,
    y = -1.16032004402742839e+00,
    z = -1.03622044471123109e-01,
    vx = 1.66007664274403694e-03 * DAYS_PER_YEAR,
    vy = 7.69901118419740425e-03 * DAYS_PER_YEAR,
    vz = -6.90460016972063023e-05 * DAYS_PER_YEAR,
    mass = 9.54791938424326609e-04 * SOLAR_MASS,
  },
  { -- saturn
    x = 8.34336671824457987e+00,
    y = 4.12479856412430479e+00,
    z = -4.03523417114321381e-01,
    vx = -2.76742510726862411e-03 * DAYS_PER_YEAR,
    vy = 4.99852801234917238e-03 * DAYS_PER_YEAR,
    vz = 2.30417297573763929e-05 * DAYS_PER_YEAR,
    mass = 2.85885980666130812e-04 * SOLAR_MASS,
  },
  { -- uranus
    x = 1.28943695621391310e+01,
    y = -1.51111514016986312e+01,
    z = -2.23307578892655734e-01,
    vx = 2.96460137564761618e-03 * DAYS_PER_YEAR,
    vy = 2.37847173959480950e-03 * DAYS_PER_YEAR,
    vz = -2.96589568540237556e-05 * DAYS_PER_YEAR,
    mass = 4.36624404335156298e-05 * SOLAR_MASS,
  },
  { -- neptune
    x = 1.53796971148509165e+01,
    y = -2.59193146099879641e+01,
    z = 1.79258772950371181e-01,
    vx = 2.68067772490389322e-03 * DAYS_PER_YEAR,
    vy = 1.62824170038242295e-03 * DAYS_PER_YEAR,
    vz = -9.51592254519715870e-05 * DAYS_PER_YEAR,
    mass = 5.15138902046611451e-05 * SOLAR_MASS,
  },
}

local function offset_momentum()
  local px, py, pz = 0, 0, 0
  for i = 1, N_BODIES do
    local b = bodies[i]
    px = px + b.vx * b.mass
    py = py + b.vy * b.mass
    pz = pz + b.vz * b.mass
  end
  bodies[1].vx = -px / SOLAR_MASS
  bodies[1].vy = -py / SOLAR_MASS
  bodies[1].vz = -pz / SOLAR_MASS
end

local function advance(dt)
  for i = 1, N_BODIES do
    local bi = bodies[i]
    for j = i + 1, N_BODIES do
      local bj = bodies[j]
      local dx = bi.x - bj.x
      local dy = bi.y - bj.y
      local dz = bi.z - bj.z
      local d2 = dx * dx + dy * dy + dz * dz
      local mag = dt / (d2 * sqrt(d2))
      bi.vx = bi.vx - dx * bj.mass * mag
      bi.vy = bi.vy - dy * bj.mass * mag
      bi.vz = bi.vz - dz * bj.mass * mag
      bj.vx = bj.vx + dx * bi.mass * mag
      bj.vy = bj.vy + dy * bi.mass * mag
      bj.vz = bj.vz + dz * bi.mass * mag
    end
  end
  for i = 1, N_BODIES do
    local b = bodies[i]
    b.x = b.x + dt * b.vx
    b.y = b.y + dt * b.vy
    b.z = b.z + dt * b.vz
  end
end

local function energy()
  local e = 0
  for i = 1, N_BODIES do
    local bi = bodies[i]
    e = e + 0.5 * bi.mass * (bi.vx * bi.vx + bi.vy * bi.vy + bi.vz * bi.vz)
    for j = i + 1, N_BODIES do
      local bj = bodies[j]
      local dx = bi.x - bj.x
      local dy = bi.y - bj.y
      local dz = bi.z - bj.z
      e = e - bi.mass * bj.mass / sqrt(dx * dx + dy * dy + dz * dz)
    end
  end
  return e
end

local n = 20000000
offset_momentum()
local before = string.format("%.9f", energy())
print(before)
for _ = 1, n do
  advance(0.01)
end
local after = string.format("%.9f", energy())
print(after)
print(string.format("Result: %s %s", before, after))
