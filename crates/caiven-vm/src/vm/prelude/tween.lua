function new_tween(from, to, frames, ease)
  return { from = from, to = to, frames = frames, ease = ease or ease_linear, t = 0, done = false }
end

function tween_update(tw)
  if tw.done then return tw.to end
  tw.t = tw.t + 1
  local p = tw.t / tw.frames
  if p >= 1 then
    p = 1
    tw.done = true
  end
  return tw.from + (tw.to - tw.from) * tw.ease(p)
end

function new_anim(frames, frame_len)
  return { frames = frames, frame_len = frame_len, timer = 0, index = 1 }
end

function anim_update(anim)
  anim.timer = anim.timer + 1
  if anim.timer >= anim.frame_len then
    anim.timer = 0
    anim.index = anim.index % #anim.frames + 1
  end
end

function anim_sprite(anim)
  return anim.frames[anim.index]
end
