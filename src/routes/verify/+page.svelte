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
  let history = $state<{ url: string; domain: string; title: string; score: number; grade: string; timestamp: number }[]>([]);
  let showMethodology = $state(false);

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

      // The agent may include preamble text before the JSON — find the JSON object
      const jsonStart = jsonStr.indexOf('{');
      if (jsonStart > 0) {
        jsonStr = jsonStr.substring(jsonStart);
      }

      const parsed: VeritasReport = JSON.parse(jsonStr);
      report = parsed;

      // Add to history (most recent first, max 50)
      history = [
        {
          url: parsed.url || url,
          domain: parsed.domain || new URL(url).hostname,
          title: parsed.title || new URL(url).hostname,
          score: parsed.overall_score,
          grade: parsed.grade,
          timestamp: Date.now(),
        },
        ...history.filter((h) => h.url !== url),
      ].slice(0, 50);
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

  function removeFromHistory(index: number) {
    history = history.filter((_, i) => i !== index);
  }

  function clearHistory() {
    history = [];
  }

  function gradeColor(grade: string): string {
    if (grade === 'A') return 'text-positive';
    if (grade === 'B') return 'text-positive/80';
    if (grade === 'C') return 'text-warning';
    if (grade === 'D') return 'text-orange-400';
    return 'text-negative';
  }

  function gradeBg(grade: string): string {
    if (grade === 'A') return 'bg-positive/10 border-positive/20';
    if (grade === 'B') return 'bg-positive/5 border-positive/15';
    if (grade === 'C') return 'bg-warning/10 border-warning/20';
    if (grade === 'D') return 'bg-orange-500/10 border-orange-500/20';
    return 'bg-negative/10 border-negative/20';
  }

  function formatTimestamp(ts: number): string {
    const d = new Date(ts);
    const now = new Date();
    const diff = now.getTime() - ts;

    if (diff < 60_000) return 'just now';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    if (d.toDateString() === new Date(now.getTime() - 86_400_000).toDateString()) return 'yesterday';
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <header class="px-8 pt-8 pb-4 shrink-0">
    <h1 class="font-display text-gold text-2xl tracking-wide font-light">Source Intelligence</h1>
    <p class="text-ivory-muted text-sm mt-1">Verify the credibility of any online source</p>
  </header>

  <!-- URL Input -->
  <div class="px-8 pb-4 shrink-0">
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

  <!-- Methodology toggle + info -->
  <div class="px-8 pb-4 shrink-0">
    <button
      onclick={() => showMethodology = !showMethodology}
      class="text-ivory-muted/60 text-xs hover:text-ivory-muted transition-colors flex items-center gap-1.5"
    >
      <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" />
      </svg>
      {showMethodology ? 'Hide' : 'How'} scoring works
      <svg class="w-3 h-3 transition-transform duration-200 {showMethodology ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path d="M19 9l-7 7-7-7" />
      </svg>
    </button>

    {#if showMethodology}
      <div class="mt-3 border border-border rounded-lg bg-surface/50 p-4 space-y-3 animate-fadeIn">
        <p class="text-ivory-muted text-xs leading-relaxed">
          Sources are evaluated across <span class="text-ivory font-medium">6 evidence-based dimensions</span>, each scored 0–100. The overall score is a weighted aggregate:
        </p>
        <div class="grid grid-cols-2 gap-x-6 gap-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Corroboration</span>
            <span class="text-gold text-[10px] font-mono">25%</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Source Reputation</span>
            <span class="text-gold text-[10px] font-mono">20%</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Evidence Quality</span>
            <span class="text-gold text-[10px] font-mono">20%</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Author Credibility</span>
            <span class="text-gold text-[10px] font-mono">15%</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Consistency</span>
            <span class="text-gold text-[10px] font-mono">10%</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-ivory text-xs">Presentation</span>
            <span class="text-gold text-[10px] font-mono">10%</span>
          </div>
        </div>
        <div class="pt-2 border-t border-border/50">
          <p class="text-ivory-muted/70 text-[10px] leading-relaxed">
            <span class="font-medium text-ivory-muted">Grades:</span>
            <span class="text-positive">A</span> (90–100) ·
            <span class="text-positive/80">B</span> (75–89) ·
            <span class="text-warning">C</span> (60–74) ·
            <span class="text-orange-400">D</span> (40–59) ·
            <span class="text-negative">F</span> (0–39)
          </p>
          <p class="text-ivory-muted/50 text-[10px] mt-1">
            The AI agent fetches the URL content, cross-references claims against its knowledge base, and evaluates editorial rigour. Results are probabilistic assessments, not definitive judgments.
          </p>
        </div>
      </div>
    {/if}
  </div>

  <!-- Search History (inline, below search bar) -->
  {#if history.length > 0 && !report && !loading && !error}
    <div class="px-8 pb-4 shrink-0">
      <div class="border border-border rounded-lg bg-surface/50 overflow-hidden">
        <div class="flex items-center justify-between px-4 py-2.5 border-b border-border/50">
          <h3 class="text-ivory-muted text-xs uppercase tracking-wider font-medium">Recent Checks</h3>
          <button
            onclick={clearHistory}
            class="text-ivory-muted/40 text-[10px] hover:text-negative transition-colors uppercase tracking-wider"
          >
            Clear all
          </button>
        </div>
        <div class="max-h-48 overflow-y-auto">
          {#each history as item, i}
            <div class="flex items-center gap-3 px-4 py-2.5 border-b border-border/30 last:border-b-0 hover:bg-surface-raised/50 transition-colors group">
              <!-- Score badge -->
              <div class="shrink-0 w-9 h-9 rounded-lg border flex items-center justify-center {gradeBg(item.grade)}">
                <span class="text-sm font-bold {gradeColor(item.grade)}">{item.grade}</span>
              </div>

              <!-- Source info (clickable) -->
              <button
                onclick={() => loadFromHistory(item.url)}
                class="flex-1 min-w-0 text-left"
              >
                <div class="text-ivory text-xs font-medium truncate">{item.title}</div>
                <div class="flex items-center gap-2 mt-0.5">
                  <span class="text-ivory-muted/60 text-[10px] font-mono truncate">{item.domain}</span>
                  <span class="text-ivory-muted/30 text-[10px]">·</span>
                  <span class="text-ivory-muted/50 text-[10px]">{item.score}/100</span>
                  <span class="text-ivory-muted/30 text-[10px]">·</span>
                  <span class="text-ivory-muted/40 text-[10px]">{formatTimestamp(item.timestamp)}</span>
                </div>
              </button>

              <!-- Delete button -->
              <button
                onclick={() => removeFromHistory(i)}
                class="shrink-0 w-6 h-6 flex items-center justify-center rounded opacity-0 group-hover:opacity-100 hover:bg-negative/10 transition-all duration-200"
                aria-label="Remove from history"
              >
                <svg class="w-3.5 h-3.5 text-ivory-muted/40 hover:text-negative transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

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

    {:else if history.length === 0}
      <!-- Empty state (only shown when no history) -->
      <div class="flex flex-col items-center justify-center py-16 gap-4">
        <div class="w-16 h-16 rounded-full border border-border flex items-center justify-center">
          <svg class="w-8 h-8 text-ivory-muted/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
          </svg>
        </div>
        <p class="text-ivory-muted text-sm">Paste a URL above to assess its credibility</p>
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .animate-fadeIn {
    animation: fadeIn 0.2s ease-out;
  }
</style>
