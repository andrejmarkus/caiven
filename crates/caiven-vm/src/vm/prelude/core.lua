-- Deterministic by default: a fresh Lua VM has RTK_SEEDED unset, so this
-- seeds once. Hot reload re-runs prelude.lua on the *same* live VM, where
-- RTK_SEEDED is already true, so a hot reload during dev doesn't reset the
-- live RNG stream mid-game. Carts opt out via their own math.randomseed(x).
if not RTK_SEEDED then
  math.randomseed(1)
  RTK_SEEDED = true
end

function random_range(lo, hi)
  return math.random(lo, hi)
end

function random_float(lo, hi)
  return lo + math.random() * (hi - lo)
end

function choice(t)
  local n = #t
  if n == 0 then
    error("choice() requires a non-empty table", 2)
  end
  return t[math.random(n)]
end

function shuffle(t)
  for i = #t, 2, -1 do
    local j = math.random(i)
    t[i], t[j] = t[j], t[i]
  end
  return t
end

function lerp(a, b, t)
  return a + (b - a) * t
end

function clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

function ease_linear(t) return t end
function ease_in_quad(t) return t * t end
function ease_out_quad(t) return 1 - (1 - t) * (1 - t) end
function ease_in_out_quad(t)
  if t < 0.5 then return 2 * t * t end
  return 1 - ((-2 * t + 2) ^ 2) / 2
end
