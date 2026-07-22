-- String demo: literals + concatenation via draw_text
-- Font only has uppercase glyphs (see FONT_GLYPHS in fc-vm/src/runtime.rs)

function _update()
  clear_screen()
  draw_text("HELLO, WORLD!", 2, 2, 7)
  draw_text("REAL LUA STRINGS", 2, 12, 6)
  draw_text("FOO" .. "BAR", 2, 22, 5)
end
