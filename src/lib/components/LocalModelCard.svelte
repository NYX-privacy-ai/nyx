<script lang="ts">
  let {
    name,
    tag,
    size,
    description,
    minRam,
    systemRam,
    installed,
    downloading,
    onDownload,
    onDelete,
  }: {
    name: string;
    tag: string;
    size: string;
    description: string;
    minRam: number;
    systemRam: number;
    installed: boolean;
    downloading: boolean;
    onDownload: () => void;
    onDelete: () => void;
  } = $props();

  const canRun = $derived(systemRam >= minRam);
</script>

<div
  class="flex items-center gap-4 p-3.5 rounded-lg border transition-all duration-200 {installed ? 'border-accent/30 bg-accent/5' : 'border-border bg-surface'} {!canRun && !installed ? 'opacity-40' : ''}"
>
  <!-- Model icon -->
  <div
    class="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0 {canRun ? 'bg-accent/10 text-accent' : 'bg-surface-raised text-ivory-muted'}"
  >
    <svg class="w-4.5 h-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
      <path d="M8.25 3v1.5M4.5 8.25H3m18 0h-1.5M4.5 12H3m18 0h-1.5m-15 3.75H3m18 0h-1.5M8.25 19.5V21M12 3v1.5m0 15V21m3.75-18v1.5m0 15V21m-9-1.5h9a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0015.75 4.5h-9A2.25 2.25 0 004.5 6.75v10.5A2.25 2.25 0 006.75 19.5z" />
    </svg>
  </div>

  <!-- Model info -->
  <div class="flex-1 min-w-0">
    <div class="flex items-center gap-2">
      <span class="text-ivory text-sm font-medium">{name}</span>
      <span class="text-ivory-muted/50 text-[10px] px-1.5 py-0.5 rounded bg-surface-raised">{size}</span>
    </div>
    <p class="text-ivory-muted text-xs mt-0.5 truncate">
      {#if !canRun}
        Requires {minRam}GB RAM
      {:else}
        {description}
      {/if}
    </p>
  </div>

  <!-- Action -->
  <div class="flex items-center gap-2 flex-shrink-0">
    {#if downloading}
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin"></div>
        <span class="text-ivory-muted text-xs">Pulling...</span>
      </div>
    {:else if installed}
      <!-- Installed checkmark -->
      <svg class="w-4.5 h-4.5 text-positive" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <!-- Delete button -->
      <button
        onclick={onDelete}
        class="text-ivory-muted/40 hover:text-negative text-xs transition-colors p-1"
        title="Remove model"
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
        </svg>
      </button>
    {:else if canRun}
      <button
        onclick={onDownload}
        class="text-xs px-3 py-1.5 rounded-md bg-accent/10 text-accent hover:bg-accent/20 transition-colors"
      >
        Download
      </button>
    {:else}
      <span class="text-ivory-muted/30 text-xs">Unavailable</span>
    {/if}
  </div>
</div>
