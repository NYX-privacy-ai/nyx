<script lang="ts">
  import { onMount } from 'svelte';

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

  interface ShieldableAsset {
    chain: string;
    symbol: string;
    name: string;
    asset_id: string;
    decimals: number;
    icon: string;
  }

  interface QuoteDetails {
    amountIn: string;
    amountInFormatted: string;
    amountInUsd: string;
    minAmountIn: string;
    amountOut: string;
    amountOutFormatted: string;
    amountOutUsd: string;
    minAmountOut: string;
    timeEstimate: number;
  }

  interface QuoteResponse {
    quote: QuoteDetails;
    signature: string | null;
    timestamp: string | null;
    correlationId: string | null;
  }

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let assets = $state<ShieldableAsset[]>([]);
  let loading = $state(true);
  let isTauri = $state(false);

  // Tab state
  let activeTab = $state<'zec' | 'confidential'>('zec');

  // Shield (to ZEC)
  let shieldAsset = $state<string>('');
  let shieldAmount = $state('');
  let shieldQuote = $state<QuoteResponse | null>(null);
  let shieldLoading = $state(false);
  let shieldError = $state('');

  // Unshield (from ZEC)
  let unshieldAsset = $state<string>('');
  let unshieldAmount = $state('');
  let unshieldRecipient = $state('');
  let unshieldQuote = $state<QuoteResponse | null>(null);
  let unshieldLoading = $state(false);
  let unshieldError = $state('');

  // Execution state
  let shieldExecuting = $state(false);
  let shieldSuccess = $state('');
  let unshieldExecuting = $state(false);
  let unshieldSuccess = $state('');

  // Confidential swap state
  let confOriginAsset = $state<string>('');
  let confDestAsset = $state<string>('');
  let confAmount = $state('');
  let confQuote = $state<QuoteResponse | null>(null);
  let confLoading = $state(false);
  let confError = $state('');
  let confExecuting = $state(false);
  let confSuccess = $state('');

  // How it works — expanded
  let howItWorksOpen = $state(false);

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function formatUsd(s: string): string {
    const n = parseFloat(s);
    if (isNaN(n)) return '$0.00';
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(n);
  }

  function formatTime(seconds: number): string {
    if (seconds < 60) return `~${seconds}s`;
    const mins = Math.round(seconds / 60);
    return `~${mins} min`;
  }

  // ---------------------------------------------------------------------------
  // Init
  // ---------------------------------------------------------------------------

  onMount(async () => {
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      isTauri = true;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const list = await invoke('get_shieldable_assets') as ShieldableAsset[];
        assets = list;
        if (list.length > 0) {
          shieldAsset = list[0].asset_id;
          confOriginAsset = list[0].asset_id;
          // Default unshield/conf dest to first non-ZEC asset
          const nonZec = list.find(a => a.symbol !== 'ZEC');
          unshieldAsset = nonZec ? nonZec.asset_id : list[0].asset_id;
          confDestAsset = list.length > 1 ? list[1].asset_id : list[0].asset_id;
        }
      } catch (e: any) {
        console.error('Failed to load shieldable assets:', e);
      }
    }
    loading = false;
  });

  // ---------------------------------------------------------------------------
  // Shield quote
  // ---------------------------------------------------------------------------

  async function getShieldQuote() {
    if (!shieldAmount || !shieldAsset) return;
    shieldLoading = true;
    shieldError = '';
    shieldQuote = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const asset = assets.find(a => a.asset_id === shieldAsset);
      const decimals = asset?.decimals ?? 18;
      const parts = shieldAmount.split('.');
      const intPart = parts[0] || '0';
      const fracPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);
      const raw = (BigInt(intPart) * (BigInt(10) ** BigInt(decimals)) + BigInt(fracPart)).toString();
      const result = await invoke('get_zec_shield_quote', {
        fromAsset: shieldAsset,
        amount: raw,
      }) as QuoteResponse;
      shieldQuote = result;
    } catch (e: any) {
      shieldError = typeof e === 'string' ? e : e?.message || 'Quote failed';
    } finally {
      shieldLoading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Shield / Unshield execution
  // ---------------------------------------------------------------------------

  async function executeShield() {
    if (!shieldQuote) return;
    shieldExecuting = true;
    shieldError = '';
    shieldSuccess = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const asset = assets.find(a => a.asset_id === shieldAsset);
      const decimals = asset?.decimals ?? 18;
      const parts = shieldAmount.split('.');
      const intPart = parts[0] || '0';
      const fracPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);
      const raw = (BigInt(intPart) * (BigInt(10) ** BigInt(decimals)) + BigInt(fracPart)).toString();
      await invoke('execute_zec_shield', {
        fromAsset: shieldAsset,
        amount: raw,
      });
      shieldSuccess = 'Shield transaction submitted successfully';
      shieldQuote = null;
      shieldAmount = '';
    } catch (e: any) {
      shieldError = typeof e === 'string' ? e : e?.message || 'Shield failed';
    } finally {
      shieldExecuting = false;
    }
  }

  async function executeUnshield() {
    if (!unshieldQuote) return;
    unshieldExecuting = true;
    unshieldError = '';
    unshieldSuccess = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [whole = '0', frac = ''] = unshieldAmount.split('.');
      const raw = (BigInt(whole) * BigInt(1e8) + BigInt((frac + '00000000').slice(0, 8))).toString();
      await invoke('execute_zec_unshield', {
        toAsset: unshieldAsset,
        zecAmount: raw,
        recipient: unshieldRecipient,
      });
      unshieldSuccess = 'Unshield transaction submitted successfully';
      unshieldQuote = null;
      unshieldAmount = '';
      unshieldRecipient = '';
    } catch (e: any) {
      unshieldError = typeof e === 'string' ? e : e?.message || 'Unshield failed';
    } finally {
      unshieldExecuting = false;
    }
  }

  async function getUnshieldQuote() {
    if (!unshieldAmount || !unshieldAsset || !unshieldRecipient) return;
    unshieldLoading = true;
    unshieldError = '';
    unshieldQuote = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [whole = '0', frac = ''] = unshieldAmount.split('.');
      const raw = (BigInt(whole) * BigInt(1e8) + BigInt((frac + '00000000').slice(0, 8))).toString();
      const result = await invoke('get_zec_unshield_quote', {
        toAsset: unshieldAsset,
        zecAmount: raw,
        recipient: unshieldRecipient,
      }) as QuoteResponse;
      unshieldQuote = result;
    } catch (e: any) {
      unshieldError = typeof e === 'string' ? e : e?.message || 'Quote failed';
    } finally {
      unshieldLoading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Confidential swap
  // ---------------------------------------------------------------------------

  async function getConfidentialQuote() {
    if (!confAmount || !confOriginAsset || !confDestAsset) return;
    confLoading = true;
    confError = '';
    confQuote = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const asset = assets.find(a => a.asset_id === confOriginAsset);
      const decimals = asset?.decimals ?? 18;
      const parts = confAmount.split('.');
      const intPart = parts[0] || '0';
      const fracPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);
      const raw = (BigInt(intPart) * (BigInt(10) ** BigInt(decimals)) + BigInt(fracPart)).toString();
      const result = await invoke('get_confidential_quote', {
        originAsset: confOriginAsset,
        destinationAsset: confDestAsset,
        amount: raw,
      }) as QuoteResponse;
      confQuote = result;
    } catch (e: any) {
      confError = typeof e === 'string' ? e : e?.message || 'Quote failed';
    } finally {
      confLoading = false;
    }
  }

  async function executeConfidentialSwap() {
    if (!confQuote) return;
    confExecuting = true;
    confError = '';
    confSuccess = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const asset = assets.find(a => a.asset_id === confOriginAsset);
      const decimals = asset?.decimals ?? 18;
      const parts = confAmount.split('.');
      const intPart = parts[0] || '0';
      const fracPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);
      const raw = (BigInt(intPart) * (BigInt(10) ** BigInt(decimals)) + BigInt(fracPart)).toString();
      await invoke('execute_confidential_swap', {
        originAsset: confOriginAsset,
        destinationAsset: confDestAsset,
        amount: raw,
      });
      confSuccess = 'Confidential swap submitted successfully';
      confQuote = null;
      confAmount = '';
    } catch (e: any) {
      confError = typeof e === 'string' ? e : e?.message || 'Swap failed';
    } finally {
      confExecuting = false;
    }
  }
