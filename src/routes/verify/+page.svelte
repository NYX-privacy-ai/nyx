<script lang="ts">
  import VeritasReportCard from '$lib/components/VeritasReportCard.svelte';

  interface VeritasReport {
    url: string;
    title: string;
    author: string | null;
    domain: string;
    published_date: string | null;
    scores: {
      source_reputation: number;
      author_credibility: number;
      corroboration: number;
      evidence_quality: number;
      consistency: number;
      presentation: number;
    };
    overall_score: number;
    grade: string;
    claims: { claim: string; status: 'verified' | 'unverified' | 'disputed' | 'misleading' }[];
    summary: string;
    limitations: string;
  }

  let url = $state('');
  let loading = $state(false);
  let report = $state<VeritasReport | null>(null);
  let error = $state('');
  let history = $state<{ url: string; domain: string; score: number; grade: string; timestamp: number }[]>([]);

  function isValidUrl(str: string): boolean {
    try {
      const u = new URL(str);
      return u.protocol === 'http:' || u.protocol === 'https:';
    } catch {
      return false;
    }
  }

  const canVerify = $derived(isValidUrl(url) && !loading);

  async function handleVerify() {
    if (!canVerify) return;
    loading = true;
    error = '';
    report = null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const raw: string = await invoke('verify_source', { url });

      // Try to extract JSON from response (agent may wrap in markdown fences)
      let jsonStr = raw;
      const jsonMatch = raw.match(/```(?:json)?\s*([\s\S]*?)```/);
      if (jsonMatch) {
        jsonStr = jsonMatch[1].trim();
      }

      const parsed: VeritasReport = JSON.parse(jsonStr);
      report = parsed;

      // Add to history (most recent first, max 20)
      history = [
        {
          url: parsed.url || url,
          domain: parsed.domain || new URL(url).hostname,
          score: parsed.overall_score,
          grade: parsed.grade,
          timestamp: Date.now(),
        },
        ...history.filter((h) => h.url !== url),
      ].slice(0, 20);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && canVerify) {
      handleVerify();
    }
  }

  function loadFromHistory(historyUrl: string) {
    url = historyUrl;
    handleVerify();
  }

  function gradeColor(grade: string): string {
    if (grade === 'A') return 'text-positive';
    if (grade === 'B') return 'text-positive/80';
    if (grade === 'C') return 'text-warning';
    if (grade === 'D') return 'text-orange-400';
    return 'text-negative';
  }
</script>

<div class="h-full flex">
  <!-- Main content area -->
  <div class="flex-1 flex flex-col overflow-hidden">
    <!-- Header -->
    <header class="px-8 pt-8 pb-4 shrink-0">
      <h1 class="font-display text-gold text-2xl tracking-wide font-light">Source Intelligence</h1>
      <p class="text-ivory-muted text-sm mt-1">Verify the credibility of any online source</p>
    </header>

    <!-- URL Input -->
    <div class="px-8 pb-6 shrink-0">
      <div class="flex gap-3">
        <div class="flex-1 relative">
          <input
            type="url"
            bind:value={url}
            onkeydown={handleKeydown}
            placeholder="Paste a URL to verify..."
            class="w-full bg-surface border border-border rounded-lg px-4 py-3 text-ivory text-sm placeholder:text-ivory-muted/50 focus:outline-none focus:border-gold-dim transition-colors duration-200"
            disabled={loading}
          />
          {#if url && !isValidUrl(url)}
            <span class="absolute right-3 top-1/2 -translate-y-1/2 text-negative text-xs">Invalid URL</span>
          {/if}
        </div>
        <button
          onclick={handleVerify}
          disabled={!canVerify}
          class="px-6 py-3 rounded-lg text-sm font-medium transition-all duration-200 shrink-0
                 {canVerify
                   ? 'bg-gold text-black hover:bg-gold-dim cursor-pointer'
                   : 'bg-surface border border-border text-ivory-muted cursor-not-allowed'}"
        >
          {#if loading}
            <span class="flex items-center gap-2">
              <svg class="w-4 h-4 animate-spin" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" opacity="0.3" />
                <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              </svg>
              Analyzing...
            </span>
          {:else}
            Verify
          {/if}
        </button>
      </div>
    </div>

    <!-- Content area -->
    <div class="flex-1 overflow-y-auto px-8 pb-8">
      {#if loading}
        <!-- Loading state -->
        <div class="flex flex-col items-center justify-center py-20 gap-4">
          <div class="relative">
            <div class="w-16 h-16 rounded-full border-2 border-gold/20 flex items-center justify-center">
              <svg class="w-8 h-8 text-gold animate-pulse" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
              </svg>
            </div>
          </div>
          <p class="text-ivory-muted text-sm">Analyzing source credibility...</p>
          <p class="text-ivory-muted/50 text-xs">This may take up to a minute</p>
        </div>

      {:else if error}
        <!-- Error state -->
        <div class="border border-negative/30 rounded-lg bg-negative/5 p-6">
          <h3 class="text-negative text-sm font-medium mb-2">Analysis Failed</h3>
          <p class="text-ivory-muted text-xs leading-relaxed">{error}</p>
          <button
            onclick={handleVerify}
            class="mt-4 text-xs text-gold hover:text-gold-dim transition-colors underline underline-offset-2"
          >
            Try again
          </button>
        </div>

      {:else if report}
        <!-- Report -->
        <VeritasReportCard {report} />

      {:else}
        <!-- Empty state -->
        <div class="flex flex-col items-center justify-center py-20 gap-4">
          <div class="w-16 h-16 rounded-full border border-border flex items-center justify-center">
            <svg class="w-8 h-8 text-ivory-muted/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
            </svg>
          </div>
          <p class="text-ivory-muted text-sm">Paste a URL above to assess its credibility</p>
          <p class="text-ivory-muted/50 text-xs max-w-sm text-center">
            Veritas evaluates sources across 6 dimensions: reputation, author credibility,
            corroboration, evidence quality, consistency, and presentation.
          </p>
        </div>
      {/if}
    </div>
  </div>

  <!-- History sidebar -->
  {#if history.length > 0}
    <aside class="w-64 border-l border-border bg-surface shrink-0 flex flex-col overflow-hidden">
      <div class="px-4 py-4 border-b border-border">
        <h3 class="text-ivory-muted text-xs uppercase tracking-wider font-medium">Recent Checks</h3>
      </div>
      <div class="flex-1 overflow-y-auto">
        {#each history as item}
          <button
            onclick={() => loadFromHistory(item.url)}
            class="w-full text-left px-4 py-3 border-b border-border/50 hover:bg-surface-raised transition-colors duration-200"
          >
            <div class="flex items-center justify-between">
              <span class="text-ivory text-xs truncate max-w-[140px]">{item.domain}</span>
              <span class="text-sm font-bold {gradeColor(item.grade)}">{item.grade}</span>
            </div>
            <div class="flex items-center justify-between mt-1">
              <span class="text-ivory-muted text-[10px] truncate max-w-[140px]">{item.url}</span>
              <span class="text-ivory-muted text-[10px]">{item.score}/100</span>
            </div>
          </button>
        {/each}
      </div>
    </aside>
  {/if}
</div>
