local tween = {}

function tween.new(from, to, frames, ease)
  return { from = from, to = to, frames = frames, ease = ease or ease_linear, t = 0, done = false }
end

function tween.update(tw)
  if tw.done then return tw.to end
  tw.t = tw.t + 1
  local p = tw.t / tw.frames
  if p >= 1 then
    p = 1
    tw.done = true
  end
  return tw.from + (tw.to - tw.from) * tw.ease(p)
end

return tween
