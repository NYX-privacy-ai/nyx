<script lang="ts">
  let {
    name,
    description,
    selected,
    onclick,
    metrics,
  }: {
    name: string;
    description: string;
    selected: boolean;
    onclick: () => void;
    metrics: { maxTx: string; dailyLoss: string; confirmation: string };
  } = $props();

  const shieldColor = $derived(
    name === 'Conservative' ? 'text-positive' :
    name === 'Balanced' ? 'text-warning' : 'text-negative'
  );
</script>

<button
  {onclick}
  class="text-left p-4 rounded-lg border transition-all duration-200 hover:bg-surface-raised"
  class:border-gold={selected}
  class:bg-surface-raised={selected}
  class:border-border={!selected}
  class:bg-surface={!selected}
>
  <!-- Shield icon -->
  <div class="mb-3">
    <svg class="w-5 h-5 {shieldColor}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
      <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
    </svg>
  </div>

  <p class="text-ivory text-sm font-medium mb-1">{name}</p>
  <p class="text-ivory-muted text-xs leading-relaxed mb-3">{description}</p>

  <!-- Key metrics -->
  <div class="space-y-1.5">
    <div class="flex justify-between text-xs">
      <span class="text-ivory-muted">Max TX</span>
      <span class="text-ivory">{metrics.maxTx}</span>
    </div>
    <div class="flex justify-between text-xs">
      <span class="text-ivory-muted">Daily Loss</span>
      <span class="text-ivory">{metrics.dailyLoss}</span>
    </div>
    <div class="flex justify-between text-xs">
      <span class="text-ivory-muted">Confirm</span>
      <span class="text-ivory">{metrics.confirmation}</span>
    </div>
  </div>
</button>
