<script lang="ts">
  import CredibilityGauge from './CredibilityGauge.svelte';

  interface VeritasClaim {
    claim: string;
    status: 'verified' | 'unverified' | 'disputed' | 'misleading';
  }

  interface VeritasScores {
    source_reputation: number;
    author_credibility: number;
    corroboration: number;
    evidence_quality: number;
    consistency: number;
    presentation: number;
  }

  interface VeritasReport {
    url: string;
    title: string;
    author: string | null;
    domain: string;
    published_date: string | null;
    scores: VeritasScores;
    overall_score: number;
    grade: string;
    claims: VeritasClaim[];
    summary: string;
    limitations: string;
  }

  let { report }: { report: VeritasReport } = $props();

  const gradeColor = $derived(
    report.grade === 'A'
      ? 'bg-positive/20 text-positive border-positive/30'
      : report.grade === 'B'
        ? 'bg-positive/10 text-positive/80 border-positive/20'
        : report.grade === 'C'
          ? 'bg-warning/20 text-warning border-warning/30'
          : report.grade === 'D'
            ? 'bg-orange-500/20 text-orange-400 border-orange-500/30'
            : 'bg-negative/20 text-negative border-negative/30'
  );

  const claimStatusIcon: Record<string, { icon: string; color: string }> = {
    verified: { icon: '\u2713', color: 'text-positive' },
    unverified: { icon: '?', color: 'text-ivory-muted' },
    disputed: { icon: '\u2717', color: 'text-warning' },
    misleading: { icon: '!', color: 'text-negative' },
  };

  const dimensions = $derived([
    { key: 'source_reputation', label: 'Source\nReputation', score: report.scores.source_reputation },
    { key: 'author_credibility', label: 'Author\nCredibility', score: report.scores.author_credibility },
    { key: 'corroboration', label: 'Corroboration', score: report.scores.corroboration },
    { key: 'evidence_quality', label: 'Evidence\nQuality', score: report.scores.evidence_quality },
    { key: 'consistency', label: 'Consistency', score: report.scores.consistency },
    { key: 'presentation', label: 'Presentation', score: report.scores.presentation },
  ]);
</script>

<div class="space-y-6">
  <!-- Header: Source metadata -->
  <div class="border border-border rounded-lg bg-surface p-5">
    <h3 class="text-ivory font-medium text-lg leading-tight">{report.title}</h3>
    <div class="flex items-center gap-3 mt-2">
      <span class="text-ivory-muted text-xs font-mono">{report.domain}</span>
      {#if report.author}
        <span class="text-ivory-muted text-xs">by {report.author}</span>
      {/if}
      {#if report.published_date}
        <span class="text-ivory-muted text-xs">{report.published_date}</span>
      {/if}
    </div>
  </div>

  <!-- Overall Score + Grade -->
  <div class="flex items-center gap-6 border border-border rounded-lg bg-surface p-5">
    <div class="relative">
      <CredibilityGauge score={report.overall_score} label="Overall" size="md" />
    </div>
    <div class="flex flex-col gap-2">
      <div class="flex items-center gap-3">
        <span class="text-3xl font-bold {gradeColor.includes('positive') ? 'text-positive' : gradeColor.includes('warning') ? 'text-warning' : gradeColor.includes('orange') ? 'text-orange-400' : 'text-negative'}">{report.grade}</span>
        <span class="px-2.5 py-1 rounded-md text-xs font-medium border {gradeColor}">
          {report.overall_score}/100
        </span>
      </div>
      <p class="text-ivory-muted text-sm leading-relaxed max-w-md">{report.summary}</p>
    </div>
  </div>

  <!-- Dimension Gauges (3x2 grid) -->
  <div class="border border-border rounded-lg bg-surface p-5">
    <h4 class="text-ivory text-sm font-medium mb-4">Dimension Scores</h4>
    <div class="grid grid-cols-3 gap-6">
      {#each dimensions as dim}
        <div class="flex justify-center relative">
          <CredibilityGauge score={dim.score} label={dim.label} size="sm" />
        </div>
      {/each}
    </div>
  </div>

  <!-- Key Claims -->
  {#if report.claims && report.claims.length > 0}
    <div class="border border-border rounded-lg bg-surface p-5">
      <h4 class="text-ivory text-sm font-medium mb-3">Key Claims</h4>
      <div class="space-y-2">
        {#each report.claims as claim}
          {@const statusInfo = claimStatusIcon[claim.status] || claimStatusIcon.unverified}
          <div class="flex items-start gap-3 py-2 px-3 rounded-md bg-black/30">
            <span class="w-5 h-5 flex items-center justify-center rounded-full text-xs font-bold shrink-0 mt-0.5 {statusInfo.color} bg-current/10">
              {statusInfo.icon}
            </span>
            <div class="flex-1 min-w-0">
              <p class="text-ivory text-sm leading-relaxed">{claim.claim}</p>
              <span class="text-[10px] uppercase tracking-wider {statusInfo.color}">{claim.status}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Limitations -->
  {#if report.limitations}
    <div class="border border-border rounded-lg bg-surface-raised p-4">
      <div class="flex items-start gap-2">
        <span class="text-ivory-muted text-sm shrink-0">&#9432;</span>
        <p class="text-ivory-muted text-xs leading-relaxed">{report.limitations}</p>
      </div>
    </div>
  {/if}
</div>
