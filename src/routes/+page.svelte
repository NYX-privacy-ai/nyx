<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';

  // Portfolio data — populated via Tauri IPC from portfolio.rs after setup
  // Shows empty state until container is running and data is available
  let portfolio = $state<{totalValue: number, change24h: number, changeAmount: number} | null>(null);
  let positions = $state<{asset: string, protocol: string, type: string, amount: string, value: string, apy: string}[]>([]);
  let allocation = $state<{asset: string, pct: number, color: string}[]>([]);
  let activity = $state<{time: string, action: string, protocol: string, hash: string}[]>([]);
  let health = $state<{burrowHealthFactor: number, guardrailsActive: boolean, dailyLoss: number, dailyLossLimit: number} | null>(null);

  // Activity Intelligence
  let intelligenceEnabled = $state(false);
  let activityStats = $state<{
    contacts_tracked: number,
    emails_observed_24h: number,
    calendar_events_7d: number,
    suggestions_pending: number,
    top_contacts: {email: string, name: string | null, interaction_count: number, last_seen: string}[],
    last_observation: string | null,
  } | null>(null);
  let suggestions = $state<{
    id: number,
    type: string,
    title: string,
    description: string,
    contact_email: string | null,
    confidence: number,
    status: string,
    created_at: string,
  }[]>([]);
  let unlistenIntelligence: (() => void) | null = null;
  let unlistenSuggestions: (() => void) | null = null;

  function formatUsd(n: number) {
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', minimumFractionDigits: 2 }).format(n);
  }

  function timeAgo(iso: string): string {
    try {
      const diff = Date.now() - new Date(iso).getTime();
      const mins = Math.floor(diff / 60000);
      if (mins < 1) return 'just now';
      if (mins < 60) return `${mins}m ago`;
      const hours = Math.floor(mins / 60);
      if (hours < 24) return `${hours}h ago`;
      const days = Math.floor(hours / 24);
      return `${days}d ago`;
    } catch {
      return '';
    }
  }

  function suggestionIcon(type: string): string {
    switch (type) {
      case 'respond': return 'M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75';
      case 'schedule_meeting': return 'M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5';
      case 'reachout': return 'M8.625 12a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H8.25m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H12m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0h-.375M21 12c0 4.556-4.03 8.25-9 8.25a9.764 9.764 0 01-2.555-.337A5.972 5.972 0 015.41 20.97a5.969 5.969 0 01-.474-.065 4.48 4.48 0 00.978-2.025c.09-.457-.133-.901-.467-1.226C3.93 16.178 3 14.189 3 12c0-4.556 4.03-8.25 9-8.25s9 3.694 9 8.25z';
      case 'catch_up': return 'M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z';
      default: return 'M12 18v-5.25m0 0a6.01 6.01 0 001.5-.189m-1.5.189a6.01 6.01 0 01-1.5-.189m3.75 7.478a12.06 12.06 0 01-4.5 0m3.75 2.355a7.5 7.5 0 01-3 0m3-6.566V12a7.5 7.5 0 10-15 0v2.034';
    }
  }

  async function loadIntelligence() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const config: any = await invoke('read_current_config');
      intelligenceEnabled = config?.capabilities?.activity_intelligence ?? false;

      if (intelligenceEnabled) {
        const [stats, suggs]: any = await Promise.all([
          invoke('get_activity_stats'),
          invoke('get_intelligence_suggestions'),
        ]);
        activityStats = stats;
        suggestions = suggs || [];
      }
    } catch {
      // Intelligence not available — silently ignore
    }
  }

  async function dismissSuggestion(id: number) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('dismiss_intelligence_suggestion', { id });
      suggestions = suggestions.filter(s => s.id !== id);
    } catch {}
  }

  async function acceptSuggestion(id: number, type: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('accept_intelligence_suggestion', { id });
      suggestions = suggestions.filter(s => s.id !== id);

      // Navigate based on suggestion type
      if (type === 'respond' || type === 'catch_up' || type === 'reachout') {
        goto('/chat');
      } else if (type === 'schedule_meeting') {
        goto('/chat');
      }
    } catch {}
  }

  async function loadPortfolio() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const data: any = await invoke('get_portfolio');
      if (data) {
        portfolio = {
          totalValue: data.total_value ?? 0,
          change24h: data.change_24h ?? 0,
          changeAmount: data.change_amount ?? 0,
        };
        positions = (data.positions ?? []).map((p: any) => ({
          asset: p.asset, protocol: p.protocol, type: p.type,
          amount: p.amount, value: p.value, apy: p.apy ?? '—',
        }));
        allocation = data.allocation ?? [];
        activity = data.activity ?? [];
        health = data.health ?? null;
      }
    } catch {
      // Portfolio not available (container not running or no data yet)
    }
  }

  onMount(async () => {
    await Promise.all([loadPortfolio(), loadIntelligence()]);

    // Listen for intelligence events
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlistenIntelligence = await listen('intelligence:update', () => {
        loadIntelligence();
      }) as unknown as () => void;
      unlistenSuggestions = await listen('intelligence:suggestions', () => {
        loadIntelligence();
      }) as unknown as () => void;
    } catch {}
  });

  onDestroy(() => {
    unlistenIntelligence?.();
    unlistenSuggestions?.();
  });
