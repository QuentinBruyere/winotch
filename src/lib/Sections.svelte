<script lang="ts">
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import type { ModuleUis } from '../modules'
  import { sliderPercent, toneColor, type Section } from './content'
  import { icons } from './icons'

  // The modules shown in an open card or pin, one part each, its icon first
  // (DF-0011).
  let {
    list,
    note = null,
    height = null,
    moduleUis,
    onaction,
  }: {
    list: Section[]
    // Shown when the list is empty (no module enabled).
    note?: string | null
    // The card's final height, null to follow the card. A fixed height keeps
    // the rows still while the card unfolds over them.
    height?: number | null
    moduleUis: ModuleUis
    // A button of an item: `itemId` is `<module id>:<item id>`.
    onaction: (itemId: string, actionId: string) => void
  } = $props()

  function iconOf(module: string) {
    return moduleUis[module]?.icon ?? Puzzle
  }

  // While a slider is held, it shows the cursor's value, not the module's:
  // the module's own updates would make it jump back under the cursor.
  let held = $state<Record<string, number>>({})
</script>

<div
  class="list"
  class:centered={list.length === 0}
  style:height={height === null ? null : `${height}px`}
>
  {#each list as section (section.module)}
    {@const Icon = iconOf(section.module)}
    {@const dottedRows = section.items.some((i) => i.dot)}
    <div class="section">
      <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
      <ul class="rows">
        {#each section.items as item (item.id)}
          <li class="row">
            {#if item.dot}
              <span
                class="dot"
                class:pulse={item.tone === 'active'}
                style:background={toneColor(item.tone)}
              ></span>
            {:else if dottedRows}
              <!-- Keeps the texts aligned with the dotted rows. -->
              <span class="dot blank"></span>
            {/if}
            {#if item.title}<span class="title">{item.title}</span>{/if}
            {#if item.slider}
              {@const slider = item.slider}
              {@const value = held[item.id] ?? slider.value}
              <input
                type="range"
                class="slider"
                aria-label={item.title}
                min={slider.min}
                max={slider.max}
                step={slider.step}
                {value}
                style:--fill="{sliderPercent(slider, value)}%"
                onpointerdown={() => (held[item.id] = slider.value)}
                oninput={(e) => {
                  const next = Number(e.currentTarget.value)
                  held[item.id] = next
                  onaction(item.id, `set:${next}`)
                }}
                onpointerup={() => delete held[item.id]}
                onpointercancel={() => delete held[item.id]}
                onclick={(e) => e.stopPropagation()}
              />
            {/if}
            <span class="label" class:lead={!item.title && !item.slider} class:value={!!item.slider}>
              {item.label}{item.detail ? ` · ${item.detail}` : ''}
            </span>
            {#if item.actions.length > 0}
              <span class="actions">
                {#each item.actions as action (action.id)}
                  {@const ButtonIcon = action.icon ? icons[action.icon] : null}
                  <button
                    class="action"
                    class:text={!ButtonIcon}
                    title={action.label}
                    aria-label={action.label}
                    onclick={(e) => {
                      // Not an acknowledgement of the whole notch.
                      e.stopPropagation()
                      onaction(item.id, action.id)
                    }}
                  >
                    {#if ButtonIcon}
                      <ButtonIcon size={13} strokeWidth={2.5} />
                    {:else}
                      {action.label}
                    {/if}
                  </button>
                {/each}
              </span>
            {/if}
          </li>
        {/each}
        <!-- A module without items still says how it is (ADR-0009). -->
        {#if section.note}
          <li class="row"><span class="label muted note">{section.note}</span></li>
        {/if}
      </ul>
    </div>
  {:else}
    {#if note}
      <span class="row"><span class="label muted note">{note}</span></span>
    {/if}
  {/each}
</div>

<style>
  /* Fills the open card and centers its rows: a vertical notch never gets
     shorter than its compact shape, which leaves room around a short list.
     Past the window height it scrolls ("safe": the top stays reachable). */
  .list {
    box-sizing: border-box;
    height: 100%;
    padding: 8px 0 10px;
    display: grid;
    align-content: safe center;
    gap: 2px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .section {
    display: flex;
    gap: 8px;
    min-width: 0;
  }

  /* Between two modules: a thin line. Its height (6 + 1 + 8 px) is
     SEPARATOR_HEIGHT in App.svelte. */
  .section + .section {
    margin-top: 6px;
    padding-top: 8px;
    border-top: 1px solid rgb(255 255 255 / 0.12);
  }

  .rows {
    flex: 1;
    min-width: 0;
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 24px;
  }

  .centered .row {
    justify-content: center;
  }

  .title {
    flex: 0 1 auto;
    font-weight: 600;
  }

  .label {
    margin-left: auto;
    opacity: 0.75;
  }

  .note,
  .lead {
    margin-left: 0;
  }

  /* A slider fills the row; its value keeps a steady width after it. */
  .slider {
    flex: 1;
    min-width: 60px;
    height: 4px;
    margin: 0;
    appearance: none;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      var(--notch-fg) var(--fill),
      rgb(255 255 255 / 0.2) var(--fill)
    );
    cursor: pointer;
    outline: none;
  }

  .slider::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--notch-fg);
  }

  .label.value {
    margin-left: 0;
    min-width: 40px;
    text-align: right;
  }

  .actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .action {
    all: unset;
    display: grid;
    place-items: center;
    min-width: 20px;
    height: 20px;
    border-radius: 6px;
    background: rgb(255 255 255 / 0.12);
    color: var(--notch-fg);
    cursor: pointer;
    transition: background-color 150ms ease;
  }

  .action.text {
    padding: 0 7px;
    font-size: 11px;
  }

  .action:hover {
    background: rgb(255 255 255 / 0.25);
  }
</style>
