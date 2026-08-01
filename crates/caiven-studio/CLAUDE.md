# caiven-studio

Tauri v2 desktop app. Frontend lives in `../caiven-studio-ui` (Vite, served on `http://localhost:1420` per `tauri.conf.json` `devUrl`).

## MCP-driven UI automation (`tauri-automation` server)

Debug builds include `tauri-plugin-webdriver-automation`, driven via `tauri-wd` and the `tauri-automation` MCP server (`~/mcp-servers/mcp-tauri-automation`). Before calling any `tauri-automation` tool (`launch_app`, `click_element`, etc.), make sure both of these are running:

```sh
curl -s http://127.0.0.1:1420 > /dev/null 2>&1 || (cd crates/caiven-studio-ui && npm run dev &)
curl -s http://127.0.0.1:4444/status > /dev/null 2>&1 || tauri-wd --port 4444 &
```

The debug binary loads the frontend from `devUrl`, not embedded files, so the Vite server must be up first. `tauri-wd` launches/kills the app binary itself — don't run `caiven-studio` manually alongside it.

Default app path is set via `TAURI_APP_PATH` in the MCP server config, pointing at `target/debug/caiven-studio`. Rebuild (`cargo build -p caiven-studio`) after code changes before asking Claude to relaunch.
