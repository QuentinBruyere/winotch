<script lang="ts">
  // Molf's head, as on the logo (assets/logo.png): one square per '#',
  // drawn as a single path in the text color.
  const HEAD = [
    '..##....##..',
    '.##.#..#.##.',
    '.#..####..#.',
    '##...##...##',
    '#..........#',
    '#..........#',
    '#.#......#.#',
    '#..........#',
    '#.#.####.#.#',
    '#.#.#..#.#.#',
  ]

  const d = HEAD.flatMap((row, y) =>
    [...row].flatMap((cell, x) => (cell === '#' ? [`M${x} ${y}h1v1h-1z`] : [])),
  ).join('')
</script>

<svg class="logo" viewBox="0 0 12 10" aria-hidden="true">
  <path {d} fill="currentColor" shape-rendering="crispEdges" />
</svg>
