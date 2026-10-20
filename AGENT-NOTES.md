# AGENT NOTES — dreammall

## Gotchas learned
- Bevy 0.20 splits render backends into crates. `bevy_ui` alone lays out UI
  but draws nothing; `bevy_ui_render` must be in the feature list or all UI
  (text, buttons, panels) silently never renders. This cost an entire bug
  hunt under headless lavapipe.
- Headless verification (no GPU): Xvfb `:99` + `WGPU_BACKEND=vulkan` forces
  llvmpipe/lavapipe; screenshots via `xdotool import`/`magick`.
- Mesa 26.x llvmpipe advertises `VK_KHR_cooperative_matrix` (properties
  query returns configs) but reports the feature `false` and its ICD
  rejects `vkCreateDevice` when the feature struct is passed. wgpu 30
  auto-requests `EXPERIMENTAL_COOPERATIVE_MATRIX` at `Functionality`
  priority → panic. Fixed in `app.rs`: RenderPlugin `WgpuSettings
  .disabled_features` disables that bit unconditionally (game never uses
  coop mats; harmless on other drivers). Note: `WGPU_SETTINGS_PRIO=webgpu`
  does NOT help — the feature is requested regardless of priority, and
  `WGPU_ALLOW_UNDERLYING_DEVICE_DISCREPANCY=1` only skips wgpu's own
  post-creation check, not the ICD rejection.
- Mouse pointer grab fails under some Wayland/headless setups; arrow-key
  turning is wired as a fallback (walking mode only).

## Tooling used
- System tools only (Xvfb, xdotool, imagemagick, cargo). Nothing installed.

- Headless driver supports `DREAMMALL_FLIGHT=1` (second process run): scripted
  hotdog take -> eat -> flight -> rise-past-roof win -> Flight ending. Water
  run remains the default (penny -> fountain dive -> Water ending).

## Debug env hooks (stripped before shipping)
- `DREAMMALL_SPAWN=x,z`, `DREAMMALL_PASS=1`, `DREAMMALL_SHOT=path:secs`
  existed for headless testing; removed from shipping code.
