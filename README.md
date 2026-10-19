# DREAM MALL

You are trapped in a shopping mall inside a dream. You cannot walk backward.
Keep walking forward until you wake up.

Phase 1 of the dream simulator project: a small, complete, playable 3D
procedurally generated dream mall built in Rust with [Bevy](https://bevyengine.org).

## Run

```sh
cargo run --release
```

Linux, macOS, and Windows are supported (wgpu; X11 and Wayland on Linux).
Everything is generated at runtime — no asset files: textures, footstep/sfx,
and the muzak are synthesized procedurally.

## Controls

| Key | Action |
| --- | --- |
| `W` / `Up` | walk forward |
| `A` / `D` | strafe left / right |
| Left mouse | grab/release the mouse for looking |
| `Left` / `Right` arrows | turn (fallback when pointer grab is unavailable) |
| `E` | interact (penny, fountain, hotdog) |
| `Esc` | release mouse (in dream) / quit (ending screen) |

There is deliberately no backward walking.

## Escaping the dream

Two ways out, both optional, both findable in one loop or several:

1. **The penny.** A gold penny sits on a pedestal by the first fountain.
   Take it, then throw it into any fountain.
2. **The hotdog.** After a full lap the mall mutates and a hotdog appears on
   a food-court table. Eat it. You become light: fly up through the glass
   atrium roof, or dive into fountain water.

Then you wake up. "GO BACK TO SLEEP" restarts with a new seed.

## Design notes

- The mall is a 10-section ring (shops, food courts, fountains, one atrium).
  Each lap re-seeds section contents from a small reusable-kit vocabulary,
  so the mall feels larger than it is.
- Mirrors line the storefronts; one of them contains nyan cat.
- Muzak is generated (synth.rs) from seeded parameters and detunes as the
  dream degrades.
- `src/bin/simcheck.rs` runs headless verification of layout math, texture
  generation, and audio synthesis: `cargo run --bin simcheck`.
