# AGENT NOTES — dreammall

## Gotchas learned
- Bevy 0.20 splits render backends into crates. `bevy_ui` alone lays out UI
  but draws nothing; `bevy_ui_render` must be in the feature list or all UI
  (text, buttons, panels) silently never renders. This cost an entire bug
  hunt under headless lavapipe.
- Headless verification (no GPU): Xvfb `:99` + `WGPU_BACKEND=vulkan
  WGPU_SETTINGS_PRIO=webgpu` forces lavapipe; screenshots via
  `xdotool import`/`magick` or bevy's Screenshot API.
- Mouse pointer grab fails under some Wayland/headless setups; arrow-key
  turning is wired as a fallback (walking mode only).

## Tooling used
- System tools only (Xvfb, xdotool, imagemagick, cargo). Nothing installed.

## Debug env hooks (stripped before shipping)
- `DREAMMALL_SPAWN=x,z`, `DREAMMALL_PASS=1`, `DREAMMALL_SHOT=path:secs`
  existed for headless testing; removed from shipping code.
