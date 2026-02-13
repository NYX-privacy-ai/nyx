<script lang="ts">
  let {
    chain,
    address,
    label,
    isActive,
    hasPrivateKey,
    onSetActive,
    onRemove,
  }: {
    chain: string;
    address: string;
    label: string;
    isActive: boolean;
    hasPrivateKey: boolean;
    onSetActive: () => void;
    onRemove: () => void;
  } = $props();

  const chainColors: Record<string, string> = {
    NEAR: 'bg-positive/30 text-positive',
    ETH: 'bg-blue-500/20 text-blue-400',
    SOL: 'bg-purple-500/20 text-purple-400',
    BTC: 'bg-orange-500/20 text-orange-400',
    ZEC: 'bg-amber-500/20 text-amber-300',
  };

  const chainColor = $derived(chainColors[chain] || 'bg-ivory-muted/20 text-ivory-muted');
  const truncated = $derived(
    address.length > 14
      ? `${address.slice(0, 6)}...${address.slice(-4)}`
      : address
  );
</script>

<div
  class="flex items-center justify-between py-3 px-4 rounded-lg border transition-colors duration-200"
  class:border-gold-dim={isActive}
  class:bg-surface-raised={isActive}
  class:border-border={!isActive}
  class:bg-surface={!isActive}
>
  <div class="flex items-center gap-3">
    <!-- Chain icon -->
    <div class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold {chainColor}">
      {chain.charAt(0)}
    </div>

    <!-- Info -->
    <div class="flex flex-col">
      <div class="flex items-center gap-2">
        <span class="text-ivory text-sm">{label}</span>
        {#if isActive}
          <span class="text-gold text-[10px] uppercase tracking-wider border border-gold-dim rounded px-1.5 py-0.5">Active</span>
        {/if}
        {#if !hasPrivateKey}
          <span class="text-ivory-muted text-[10px] uppercase tracking-wider border border-border rounded px-1.5 py-0.5">Watch-only</span>
        {/if}
      </div>
      <span class="text-ivory-muted text-xs font-mono">{truncated}</span>
    </div>
  </div>

  <!-- Actions -->
  <div class="flex items-center gap-2">
    {#if !isActive}
      <button
        onclick={onSetActive}
        class="text-ivory-muted text-xs hover:text-gold transition-colors"
      >
        Set Active
      </button>
    {/if}
    <button
      onclick={onRemove}
      class="text-ivory-muted text-xs hover:text-negative transition-colors"
    >
      Remove
    </button>
  </div>
</div>
