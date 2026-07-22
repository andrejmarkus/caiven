-- String demo: literals + concatenation via draw_text

function _update()
  clear_screen()
  draw_text("hello, world!", 2, 2, 7)
  draw_text("real lua strings", 2, 12, 6)
  draw_text("foo" .. "bar", 2, 22, 5)
end
