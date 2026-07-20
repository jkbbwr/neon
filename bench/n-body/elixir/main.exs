# N-body: the benchmarks-game gravitational integrator, single-threaded.
# Faithful port of ../c/main.c — identical constants and operation order.
# Bodies are immutable tuples {x, y, z, vx, vy, vz, mass}; each advance step
# threads the five bodies through the same ten (i, j) pair updates as the C
# double loop, in the same order, then moves the positions.

defmodule NBody do
  @pi 3.141592653589793
  @solar_mass 4 * @pi * @pi
  @days_per_year 365.24

  def solar_mass, do: @solar_mass

  def initial_bodies do
    {
      # sun
      {0.0, 0.0, 0.0, 0.0, 0.0, 0.0, @solar_mass},
      # jupiter
      {4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01,
       1.66007664274403694e-03 * @days_per_year, 7.69901118419740425e-03 * @days_per_year,
       -6.90460016972063023e-05 * @days_per_year, 9.54791938424326609e-04 * @solar_mass},
      # saturn
      {8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01,
       -2.76742510726862411e-03 * @days_per_year, 4.99852801234917238e-03 * @days_per_year,
       2.30417297573763929e-05 * @days_per_year, 2.85885980666130812e-04 * @solar_mass},
      # uranus
      {1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01,
       2.96460137564761618e-03 * @days_per_year, 2.37847173959480950e-03 * @days_per_year,
       -2.96589568540237556e-05 * @days_per_year, 4.36624404335156298e-05 * @solar_mass},
      # neptune
      {1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01,
       2.68067772490389322e-03 * @days_per_year, 1.62824170038242295e-03 * @days_per_year,
       -9.51592254519715870e-05 * @days_per_year, 5.15138902046611451e-05 * @solar_mass}
    }
  end

  def offset_momentum({b0, b1, b2, b3, b4}) do
    {px, py, pz} =
      Enum.reduce([b0, b1, b2, b3, b4], {0.0, 0.0, 0.0}, fn
        {_x, _y, _z, vx, vy, vz, m}, {px, py, pz} ->
          {px + vx * m, py + vy * m, pz + vz * m}
      end)

    {x, y, z, _vx, _vy, _vz, m} = b0
    b0 = {x, y, z, -px / @solar_mass, -py / @solar_mass, -pz / @solar_mass, m}
    {b0, b1, b2, b3, b4}
  end

  # One pairwise interaction, C operation order: update i's velocity, then j's.
  defp pair({xi, yi, zi, vxi, vyi, vzi, mi}, {xj, yj, zj, vxj, vyj, vzj, mj}, dt) do
    dx = xi - xj
    dy = yi - yj
    dz = zi - zj
    d2 = dx * dx + dy * dy + dz * dz
    mag = dt / (d2 * :math.sqrt(d2))

    {{xi, yi, zi, vxi - dx * mj * mag, vyi - dy * mj * mag, vzi - dz * mj * mag, mi},
     {xj, yj, zj, vxj + dx * mi * mag, vyj + dy * mi * mag, vzj + dz * mi * mag, mj}}
  end

  defp move({x, y, z, vx, vy, vz, m}, dt) do
    {x + dt * vx, y + dt * vy, z + dt * vz, vx, vy, vz, m}
  end

  def advance({b0, b1, b2, b3, b4}, dt) do
    # The same (i, j) pair order as the C double loop.
    {b0, b1} = pair(b0, b1, dt)
    {b0, b2} = pair(b0, b2, dt)
    {b0, b3} = pair(b0, b3, dt)
    {b0, b4} = pair(b0, b4, dt)
    {b1, b2} = pair(b1, b2, dt)
    {b1, b3} = pair(b1, b3, dt)
    {b1, b4} = pair(b1, b4, dt)
    {b2, b3} = pair(b2, b3, dt)
    {b2, b4} = pair(b2, b4, dt)
    {b3, b4} = pair(b3, b4, dt)
    {move(b0, dt), move(b1, dt), move(b2, dt), move(b3, dt), move(b4, dt)}
  end

  def energy({b0, b1, b2, b3, b4}) do
    energy_loop([b0, b1, b2, b3, b4], 0.0)
  end

  defp energy_loop([], e), do: e

  defp energy_loop([{xi, yi, zi, vxi, vyi, vzi, mi} | rest], e) do
    e = e + 0.5 * mi * (vxi * vxi + vyi * vyi + vzi * vzi)

    e =
      Enum.reduce(rest, e, fn {xj, yj, zj, _vxj, _vyj, _vzj, mj}, e ->
        dx = xi - xj
        dy = yi - yj
        dz = zi - zj
        e - mi * mj / :math.sqrt(dx * dx + dy * dy + dz * dz)
      end)

    energy_loop(rest, e)
  end

  def run(bodies, 0, _dt), do: bodies
  def run(bodies, n, dt), do: run(advance(bodies, dt), n - 1, dt)

  def main do
    n = 20_000_000
    bodies = offset_momentum(initial_bodies())
    before = :erlang.float_to_binary(energy(bodies), decimals: 9)
    IO.puts(before)
    bodies = run(bodies, n, 0.01)
    after_ = :erlang.float_to_binary(energy(bodies), decimals: 9)
    IO.puts(after_)
    IO.puts("Result: #{before} #{after_}")
  end
end

NBody.main()
