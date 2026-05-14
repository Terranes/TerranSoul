# Scene illustrations for `docs/ai-memory-five-scenes-terransoul.md`

Five hero illustrations, one per scene. The doc references them as
`images/ai-memory-five-scenes/scene-N-*.webp`.

## Style guide (apply to every scene)

- **Format**: `.webp`, ~1600×900 (16:9), under ~250 KB each.
- **Mood**: warm, soft, slightly cinematic. Painterly digital
  illustration — *not* a flat vector diagram, *not* a screenshot, *not*
  a chart. The reader should *feel* the moment, not parse it.
- **Lighting**: single warm key light per scene (window light, TV glow,
  desk lamp, monitor glow, string lights). Soft shadows.
- **People**: shown from behind or three-quarter, faces partial or
  obscured. No recognisable real people, no logos, no readable brand
  text on screens or book spines.
- **Diversity**: the five characters across the scenes should look like
  five different people of different ages, ethnicities, and body types.
- **Palette**: muted, harmonious. Avoid neon. The TerranSoul brand
  accents (warm peach `#f4a261`, deep teal `#264653`, soft cream
  `#faf3e0`) can show up subtly in fabric, light, or props but should
  not dominate.
- **No text on the image**, no diagram arrows, no labels, no UI mockups.

## Per-scene prompts

### `scene-1-library.webp`

A small child (about twelve), backpack on, standing in the foreground
looking slightly up and away. Around them, towering wooden bookshelves
recede into warm afternoon light filtering through tall library
windows. In the middle distance, a curved wooden front desk with three
adult figures: one gesturing animatedly mid-sentence; one bent over a
glowing computer screen; one older woman with grey hair leaning forward
with a knowing half-smile, a stack of books tucked under one arm.
Painterly, calm, dust motes in the light shaft.

### `scene-2-movies.webp`

A dim living room at night. A teenager sits cross-legged on a couch
with a blanket pulled up to their chest, a half-finished bowl of
popcorn balanced on the cushion. The TV is the main light source; only
the glow on their face is visible, the screen content soft and
abstract. A smaller child is curled up asleep against their shoulder.
A phone is face-down on the cushion. Cosy, intimate, slightly tired
mood.

### `scene-3-exam.webp`

Past midnight at a small student desk. A college student, head propped
on one hand, hair messy, wearing a hoodie. Open laptop in front of
them, the screen washing their face in soft white-blue light. Around
them: stacks of paper notes, a textbook splayed open, a half-empty
mug, sticky notes around the monitor edge, a highlighter cap missing.
A single warm desk lamp in the corner. Tired, focused, sympathetic —
not depressing.

### `scene-4-job-hunt.webp`

A young engineer at a small apartment desk in early evening, leaning
forward toward a single monitor that shows several browser-tab
silhouettes (no readable text or logos). Beside the monitor, a piece
of notebook paper is taped to the wall with a hand-written wish-list
visible only as the *rhythm* of bullet points (no readable words). A
plant in a clay pot, a half-eaten sandwich on a small plate. Soft
warm window light from the side. Hopeful, slightly weary
determination.

### `scene-5-party.webp`

A rooftop party at dusk turning to night. String lights overhead, the
city skyline glowing in the background, slightly out of focus. Two
adults stand to one side in the foreground, both with drinks in hand,
leaning slightly toward each other in a quieter side-conversation
while a blurred crowd mingles behind them. Their body language is
relaxed but engaged — a moment of mutual recognition between
professionals who've been around the block. Warm bokeh, cinematic
depth of field.

## Generation notes

- Files must be created and dropped into this folder before the doc
  renders correctly on GitHub. Until then, GitHub will show a broken-image
  icon next to each scene heading.
- If you regenerate any image, keep the same filename so the doc
  doesn't need editing.
- If the chosen tool produces PNG, convert to WebP before committing
  (`cwebp -q 80 input.png -o scene-N-*.webp`) to keep repo weight down.
