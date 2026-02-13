<script lang="ts">
  let {
    provider,
    description = '',
    configured,
    helpUrl,
    placeholder = '',
    onSave,
  }: {
    provider: string;
    description?: string;
    configured: boolean;
    helpUrl: string;
    placeholder?: string;
    onSave: (key: string) => void;
  } = $props();

  let editing = $state(false);
  let keyValue = $state('');

  function save() {
    if (keyValue.trim()) {
      onSave(keyValue.trim());
      keyValue = '';
      editing = false;
    }
  }

  function cancel() {
    keyValue = '';
    editing = false;
  }

  async function openExternal(url: string) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    } catch {
      window.open(url, '_blank');
    }
  }
</script>

<div class="flex items-center gap-3 py-2.5">
  <div class="flex-1 min-w-0">
    <div class="flex items-center gap-2">
      <span class="text-ivory text-sm">{provider}</span>
      {#if description}
        <span class="text-ivory-muted/40 text-[10px]">{description}</span>
      {/if}
    </div>
  </div>

  <div class="flex items-center gap-2 flex-shrink-0">
    {#if editing}
      <input
        type="password"
        bind:value={keyValue}
        {placeholder}
        class="w-56 bg-surface text-ivory text-xs px-3 py-1.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors selectable"
        onkeydown={(e) => { if (e.key === 'Enter') save(); if (e.key === 'Escape') cancel(); }}
      />
      <button onclick={save} class="text-xs px-2.5 py-1 rounded bg-accent/10 text-accent hover:bg-accent/20 transition-colors">Save</button>
      <button onclick={cancel} class="text-xs text-ivory-muted hover:text-ivory transition-colors">Cancel</button>
    {:else}
      {#if configured}
        <div class="flex items-center gap-1.5 text-positive text-xs">
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          Configured
        </div>
      {:else}
        <span class="text-ivory-muted/40 text-xs">Not set</span>
      {/if}
      <button
        onclick={() => editing = true}
        class="text-xs px-2.5 py-1 rounded border border-border text-ivory-muted hover:text-ivory hover:border-ivory-muted/30 transition-colors"
      >
        {configured ? 'Update' : 'Add'}
      </button>
      <button
        onclick={() => openExternal(helpUrl)}
        class="text-gold-dim hover:text-gold transition-colors"
        title="Get API key"
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
        </svg>
      </button>
    {/if}
  </div>
</div>