</script>

<div class="h-full overflow-y-auto px-10 py-8">
  <!-- Page header -->
  <div class="flex items-center gap-4 mb-8">
    <div class="w-12 h-12 rounded-xl bg-gold/10 border border-gold/20 flex items-center justify-center">
      <svg class="w-6 h-6 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
      </svg>
    </div>
    <div>
      <h1 class="font-display text-3xl font-light tracking-wide text-ivory">Privacy</h1>
      <p class="text-ivory-muted/60 text-sm mt-1">Privacy tools powered by NEAR Intents</p>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center h-64">
      <div class="w-6 h-6 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></div>
    </div>
  {:else}

    <!-- Tab Navigation -->
    <div class="flex gap-1 mb-8 bg-surface border border-border rounded-xl p-1">
      <button
        onclick={() => activeTab = 'zec'}
        class="flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg text-sm font-medium tracking-wide transition-all duration-300
          {activeTab === 'zec'
            ? 'bg-gold/10 border border-gold/30 text-gold'
            : 'text-ivory-muted/50 hover:text-ivory-muted/80 border border-transparent'}"
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
        </svg>
        ZEC Shield
      </button>
      <button
        onclick={() => activeTab = 'confidential'}
        class="flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg text-sm font-medium tracking-wide transition-all duration-300
          {activeTab === 'confidential'
            ? 'bg-gold/10 border border-gold/30 text-gold'
            : 'text-ivory-muted/50 hover:text-ivory-muted/80 border border-transparent'}"
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
        </svg>
        Confidential Swap
        <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-gold/20 text-gold/80 font-normal">NEW</span>
      </button>
    </div>

    <!-- TAB: ZEC Shield -->
    {#if activeTab === 'zec'}
      <div class="bg-surface border border-gold/10 rounded-xl p-6 mb-10 animate-fade-in">
        <div class="flex items-start gap-4">
          <div class="w-10 h-10 rounded-lg bg-gold/10 flex items-center justify-center shrink-0 mt-0.5">
            <svg class="w-5 h-5 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
          </div>
          <div>
            <h3 class="text-ivory font-medium text-sm mb-1">Why Shielded ZEC?</h3>
            <p class="text-ivory-muted/60 text-xs leading-relaxed">
              Most blockchains are pseudonymous — all transactions are visible on-chain. Zcash provides
              <span class="text-gold/80">cryptographic privacy</span> using zero-knowledge proofs, making your holdings
              and transactions invisible to chain analysis. Convert any crypto to shielded ZEC with one click,
              and convert back when you need to transact — powered by the NEAR Intents cross-chain network.
            </p>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-10 animate-fade-in">
        <!-- SHIELD: Convert TO ZEC -->
        <section class="bg-surface border border-border rounded-xl p-6">
          <div class="flex items-center gap-3 mb-6">
            <div class="w-8 h-8 rounded-lg bg-positive/10 flex items-center justify-center">
              <svg class="w-4 h-4 text-positive" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M12 4.5v15m7.5-7.5h-15" /></svg>
            </div>
            <div>
              <h2 class="text-ivory text-base font-medium">Shield Your Assets</h2>
              <p class="text-ivory-muted/50 text-xs">Convert to shielded ZEC</p>
            </div>
          </div>
          <label class="block mb-4">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">From Asset</span>
            <select bind:value={shieldAsset} class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm focus:border-gold/50 focus:outline-none transition-colors">
              {#each assets as a}<option value={a.asset_id}>{a.icon} {a.symbol} — {a.name} ({a.chain.toUpperCase()})</option>{/each}
            </select>
          </label>
          <label class="block mb-5">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">Amount</span>
            <div class="relative">
              <input type="number" step="any" min="0" bind:value={shieldAmount} placeholder="0.00" class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm font-mono focus:border-gold/50 focus:outline-none transition-colors pr-16" />
              <span class="absolute right-4 top-1/2 -translate-y-1/2 text-ivory-muted/40 text-xs">{assets.find(a => a.asset_id === shieldAsset)?.symbol ?? ''}</span>
            </div>
          </label>
          <button onclick={getShieldQuote} disabled={shieldLoading || !shieldAmount || !shieldAsset} class="w-full py-2.5 rounded-lg text-sm font-medium tracking-wider uppercase transition-all duration-300 disabled:opacity-30 disabled:cursor-not-allowed {shieldLoading ? 'bg-gold/10 text-gold/60 border border-gold/20' : 'bg-gold/10 border border-gold/40 text-gold hover:bg-gold/20 hover:border-gold'}">
            {#if shieldLoading}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Routing via NEAR Intents...</span>{:else}Get Quote{/if}
          </button>
          {#if shieldError}<div class="mt-4 p-3 bg-negative/10 border border-negative/20 rounded-lg"><p class="text-negative text-xs">{shieldError}</p></div>{/if}
          {#if shieldQuote}
            <div class="mt-5 p-4 bg-black/50 border border-gold/15 rounded-lg space-y-3">
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Send</span><span class="text-ivory text-sm font-mono">{shieldQuote.quote.amountInFormatted || shieldQuote.quote.amountIn}<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(shieldQuote.quote.amountInUsd)}</span></span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Receive</span><span class="text-gold text-sm font-mono font-medium">{shieldQuote.quote.amountOutFormatted || shieldQuote.quote.amountOut} ZEC<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(shieldQuote.quote.amountOutUsd)}</span></span></div>
              {#if shieldQuote.quote.minAmountOut}<div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Min. Received</span><span class="text-ivory-muted text-xs font-mono">{shieldQuote.quote.minAmountOut} ZEC</span></div>{/if}
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Est. Time</span><span class="text-ivory-muted text-xs">{formatTime(shieldQuote.quote.timeEstimate)}</span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Slippage</span><span class="text-ivory-muted text-xs">1% max</span></div>
              <div class="border-t border-border/30 pt-3 mt-3">
                <button onclick={executeShield} disabled={shieldExecuting} class="w-full py-2.5 bg-gold/20 border border-gold text-gold text-sm font-medium tracking-wider uppercase rounded-lg hover:bg-gold/30 transition-all duration-300 disabled:opacity-40 disabled:cursor-not-allowed">
                  {#if shieldExecuting}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Shielding...</span>{:else}Shield Now{/if}
                </button>
                {#if shieldSuccess}<p class="text-positive text-xs mt-2">{shieldSuccess}</p>{/if}
              </div>
            </div>
          {/if}
        </section>

        <!-- UNSHIELD: Convert FROM ZEC -->
        <section class="bg-surface border border-border rounded-xl p-6">
          <div class="flex items-center gap-3 mb-6">
            <div class="w-8 h-8 rounded-lg bg-gold/10 flex items-center justify-center">
              <svg class="w-4 h-4 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M3 7.5L7.5 3m0 0L12 7.5M7.5 3v13.5m13.5 0L16.5 21m0 0L12 16.5m4.5 4.5V7.5" /></svg>
            </div>
            <div>
              <h2 class="text-ivory text-base font-medium">Spend from Shield</h2>
              <p class="text-ivory-muted/50 text-xs">Convert shielded ZEC to any asset</p>
            </div>
          </div>
          <label class="block mb-4">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">To Asset</span>
            <select bind:value={unshieldAsset} class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm focus:border-gold/50 focus:outline-none transition-colors">
              {#each assets.filter(a => a.symbol !== 'ZEC') as a}<option value={a.asset_id}>{a.icon} {a.symbol} — {a.name} ({a.chain.toUpperCase()})</option>{/each}
            </select>
          </label>
          <label class="block mb-4">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">ZEC Amount</span>
            <div class="relative">
              <input type="number" step="any" min="0" bind:value={unshieldAmount} placeholder="0.00" class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm font-mono focus:border-gold/50 focus:outline-none transition-colors pr-16" />
              <span class="absolute right-4 top-1/2 -translate-y-1/2 text-ivory-muted/40 text-xs">ZEC</span>
            </div>
          </label>
          <label class="block mb-5">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">Recipient Address</span>
            <input type="text" bind:value={unshieldRecipient} placeholder="Destination wallet address" class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm font-mono focus:border-gold/50 focus:outline-none transition-colors" />
            <p class="text-ivory-muted/30 text-xs mt-1">Address on the destination chain</p>
          </label>
          <button onclick={getUnshieldQuote} disabled={unshieldLoading || !unshieldAmount || !unshieldAsset || !unshieldRecipient} class="w-full py-2.5 rounded-lg text-sm font-medium tracking-wider uppercase transition-all duration-300 disabled:opacity-30 disabled:cursor-not-allowed {unshieldLoading ? 'bg-gold/10 text-gold/60 border border-gold/20' : 'bg-gold/10 border border-gold/40 text-gold hover:bg-gold/20 hover:border-gold'}">
            {#if unshieldLoading}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Routing via NEAR Intents...</span>{:else}Get Quote{/if}
          </button>
          {#if unshieldError}<div class="mt-4 p-3 bg-negative/10 border border-negative/20 rounded-lg"><p class="text-negative text-xs">{unshieldError}</p></div>{/if}
          {#if unshieldQuote}
            <div class="mt-5 p-4 bg-black/50 border border-gold/15 rounded-lg space-y-3">
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Send</span><span class="text-ivory text-sm font-mono">{unshieldQuote.quote.amountInFormatted || unshieldQuote.quote.amountIn} ZEC<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(unshieldQuote.quote.amountInUsd)}</span></span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Receive</span><span class="text-gold text-sm font-mono font-medium">{unshieldQuote.quote.amountOutFormatted || unshieldQuote.quote.amountOut}<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(unshieldQuote.quote.amountOutUsd)}</span></span></div>
              {#if unshieldQuote.quote.minAmountOut}<div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Min. Received</span><span class="text-ivory-muted text-xs font-mono">{unshieldQuote.quote.minAmountOut}</span></div>{/if}
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Est. Time</span><span class="text-ivory-muted text-xs">{formatTime(unshieldQuote.quote.timeEstimate)}</span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Slippage</span><span class="text-ivory-muted text-xs">1% max</span></div>
              <div class="border-t border-border/30 pt-3 mt-3">
                <button onclick={executeUnshield} disabled={unshieldExecuting} class="w-full py-2.5 bg-gold/20 border border-gold text-gold text-sm font-medium tracking-wider uppercase rounded-lg hover:bg-gold/30 transition-all duration-300 disabled:opacity-40 disabled:cursor-not-allowed">
                  {#if unshieldExecuting}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Converting...</span>{:else}Convert Now{/if}
                </button>
                {#if unshieldSuccess}<p class="text-positive text-xs mt-2">{unshieldSuccess}</p>{/if}
              </div>
            </div>
          {/if}
        </section>
      </div>
    {/if}

    <!-- TAB: Confidential Swap -->
    {#if activeTab === 'confidential'}
      <div class="bg-surface border border-gold/10 rounded-xl p-6 mb-10 animate-fade-in">
        <div class="flex items-start gap-4">
          <div class="w-10 h-10 rounded-lg bg-gold/10 flex items-center justify-center shrink-0 mt-0.5">
            <svg class="w-5 h-5 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
            </svg>
          </div>
          <div>
            <h3 class="text-ivory font-medium text-sm mb-1">NEAR Confidential Intents</h3>
            <p class="text-ivory-muted/60 text-xs leading-relaxed">
              Swap any asset across 25+ chains with <span class="text-gold/80">execution privacy</span>.
              Transactions execute inside a TEE-secured private shard — validators verify correctness
              without seeing your amounts, routes, or strategy. Prevents MEV extraction, frontrunning,
              and sandwich attacks. Same speed and cost as public swaps.
            </p>
          </div>
        </div>
      </div>

      <div class="max-w-xl mx-auto animate-fade-in">
        <section class="bg-surface border border-border rounded-xl p-6">
          <div class="flex items-center gap-3 mb-6">
            <div class="w-8 h-8 rounded-lg bg-gold/10 flex items-center justify-center">
              <svg class="w-4 h-4 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                <path d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
              </svg>
            </div>
            <div class="flex-1">
              <h2 class="text-ivory text-base font-medium">Confidential Swap</h2>
              <p class="text-ivory-muted/50 text-xs">TEE-protected cross-chain execution</p>
            </div>
            <span class="px-2.5 py-1 rounded-full bg-gold/10 border border-gold/20 text-gold text-[10px] font-medium tracking-wider uppercase">Private Shard</span>
          </div>

          <label class="block mb-4">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">From</span>
            <select bind:value={confOriginAsset} class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm focus:border-gold/50 focus:outline-none transition-colors">
              {#each assets as a}<option value={a.asset_id}>{a.icon} {a.symbol} — {a.name} ({a.chain.toUpperCase()})</option>{/each}
            </select>
          </label>

          <label class="block mb-4">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">Amount</span>
            <div class="relative">
              <input type="number" step="any" min="0" bind:value={confAmount} placeholder="0.00" class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm font-mono focus:border-gold/50 focus:outline-none transition-colors pr-16" />
              <span class="absolute right-4 top-1/2 -translate-y-1/2 text-ivory-muted/40 text-xs">{assets.find(a => a.asset_id === confOriginAsset)?.symbol ?? ''}</span>
            </div>
          </label>

          <div class="flex justify-center my-2">
            <div class="w-8 h-8 rounded-full bg-surface-raised border border-border flex items-center justify-center">
              <svg class="w-4 h-4 text-ivory-muted/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M19 14l-7 7m0 0l-7-7m7 7V3" /></svg>
            </div>
          </div>

          <label class="block mb-5">
            <span class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">To</span>
            <select bind:value={confDestAsset} class="w-full bg-black border border-border rounded-lg px-4 py-2.5 text-ivory text-sm focus:border-gold/50 focus:outline-none transition-colors">
              {#each assets as a}<option value={a.asset_id}>{a.icon} {a.symbol} — {a.name} ({a.chain.toUpperCase()})</option>{/each}
            </select>
          </label>

          <div class="flex items-center gap-2 mb-5 px-3 py-2 bg-gold/5 border border-gold/10 rounded-lg">
            <svg class="w-3.5 h-3.5 text-gold/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
            </svg>
            <span class="text-gold/60 text-xs">Execution via NEAR private shard — MEV &amp; frontrunning protected</span>
          </div>

          <button onclick={getConfidentialQuote} disabled={confLoading || !confAmount || !confOriginAsset || !confDestAsset || confOriginAsset === confDestAsset} class="w-full py-2.5 rounded-lg text-sm font-medium tracking-wider uppercase transition-all duration-300 disabled:opacity-30 disabled:cursor-not-allowed {confLoading ? 'bg-gold/10 text-gold/60 border border-gold/20' : 'bg-gold/10 border border-gold/40 text-gold hover:bg-gold/20 hover:border-gold'}">
            {#if confLoading}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Routing confidentially...</span>{:else}Get Confidential Quote{/if}
          </button>

          {#if confError}<div class="mt-4 p-3 bg-negative/10 border border-negative/20 rounded-lg"><p class="text-negative text-xs">{confError}</p></div>{/if}

          {#if confQuote}
            <div class="mt-5 p-4 bg-black/50 border border-gold/15 rounded-lg space-y-3">
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Send</span><span class="text-ivory text-sm font-mono">{confQuote.quote.amountInFormatted || confQuote.quote.amountIn}<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(confQuote.quote.amountInUsd)}</span></span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">You Receive</span><span class="text-gold text-sm font-mono font-medium">{confQuote.quote.amountOutFormatted || confQuote.quote.amountOut}<span class="text-ivory-muted/50 text-xs ml-1">{formatUsd(confQuote.quote.amountOutUsd)}</span></span></div>
              {#if confQuote.quote.minAmountOut}<div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Min. Received</span><span class="text-ivory-muted text-xs font-mono">{confQuote.quote.minAmountOut}</span></div>{/if}
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Est. Time</span><span class="text-ivory-muted text-xs">{formatTime(confQuote.quote.timeEstimate)}</span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Privacy</span><span class="text-gold text-xs flex items-center gap-1"><svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M9 12.75L11.25 15 15 9.75" /></svg>Confidential (TEE)</span></div>
              <div class="flex justify-between items-center"><span class="text-ivory-muted text-xs">Slippage</span><span class="text-ivory-muted text-xs">1% max</span></div>
              <div class="border-t border-border/30 pt-3 mt-3">
                <button onclick={executeConfidentialSwap} disabled={confExecuting} class="w-full py-2.5 bg-gold/20 border border-gold text-gold text-sm font-medium tracking-wider uppercase rounded-lg hover:bg-gold/30 transition-all duration-300 disabled:opacity-40 disabled:cursor-not-allowed">
                  {#if confExecuting}<span class="flex items-center justify-center gap-2"><span class="w-3.5 h-3.5 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>Executing confidentially...</span>{:else}Swap Confidentially{/if}
                </button>
                {#if confSuccess}<p class="text-positive text-xs mt-2">{confSuccess}</p>{/if}
              </div>
            </div>
          {/if}
        </section>

        <div class="grid grid-cols-3 gap-4 mt-6">
          <div class="bg-surface border border-border/30 rounded-lg p-4 text-center">
            <div class="text-gold text-lg mb-1">MEV</div>
            <p class="text-ivory-muted/50 text-[10px] leading-relaxed">Protected from value extraction</p>
          </div>
          <div class="bg-surface border border-border/30 rounded-lg p-4 text-center">
            <div class="text-gold text-lg mb-1">TEE</div>
            <p class="text-ivory-muted/50 text-[10px] leading-relaxed">Hardware-secured execution</p>
          </div>
          <div class="bg-surface border border-border/30 rounded-lg p-4 text-center">
            <div class="text-gold text-lg mb-1">25+</div>
            <p class="text-ivory-muted/50 text-[10px] leading-relaxed">Chains supported</p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Supported Chains (shared) -->
    <section class="mb-10 mt-10">
      <h2 class="text-ivory-muted text-xs tracking-widest uppercase mb-4">Supported Chains</h2>
      <div class="flex flex-wrap gap-3">
        {#each [
          { name: 'Ethereum', symbol: 'ETH', color: 'bg-[#627EEA]/10 border-[#627EEA]/20 text-[#627EEA]' },
          { name: 'NEAR', symbol: 'NEAR', color: 'bg-[#00EC97]/10 border-[#00EC97]/20 text-[#00EC97]' },
          { name: 'Solana', symbol: 'SOL', color: 'bg-[#9945FF]/10 border-[#9945FF]/20 text-[#9945FF]' },
          { name: 'Bitcoin', symbol: 'BTC', color: 'bg-[#F7931A]/10 border-[#F7931A]/20 text-[#F7931A]' },
          { name: 'USDC', symbol: 'USDC', color: 'bg-[#2775CA]/10 border-[#2775CA]/20 text-[#2775CA]' },
          { name: 'USDT', symbol: 'USDT', color: 'bg-[#26A17B]/10 border-[#26A17B]/20 text-[#26A17B]' },
        ] as chain}
          <div class="px-3 py-1.5 rounded-full border text-xs font-medium {chain.color}">{chain.symbol}</div>
        {/each}
        <div class="px-3 py-1.5 rounded-full border text-xs font-medium bg-ivory-muted/5 border-ivory-muted/10 text-ivory-muted/40">25+ chains via NEAR Intents</div>
      </div>
    </section>

    <!-- How It Works (expandable) -->
    <section class="mb-10">
      <button onclick={() => howItWorksOpen = !howItWorksOpen} class="flex items-center justify-between w-full text-left group">
        <h2 class="text-ivory-muted text-xs tracking-widest uppercase">How It Works</h2>
        <svg class="w-4 h-4 text-ivory-muted/40 transition-transform duration-300" class:rotate-180={howItWorksOpen} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path d="M19.5 8.25l-7.5 7.5-7.5-7.5" /></svg>
      </button>
      {#if howItWorksOpen}
        <div class="mt-6 space-y-8 animate-fade-in">
          <div>
            <h3 class="text-ivory text-sm font-medium mb-3">ZEC Shield — Balance Privacy</h3>
            <div class="bg-surface border border-border/50 rounded-xl p-6">
              <div class="flex items-center justify-center gap-2 text-xs font-mono flex-wrap">
                <span class="px-3 py-1.5 bg-surface-raised rounded-lg text-ivory">Your Crypto</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-[#00EC97]/10 border border-[#00EC97]/20 rounded-lg text-[#00EC97]">NEAR Intents</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-gold/10 border border-gold/20 rounded-lg text-gold">ZEC</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-positive/10 border border-positive/20 rounded-lg text-positive">Shielded Pool</span>
              </div>
              <p class="text-ivory-muted/40 text-xs text-center mt-3">zk-SNARKs hide your balance, sender, and receiver on-chain. Long-term cryptographic privacy.</p>
            </div>
          </div>
          <div>
            <h3 class="text-ivory text-sm font-medium mb-3">Confidential Intents — Execution Privacy</h3>
            <div class="bg-surface border border-border/50 rounded-xl p-6">
              <div class="flex items-center justify-center gap-2 text-xs font-mono flex-wrap">
                <span class="px-3 py-1.5 bg-surface-raised rounded-lg text-ivory">Your Intent</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-gold/10 border border-gold/20 rounded-lg text-gold">TEE Enclave</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-[#00EC97]/10 border border-[#00EC97]/20 rounded-lg text-[#00EC97]">Private Shard</span>
                <span class="text-ivory-muted/30">&#8594;</span>
                <span class="px-3 py-1.5 bg-positive/10 border border-positive/20 rounded-lg text-positive">Settlement</span>
              </div>
              <p class="text-ivory-muted/40 text-xs text-center mt-3">TEE hardware prevents validators from seeing your amounts or routes during execution. No MEV, no frontrunning.</p>
            </div>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="bg-surface border border-border/30 rounded-lg p-4">
              <h3 class="text-ivory text-sm font-medium mb-2">ZEC Shield</h3>
              <ul class="text-ivory-muted/50 text-xs leading-relaxed space-y-1">
                <li>Cryptographic balance privacy (zk-SNARKs)</li>
                <li>Holdings invisible to chain analysis</li>
                <li>Best for: long-term privacy, cold storage</li>
              </ul>
            </div>
            <div class="bg-surface border border-border/30 rounded-lg p-4">
              <h3 class="text-ivory text-sm font-medium mb-2">Confidential Intents</h3>
              <ul class="text-ivory-muted/50 text-xs leading-relaxed space-y-1">
                <li>Execution privacy (TEE hardware enclaves)</li>
                <li>Prevents MEV, frontrunning, sandwich attacks</li>
                <li>Best for: active trading, large swaps, DeFi</li>
              </ul>
            </div>
          </div>
          <p class="text-ivory-muted/30 text-xs leading-relaxed">Privacy applies to on-chain analysis. Nyx processes swaps locally on your machine — no data leaves your device. Compliance with local regulations remains your responsibility.</p>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .animate-fade-in {
    animation: fade-in 0.3s ease-out;
  }
</style>
