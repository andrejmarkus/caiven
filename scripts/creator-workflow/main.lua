local settings = require("settings")
local x = settings.start_x

function _update()
  if button_down(3) then x = 40 end
  clear_screen()
  sprite(0, x, 16)
end
