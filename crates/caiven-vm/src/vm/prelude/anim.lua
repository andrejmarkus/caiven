local anim = {}

function anim.new(frames, frame_len)
  return { frames = frames, frame_len = frame_len, timer = 0, index = 1 }
end

function anim.update(a)
  a.timer = a.timer + 1
  if a.timer >= a.frame_len then
    a.timer = 0
    a.index = a.index % #a.frames + 1
  end
end

function anim.sprite(a)
  return a.frames[a.index]
end

return anim
