<script lang="ts">
  let { condensed = false }: { condensed?: boolean } = $props();

  const capabilities = [
    { abbr: 'Fi', name: 'DeFi & Crypto', description: 'Cross-chain swaps, staking, lending, leveraged looping, portfolio management', color: 'bg-emerald-500/15 text-emerald-400' },
    { abbr: 'Co', name: 'Communications', description: 'Telegram, WhatsApp, Slack, Signal with autonomy controls', color: 'bg-purple-500/15 text-purple-400' },
    { abbr: 'Gw', name: 'Google Workspace', description: 'Gmail, Calendar, Drive, Docs integration', color: 'bg-red-500/15 text-red-400' },
    { abbr: 'Em', name: 'Email Intelligence', description: 'Priority triage, daily digest, inbox awareness', color: 'bg-amber-500/15 text-amber-300' },
    { abbr: 'Tr', name: 'Travel', description: 'Flight, hotel, and transport research', color: 'bg-blue-400/15 text-blue-300' },
    { abbr: 'Ve', name: 'Source Verification', description: 'Credibility analysis and fact-checking', color: 'bg-cyan-500/15 text-cyan-300' },
  ];
</script>

{#if condensed}
  <!-- Compact 2-column grid for chat empty state -->
  <div class="grid grid-cols-2 gap-2 max-w-sm">
    {#each capabilities as cap}
      <div class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-surface/50">
        <div class="w-5 h-5 rounded flex items-center justify-center text-[9px] font-bold flex-shrink-0 {cap.color}">
          {cap.abbr}
        </div>
        <span class="text-ivory-muted/60 text-[11px] truncate">{cap.name}</span>
      </div>
    {/each}
  </div>
{:else}
  <!-- Full layout for setup completion -->
  <div class="grid gap-2">
    {#each capabilities as cap}
      <div class="flex items-center gap-3 px-3 py-2.5 rounded-lg bg-surface border border-border/50">
        <div class="w-7 h-7 rounded-lg flex items-center justify-center text-[10px] font-bold flex-shrink-0 {cap.color}">
          {cap.abbr}
        </div>
        <div class="min-w-0">
          <div class="text-ivory text-xs font-medium">{cap.name}</div>
          <div class="text-ivory-muted/50 text-[10px] truncate">{cap.description}</div>
        </div>
      </div>
    {/each}
  </div>
{/if}
