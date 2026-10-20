# DREAM MALL

A small, complete, playable 3D procedurally generated dream simulator.
You are trapped in a strange shopping mall, forced to keep walking forward.
The mall loops. The mall remembers.

Rust + [Bevy](https://bevyengine.org). Runs on Linux, Mac, and Windows.

## Build & run

```sh
cargo run --release
```

First build compiles Bevy; be patient. Runtime asset generation is
procedural — no asset downloads, no data files.

## Controls

| Input | Action |
|---|---|
| `W` / `↑` | walk forward |
| `A` / `←` | step left (along the ring) |
| `D` / `→` | step right (along the ring) |
| mouse | look around (click to grab pointer) |
| `E` | interact |
| `F11` | toggle fullscreen |
| `Esc` | release pointer |

You cannot walk backward. The mall will not let you, and neither will we.

## The mall

The world is a circular ring of reusable procedurally assembled sections
(corridor, storefront, atrium, fountain court) around a central void.
Per-section variation comes from seeded textures, wall tints, ceiling
heights, prop placement, and mutation events, so repetition never quite
repeats. The ring loops forever; passes around it advance the dream.

## Getting out (two ways)

1. **The penny.** Somewhere in the mall a penny glints on a table. Find it,
   take it to the fountain, and throw it in. The water takes you down.
2. **The hotdog.** The food court hotdog is older than the mall. Eat it and
   the dream lets go of you — you rise through the glass roof and fly.

Which exit you find depends on what the mall reveals. Watch for toasts.

## Surreal features

- **Mirrors** reflect a version of the mall that is *almost* right. One of
  them carries Nyan Cat (real 8-bit pixels, rendered to a texture at
  build time).
- **The void.** Past the atrium glass the mall floats over a slowly
  churning psychedelic singularity.
- **Music.** Procedural MIDI-event muzak: bass, chords, melody, and drums
  scheduled from scales and voice-leading. Every dream renders its own set
  of six loops from its seed — same style rules, new performance.
- **Mutation.** Sections change color and reveal hidden items as passes
  advance. SOMETHING GLINTS ON A TABLE SOMEWHERE.

## Development

```sh
cargo run --bin headless   # full E2E regression: movement, reveals, glass,
                           # both win conditions, ending screen
cargo run --bin simcheck   # simulation sanity checks
```

`src/bin/headless.rs` drives the real app state machine without a window:
clean start, penny reveal gating, mutation invariants (atrium glass must
stay blend-transparent), fountain win, hotdog/flight win, ending buttons.

## Layout

| File | Role |
|---|---|
| `src/app.rs` | Bevy setup, input, camera, UI, win/ending flow |
| `src/world.rs` | ring geometry, section spawners, mutation |
| `src/texturegen.rs` | procedural textures (walls, ads, Nyan Cat, void) |
| `src/synth.rs` | WAV synthesis + MIDI-event muzak engine |
| `src/game.rs` | run state, reveals, win kinds |
| `src/shared.rs` | shared components/resources |

Phase 1 of the Dream Simulator project.
