<script lang="ts">
  // Portfolio data — populated via Tauri IPC from portfolio.rs after setup
  // Shows empty state until container is running and data is available
  let portfolio = $state<{totalValue: number, change24h: number, changeAmount: number} | null>(null);
  let positions = $state<{asset: string, protocol: string, type: string, amount: string, value: string, apy: string}[]>([]);
  let allocation = $state<{asset: string, pct: number, color: string}[]>([]);
  let activity = $state<{time: string, action: string, protocol: string, hash: string}[]>([]);
  let health = $state<{burrowHealthFactor: number, guardrailsActive: boolean, dailyLoss: number, dailyLossLimit: number} | null>(null);

  function formatUsd(n: number) {
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', minimumFractionDigits: 2 }).format(n);
  }
</script>

<div class="h-full overflow-y-auto px-10 py-8">
  {#if !portfolio}
    <!-- Empty state — no data yet -->
    <div class="flex flex-col items-center justify-center h-full min-h-[500px] text-center">
      <div class="font-display text-5xl font-light tracking-wider text-ivory/10 mb-6">Nyx</div>
      <p class="text-ivory-muted/40 text-sm mb-2">Your private AI chief of staff</p>
      <p class="text-ivory-muted/30 text-xs max-w-sm">Built on <span class="text-gold/40">OpenClaw</span>. Complete setup to connect your communications, calendars and wallets. Seamless cross-chain transactions — including from shielded ZEC — powered by NEAR Intents. Your data stays local, your trail stays clean.</p>
      <a href="/setup" class="mt-8 px-6 py-2.5 border border-gold/40 text-gold/60 text-xs tracking-widest uppercase hover:border-gold hover:text-gold transition-colors duration-300 rounded">
        Run Setup
      </a>
    </div>
  {:else}
    <!-- Portfolio Value -->
    <section class="mb-12">
      <p class="text-ivory-muted text-sm tracking-widest uppercase mb-2 font-body">Portfolio Value</p>
      <h1 class="font-display text-5xl font-light tracking-wide text-ivory">
        {formatUsd(portfolio.totalValue)}
      </h1>
      <div class="flex items-center gap-3 mt-2">
        <span class="text-sm" class:text-positive={portfolio.change24h >= 0} class:text-negative={portfolio.change24h < 0}>
          {portfolio.change24h >= 0 ? '+' : ''}{portfolio.change24h.toFixed(2)}%
        </span>
        <span class="text-ivory-muted text-sm">
          {portfolio.changeAmount >= 0 ? '+' : ''}{formatUsd(portfolio.changeAmount)} today
        </span>
      </div>
    </section>

    <!-- Health Indicators -->
    {#if health}
      <section class="flex gap-8 mb-10">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-positive"></div>
          <span class="text-ivory-muted text-xs tracking-wide">Health Factor {health.burrowHealthFactor.toFixed(2)}</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-positive"></div>
          <span class="text-ivory-muted text-xs tracking-wide">Guardrails Active</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-positive"></div>
          <span class="text-ivory-muted text-xs tracking-wide">Daily Loss {health.dailyLoss.toFixed(1)}% / {health.dailyLossLimit.toFixed(1)}%</span>
        </div>
      </section>
    {/if}

    <div class="grid grid-cols-3 gap-8 mb-10">
      <!-- Positions -->
      <section class="col-span-2">
        <h2 class="text-ivory-muted text-xs tracking-widest uppercase mb-4">Positions</h2>
        {#if positions.length > 0}
          <div class="bg-surface rounded-lg overflow-hidden">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-border">
                  <th class="text-left py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">Asset</th>
                  <th class="text-left py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">Protocol</th>
                  <th class="text-left py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">Type</th>
                  <th class="text-right py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">Amount</th>
                  <th class="text-right py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">Value</th>
                  <th class="text-right py-3 px-4 text-ivory-muted font-normal text-xs tracking-wider">APY</th>
                </tr>
              </thead>
              <tbody>
                {#each positions as pos}
                  <tr class="border-b border-border/50 last:border-0 transition-colors duration-200 hover:bg-surface-raised">
                    <td class="py-3 px-4 text-ivory font-normal">{pos.asset}</td>
                    <td class="py-3 px-4 text-ivory-muted">{pos.protocol}</td>
                    <td class="py-3 px-4 text-ivory-muted">{pos.type}</td>
                    <td class="py-3 px-4 text-right font-mono text-ivory">{pos.amount}</td>
                    <td class="py-3 px-4 text-right text-ivory">{pos.value}</td>
                    <td class="py-3 px-4 text-right" class:text-gold={pos.apy !== '—'} class:text-ivory-muted={pos.apy === '—'}>{pos.apy}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="bg-surface rounded-lg p-8 text-center">
            <p class="text-ivory-muted/40 text-sm">No positions yet</p>
          </div>
        {/if}
      </section>

      <!-- Allocation -->
      <section>
        <h2 class="text-ivory-muted text-xs tracking-widest uppercase mb-4">Allocation</h2>
        {#if allocation.length > 0}
          <div class="bg-surface rounded-lg p-4 space-y-4">
            {#each allocation as alloc}
              <div>
                <div class="flex justify-between text-xs mb-1.5">
                  <span class="text-ivory">{alloc.asset}</span>
                  <span class="text-ivory-muted">{alloc.pct}%</span>
                </div>
                <div class="h-1 bg-border rounded-full overflow-hidden">
                  <div class="{alloc.color} h-full rounded-full transition-all duration-500" style="width: {alloc.pct}%"></div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="bg-surface rounded-lg p-8 text-center">
            <p class="text-ivory-muted/40 text-xs">No data</p>
          </div>
        {/if}
      </section>
    </div>

    <!-- Privacy Shield Status -->
    <section class="mb-10">
      <a href="/privacy" class="block bg-surface border border-border rounded-xl p-5 hover:border-gold/30 transition-all duration-300 group">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <div class="w-10 h-10 rounded-lg bg-gold/10 border border-gold/20 flex items-center justify-center">
              <svg class="w-5 h-5 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                <path d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
              </svg>
            </div>
            <div>
              <h3 class="text-ivory text-sm font-medium">Privacy Shield</h3>
              <p class="text-ivory-muted/50 text-xs mt-0.5">Convert to shielded ZEC for cryptographic privacy</p>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-gold/60 text-xs tracking-wider uppercase group-hover:text-gold transition-colors">Shield Assets</span>
            <svg class="w-4 h-4 text-ivory-muted/30 group-hover:text-gold/60 transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
            </svg>
          </div>
        </div>
      </a>
    </section>

    <!-- Activity -->
    <section>
      <h2 class="text-ivory-muted text-xs tracking-widest uppercase mb-4">Recent Activity</h2>
      {#if activity.length > 0}
        <div class="space-y-0">
          {#each activity as tx}
            <div class="flex items-center justify-between py-3 border-b border-border/30 last:border-0">
              <div class="flex items-center gap-4">
                <span class="text-ivory-muted text-xs w-16">{tx.time}</span>
                <span class="text-ivory text-sm">{tx.action}</span>
                <span class="text-ivory-muted text-xs">{tx.protocol}</span>
              </div>
              <span class="font-mono text-xs text-ivory-muted selectable">{tx.hash}</span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="py-6 text-center">
          <p class="text-ivory-muted/40 text-sm">No activity yet</p>
        </div>
      {/if}
    </section>
  {/if}
</div>
