<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Companion from '../lib/Companion.svelte'
  import { companions } from '../lib/companions'
  import { customCompanions } from '../lib/companions/custom.svelte'
  import { templatePack } from '../lib/companions/template'
  import { t, type Key } from '../lib/i18n.svelte'

  // The Companions category (DF-0025): the built-in companions, shown only;
  // then the imported ones, each with a preview, or why its pack is
  // refused; add one, remove one (two clicks), the folder, and a template
  // made from Bloop to start from.
  let message = $state<{ text: string; error: boolean } | null>(null)
  // The companion whose "Delete" was clicked once.
  let confirming = $state<string | null>(null)
  const list = $derived(customCompanions())

  async function attempt(work: () => Promise<string | null>) {
    message = null
    try {
      const text = await work()
      if (text) message = { text, error: false }
    } catch (e) {
      message = { text: String(e), error: true }
    }
  }

  const add = () =>
    attempt(async () => {
      const name = await invoke<string | null>('import_companion')
      return name ? t('settings.companions.added', { name }) : null
    })

  function remove(id: string) {
    if (confirming !== id) {
      confirming = id
      return
    }
    confirming = null
    void attempt(async () => {
      await invoke('delete_companion', { id })
      return null
    })
  }

  const openFolder = () =>
    attempt(async () => {
      await invoke('reveal_companions')
      return null
    })

  const exportTemplate = () =>
    attempt(async () => {
      const { sheet, manifest } = await templatePack(companions.bloop, 'Bloop')
      const saved = await invoke<boolean>('export_companion_template', { sheet, manifest })
      return saved ? t('settings.companions.exported') : null
    })
</script>

<section>
  <h2>{t('settings.companions.built_in')}</h2>
  <div class="companion-tiles">
    {#each Object.keys(companions) as id (id)}
      <div class="companion-tile">
        <div class="companion-preview">
          <Companion item="settings:{id}" {id} size="large" tone="active" compact />
        </div>
        <div class="label">{t(`companion.${id}` as Key)}</div>
      </div>
    {/each}
  </div>
</section>

<section>
  <h2>{t('settings.companions')}</h2>
  <p class="hint">{t('settings.companions.hint')}</p>
  {#if list.length === 0}
    <div class="hint separated">{t('settings.companions.empty')}</div>
  {/if}
  {#each list as custom (custom.id)}
    <div class="row separated">
      <div class="custom-companion">
        <div class="companion-preview">
          {#if custom.definition}
            <Companion item="settings:{custom.id}" id={custom.id} size="large" tone="active" compact />
          {/if}
        </div>
        <div>
          <div class="label">{custom.name}</div>
          {#if custom.error}<div class="error">{custom.error}</div>{/if}
        </div>
      </div>
      <button onclick={() => remove(custom.id)} onblur={() => (confirming = null)}>
        {confirming === custom.id
          ? t('settings.companions.confirm_delete')
          : t('settings.companions.delete')}
      </button>
    </div>
  {/each}
  <div class="actions separated">
    <button class="primary" onclick={add}>{t('settings.companions.add')}</button>
    <button onclick={openFolder}>{t('settings.companions.open_folder')}</button>
    <button onclick={exportTemplate}>{t('settings.companions.export')}</button>
  </div>
  {#if message}
    <p class={message.error ? 'error' : 'hint'} role="status">{message.text}</p>
  {/if}
</section>
