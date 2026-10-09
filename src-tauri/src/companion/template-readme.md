# winotch companion pack

This pack is a template: Bloop, one of winotch's built-in companions. Change
its pictures (or draw your own), then import it in winotch: Settings →
Companions → Add a companion.

A pack is a `.zip` (or a folder) holding `companion.json` and a PNG sheet.

## The sheet (`sheet.png`)

All the pictures, side by side, in a grid. They are numbered from 0, left to
right, then top to bottom. Every picture has the same size, set in
`companion.json` (`frameWidth` × `frameHeight`). Any pixel art tool works:
Piskel (free, online), Aseprite, LibreSprite… Export the animation as a
sprite sheet (PNG).

- Width and height in a **3:2** ratio (24 × 16, 36 × 24, 48 × 32…), from 8 to
  64 pixels high. The creature stands in the middle; the sides are room for
  its moves.
- A pixel is drawn when it is at least 50 % opaque.
- At most 512 pictures, a 2048 × 2048 sheet and 2 MB.

## `companion.json`

```json
{
  "name": "Pixel",
  "sheet": "sheet.png",
  "frameWidth": 24,
  "frameHeight": 16,
  "colors": "tint",
  "fps": 6,
  "animations": {
    "neutral": { "frames": ["0-3"] },
    "active": { "intro": [4, 5], "frames": ["6-9", 8, 7], "fps": 8 }
  }
}
```

- `name`: shown in winotch's menus.
- `sheet`: the PNG next to `companion.json` (`sheet.png` if left out).
- `colors`: `"tint"` (default): only the shape counts, winotch paints it like
  its own companions (the state's color, the color chosen by the user,
  "Darker"). `"own"`: the pack keeps its colors (62 at most); the state then
  only shows through the animation.
- `fps`: pictures per second (6 if left out); each animation may set its own.
- `animations`: one per state, and optionally per mood.
  - `frames`: the loop. A picture is its number, or a range `"a-b"`; repeat a
    number to hold a picture longer.
  - `intro`: played once when the state starts, before the loop.
  - `scenes`: lists of pictures played in a random order, each after the loop
    (`frames` is then a calm pause between them).

### States (Claude Code sessions)

| Name | When | Required |
|---|---|---|
| `neutral` | at rest | yes |
| `active` | working | yes |
| `attention` | waiting for a permission | no, plays `active` |
| `question` | asking a question | no, plays `active` |
| `success` | done | no, plays `neutral` |
| `error` | error | no, plays `active` |

### Moods (the Companion module)

`dance` (music playing), `run` (stopwatch running), `alarm` (a timer
ringing), `watch` (the mouse over it), `petted` (a click), `stretch` (each
o'clock), `idle` (awake, nothing happening), `sleep` (at night). All
optional: a missing mood plays a state.
