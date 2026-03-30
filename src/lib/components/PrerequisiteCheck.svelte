<script lang="ts">
  let {
    label,
    status,
    version = null,
    downloadUrl = null,
    optional = false,
    onInstall = undefined,
    installError = '',
  }: {
    label: string;
    status: 'checking' | 'installed' | 'not_installed' | 'running' | 'not_running' | 'installing';
    version?: string | null;
    downloadUrl?: string | null;
    optional?: boolean;
    onInstall?: (() => void) | undefined;
    installError?: string;
  } = $props();

  const dotColor = $derived(
    status === 'running'
      ? 'bg-positive'
      : status === 'installed' || status === 'not_running'
        ? 'bg-accent'
        : status === 'not_installed'
          ? 'bg-negative'
          : status === 'installing'
            ? 'bg-accent'
            : 'bg-ivory-muted'
  );

  const statusLabel = $derived(
    status === 'checking'
      ? 'Checking...'
      : status === 'running'
        ? 'Running'
        : status === 'installed'
          ? 'Installed'
          : status === 'not_running'
            ? 'Not running'
            : status === 'installing'
              ? 'Installing...'
              : 'Not installed'
  );
</script>

<div class="flex items-center justify-between py-3 px-4 rounded-lg bg-surface border border-border">
  <div class="flex items-center gap-3">
    {#if status === 'checking' || status === 'installing'}
      <div class="w-2.5 h-2.5 rounded-full bg-ivory-muted animate-pulse"></div>
    {:else}
      <div class="w-2.5 h-2.5 rounded-full {dotColor}"></div>
    {/if}

    <div class="flex flex-col">
      <div class="flex items-center gap-2">
        <span class="text-ivory text-sm">{label}</span>
        {#if optional}
          <span class="text-ivory-muted text-[10px] uppercase tracking-wider border border-border rounded px-1.5 py-0.5">
            Optional
          </span>
        {/if}
      </div>
      {#if version}
        <span class="text-ivory-muted text-xs font-mono">{version}</span>
      {/if}
      {#if installError}
        <span class="text-negative text-xs">{installError}</span>
      {/if}
    </div>
  </div>

  <div class="flex items-center gap-3">
    <span
      class="text-xs tracking-wide"
      class:text-positive={status === 'running'}
      class:text-gold={status === 'installed' || status === 'not_running' || status === 'installing'}
      class:text-negative={status === 'not_installed'}
      class:text-ivory-muted={status === 'checking'}
    >
      {statusLabel}
    </span>

    {#if status === 'not_installed' && onInstall}
      <button
        onclick={onInstall}
        class="text-xs text-gold hover:text-gold-dim transition-colors duration-200 underline underline-offset-2"
      >
        Install
      </button>
    {:else if status === 'not_installed' && downloadUrl}
      <a
        href={downloadUrl}
        target="_blank"
        rel="noopener noreferrer"
        class="text-xs text-gold hover:text-gold-dim transition-colors duration-200 underline underline-offset-2"
      >
        Download
      </a>
    {/if}
  </div>
</div>
