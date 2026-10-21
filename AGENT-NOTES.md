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
- README screenshots are retaken with the `headless` test binary's shot
  mode (NOT in shipping code): `DREAMMALL_SHOT=path:x,z:heading:secs`
  teleports the player, then takes a GPU-side swapchain capture (Bevy
  Screenshot + save_to_disk observer). Run under Xvfb sized exactly to
  the target art (1600x900). The target directory must pre-exist;
  save_to_disk fails silently otherwise.

## Dark-band complaint (4K laptop)
- Two causes. (1) Capture artifact in our own Xvfb runs: the game window is
  1600x900 top-left; everything beyond it in a full-root screenshot is the
  black desktop, not the game. (2) Real scene issue: far sections at ambient
  55 sat at luminance ~15 and the atrium glass looked through a near-black
  void horizon band. Fixed: ambient 130, light range 34, glowing void
  horizon, window opens maximized + F11 fullscreen toggle (needs a WM;
  no-op under bare Xvfb, applies on KDE).
