<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    icon,
    expanded = false,
    badge,
    children,
  }: {
    title: string;
    icon: string;
    expanded?: boolean;
    badge?: string;
    children: Snippet;
  } = $props();

  let isExpanded = $state(expanded);
</script>

<div class="border border-border rounded-lg overflow-hidden transition-colors duration-200">
  <!-- Header -->
  <button
    onclick={() => isExpanded = !isExpanded}
    class="w-full flex items-center justify-between px-5 py-4 hover:bg-surface-raised/50 transition-colors duration-200"
  >
    <div class="flex items-center gap-3">
      <svg class="w-4.5 h-4.5 text-ivory-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d={icon} />
      </svg>
      <span class="text-ivory text-sm font-medium tracking-wide">{title}</span>
      {#if badge}
        <span class="text-[10px] px-2 py-0.5 rounded-full bg-accent/10 text-accent">{badge}</span>
      {/if}
    </div>
    <svg
      class="w-4 h-4 text-ivory-muted transition-transform duration-200 {isExpanded ? 'rotate-180' : ''}"
      fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
    >
      <path d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
    </svg>
  </button>

  <!-- Content -->
  {#if isExpanded}
    <div class="px-5 pb-5 pt-1 border-t border-border/50">
      {@render children()}
    </div>
  {/if}
</div>