</script>

<div class="h-full overflow-y-auto px-10 py-8">
  {#if !portfolio}
    <!-- Empty state — no data yet -->
    <div class="flex flex-col items-center justify-center {intelligenceEnabled ? 'min-h-[300px]' : 'h-full min-h-[500px]'} text-center">
      <div class="font-display text-5xl font-light tracking-wider text-ivory/10 mb-6">Nyx</div>
      <p class="text-ivory-muted/40 text-sm mb-2">Your private AI chief of staff</p>
      <p class="text-ivory-muted/30 text-xs max-w-sm">Built on <span class="text-gold/40">OpenClaw</span>. Complete setup to connect your communications, calendars and wallets. Seamless cross-chain transactions — including from shielded ZEC — powered by NEAR Intents. Your data stays local, your trail stays clean.</p>
      <a href="/setup" class="mt-8 px-6 py-2.5 border border-gold/40 text-gold/60 text-xs tracking-widest uppercase hover:border-gold hover:text-gold transition-colors duration-300 rounded">
        Run Setup
      </a>
    </div>

    <!-- Intelligence section in empty state -->
    {#if intelligenceEnabled && activityStats}
      {@render intelligenceSection()}
    {/if}
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

    <!-- Intelligence section in portfolio view -->
    {#if intelligenceEnabled && activityStats}
      {@render intelligenceSection()}
    {/if}
  {/if}
</div>

<!-- Reusable Intelligence section snippet -->
{#snippet intelligenceSection()}
  <section class="mt-10">
    <h2 class="text-ivory-muted text-xs tracking-widest uppercase mb-5 flex items-center gap-2">
      <svg class="w-3.5 h-3.5 text-rose-300/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M12 18v-5.25m0 0a6.01 6.01 0 001.5-.189m-1.5.189a6.01 6.01 0 01-1.5-.189m3.75 7.478a12.06 12.06 0 01-4.5 0m3.75 2.355a7.5 7.5 0 01-3 0m3-6.566V12a7.5 7.5 0 10-15 0v2.034" />
      </svg>
      Intelligence
    </h2>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Activity Stats -->
      {#if activityStats}
        <div class="bg-surface rounded-xl border border-border/50 p-5">
          <h3 class="text-ivory-muted text-[10px] tracking-widest uppercase mb-4">Activity</h3>
          <div class="space-y-3">
            <div class="flex justify-between items-center">
              <span class="text-ivory-muted/60 text-xs">Contacts</span>
              <span class="text-ivory text-sm font-light">{activityStats.contacts_tracked}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-ivory-muted/60 text-xs">Emails today</span>
              <span class="text-ivory text-sm font-light">{activityStats.emails_observed_24h}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="text-ivory-muted/60 text-xs">Events this week</span>
              <span class="text-ivory text-sm font-light">{activityStats.calendar_events_7d}</span>
            </div>
          </div>

          {#if activityStats.top_contacts.length > 0}
            <div class="mt-5 pt-4 border-t border-border/30">
              <h4 class="text-ivory-muted/40 text-[9px] tracking-widest uppercase mb-3">Top Contacts</h4>
              <div class="space-y-2">
                {#each activityStats.top_contacts.slice(0, 3) as contact}
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2 min-w-0">
                      <div class="w-5 h-5 rounded-full bg-rose-500/10 flex items-center justify-center flex-shrink-0">
                        <span class="text-rose-300/60 text-[8px] font-bold uppercase">
                          {(contact.name || contact.email).charAt(0)}
                        </span>
                      </div>
                      <span class="text-ivory text-xs truncate">{contact.name || contact.email.split('@')[0]}</span>
                    </div>
                    <span class="text-ivory-muted/40 text-[10px] flex-shrink-0 ml-2">{contact.interaction_count}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if activityStats.last_observation}
            <div class="mt-4 pt-3 border-t border-border/20">
              <span class="text-ivory-muted/25 text-[9px]">Last sync: {timeAgo(activityStats.last_observation)}</span>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Suggestions -->
      <div class="lg:col-span-2">
        {#if suggestions.length > 0}
          <div class="space-y-3">
            {#each suggestions.slice(0, 4) as suggestion}
              <div class="bg-surface rounded-xl border border-border/50 p-4 group hover:border-rose-500/20 transition-colors duration-200">
                <div class="flex items-start gap-3">
                  <div class="w-8 h-8 rounded-lg bg-rose-500/10 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <svg class="w-4 h-4 text-rose-300/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d={suggestionIcon(suggestion.type)} />
                    </svg>
                  </div>
                  <div class="flex-1 min-w-0">
                    <h4 class="text-ivory text-sm font-medium">{suggestion.title}</h4>
                    <p class="text-ivory-muted/50 text-xs mt-1 leading-relaxed">{suggestion.description}</p>
                    <div class="flex items-center gap-3 mt-3">
                      <button
                        onclick={() => acceptSuggestion(suggestion.id, suggestion.type)}
                        class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-rose-500/30 text-rose-300/70 hover:text-rose-200 hover:border-rose-400/50 hover:bg-rose-500/10 transition-all duration-200"
                      >
                        {suggestion.type === 'respond' ? 'Draft Reply' :
                         suggestion.type === 'schedule_meeting' ? 'Schedule' :
                         suggestion.type === 'reachout' ? 'Respond' : 'Take Action'}
                      </button>
                      <button
                        onclick={() => dismissSuggestion(suggestion.id)}
                        class="px-3 py-1 text-[10px] tracking-wider uppercase rounded text-ivory-muted/30 hover:text-ivory-muted/60 transition-colors"
                      >
                        Dismiss
                      </button>
                      <span class="ml-auto text-ivory-muted/20 text-[9px]">{timeAgo(suggestion.created_at)}</span>
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="bg-surface rounded-xl border border-border/50 p-8 text-center h-full flex flex-col items-center justify-center">
            <svg class="w-8 h-8 text-ivory-muted/15 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
              <path d="M12 18v-5.25m0 0a6.01 6.01 0 001.5-.189m-1.5.189a6.01 6.01 0 01-1.5-.189m3.75 7.478a12.06 12.06 0 01-4.5 0m3.75 2.355a7.5 7.5 0 01-3 0m3-6.566V12a7.5 7.5 0 10-15 0v2.034" />
            </svg>
            <p class="text-ivory-muted/30 text-xs">No suggestions yet</p>
            <p class="text-ivory-muted/20 text-[10px] mt-1">Nyx is observing patterns in your calendar and email</p>
          </div>
        {/if}
      </div>
    </div>
  </section>
{/snippet}
