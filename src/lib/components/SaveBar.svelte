<script lang="ts">
  let {
    hasChanges,
    saving,
    restartRequired,
    saveError = '',
    onSave,
    onDiscard,
  }: {
    hasChanges: boolean;
    saving: boolean;
    restartRequired: boolean;
    saveError?: string;
    onSave: () => void;
    onDiscard: () => void;
  } = $props();
</script>

{#if hasChanges || saving || saveError}
  <div class="fixed bottom-0 left-[60px] right-0 bg-surface border-t border-gold/30 px-6 py-3 flex items-center justify-between z-50 animate-slide-up">
    <div class="flex items-center gap-3">
      {#if saving}
        <div class="w-4 h-4 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
        <span class="text-ivory text-sm">Saving...</span>
      {:else if saveError}
        <svg class="w-4 h-4 text-negative flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
        <span class="text-negative text-sm">{saveError}</span>
      {:else}
        <div class="w-2 h-2 rounded-full bg-warning animate-pulse"></div>
        <span class="text-ivory-muted text-sm">Unsaved changes</span>
        {#if restartRequired}
          <span class="text-[10px] px-2 py-0.5 rounded-full bg-warning/10 text-warning">Restart required</span>
        {/if}
      {/if}
    </div>
    <div class="flex items-center gap-3">
      <button
        onclick={onDiscard}
        disabled={saving}
        class="text-ivory-muted text-xs hover:text-ivory transition-colors disabled:opacity-50"
      >
        Discard
      </button>
      <button
        onclick={onSave}
        disabled={saving}
        class="px-5 py-1.5 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {restartRequired ? 'Save & Restart' : 'Save'}
      </button>
    </div>
  </div>
{/if}

<style>
  @keyframes slide-up {
    from { transform: translateY(100%); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  .animate-slide-up {
    animation: slide-up 0.3s ease-out;
  }
</style>
