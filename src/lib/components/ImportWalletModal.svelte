<script lang="ts">
  let {
    open,
    onClose,
    onImport,
  }: {
    open: boolean;
    onClose: () => void;
    onImport: (chain: string, address: string, label: string) => void;
  } = $props();

  let chain = $state('ETH');
  let address = $state('');
  let label = $state('');
  let error = $state('');

  const chains = ['NEAR', 'ETH', 'SOL', 'BTC', 'ZEC'];

  const isValid = $derived(() => {
    if (!address.trim()) return false;
    if (chain === 'ETH') return address.startsWith('0x') && address.length === 42;
    if (chain === 'SOL') return address.length >= 32 && address.length <= 44;
    if (chain === 'BTC') return address.length >= 25 && address.length <= 62;
    if (chain === 'NEAR') return address.endsWith('.near') || address.endsWith('.testnet') || address.length === 64;
    if (chain === 'ZEC') return address.startsWith('t1') || address.startsWith('t3') || address.startsWith('zs1') || address.startsWith('u1');
    return false;
  });

  function handleSubmit() {
    if (!isValid()) return;
    const finalLabel = label.trim() || `${chain} wallet`;
    onImport(chain, address.trim(), finalLabel);
    // Reset
    chain = 'ETH';
    address = '';
    label = '';
    error = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  const placeholder = $derived(
    chain === 'ETH' ? '0x...' :
    chain === 'SOL' ? 'Base58 address...' :
    chain === 'BTC' ? 'bc1... / 1... / 3...' :
    chain === 'ZEC' ? 't1... / zs1... / u1...' :
    'account.near or 64-char hex'
  );
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    onkeydown={handleKeydown}
  >
    <!-- Backdrop -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute inset-0 bg-black/60" onclick={onClose}></div>

    <!-- Modal -->
    <div class="relative bg-surface border border-border rounded-lg p-6 w-full max-w-md mx-4">
      <h3 class="font-display text-xl font-light tracking-wider text-ivory mb-6">Import Wallet</h3>

      <div class="space-y-4">
        <!-- Chain selector -->
        <div>
          <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Chain</label>
          <select
            bind:value={chain}
            class="w-full bg-surface-raised text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors"
          >
            {#each chains as c}
              <option value={c}>{c}</option>
            {/each}
          </select>
        </div>

        <!-- Address -->
        <div>
          <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Address</label>
          <input
            type="text"
            bind:value={address}
            placeholder={placeholder}
            class="w-full bg-surface-raised text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors selectable font-mono"
          />
        </div>

        <!-- Label -->
        <div>
          <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Label (optional)</label>
          <input
            type="text"
            bind:value={label}
            placeholder={`My ${chain} wallet`}
            class="w-full bg-surface-raised text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors selectable"
          />
        </div>
      </div>

      {#if error}
        <p class="text-negative text-xs mt-3">{error}</p>
      {/if}

      <div class="flex justify-end gap-3 mt-6">
        <button
          onclick={onClose}
          class="px-4 py-2 text-ivory-muted text-sm hover:text-ivory transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={handleSubmit}
          disabled={!isValid()}
          class="px-6 py-2 border text-sm rounded transition-colors duration-200 {isValid() ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted cursor-not-allowed'}"
        >
          Import
        </button>
      </div>
    </div>
  </div>
{/if}
