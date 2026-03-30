<script lang="ts">
  let {
    name,
    description,
    enabled,
    onToggle,
    icon,
    prerequisites = [],
    locked = false,
    lockReason = '',
  }: {
    name: string;
    description: string;
    enabled: boolean;
    onToggle: (enabled: boolean) => void;
    icon: string;
    prerequisites?: string[];
    locked?: boolean;
    lockReason?: string;
  } = $props();

  const iconColors: Record<string, string> = {
    Fi: 'bg-positive/20 text-positive',
    Tr: 'bg-blue-400/20 text-blue-300',
    Gw: 'bg-red-500/20 text-red-400',
    Em: 'bg-amber-500/20 text-amber-300',
    Co: 'bg-purple-500/20 text-purple-400',
    Ve: 'bg-cyan-500/20 text-cyan-300',
  };

  const color = $derived(iconColors[icon] || 'bg-ivory-muted/20 text-ivory-muted');
</script>

<div
  class="rounded-lg border bg-surface p-4 transition-all duration-200"
  class:border-gold-dim={enabled && !locked}
  class:bg-surface-raised={enabled && !locked}
  class:border-border={!enabled || locked}
  class:opacity-50={locked}
>
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <!-- Icon -->
      <div class="w-9 h-9 rounded-lg flex items-center justify-center text-xs font-bold {color}">
        {icon}
      </div>
      <div>
        <span class="text-ivory text-sm">{name}</span>
        {#if prerequisites.length > 0}
          <div class="flex gap-1 mt-0.5">
            {#each prerequisites as prereq}
              <span class="text-ivory-muted/50 text-[10px] border border-border rounded px-1.5 py-0.5">{prereq}</span>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Toggle -->
    <button
      onclick={() => !locked && onToggle(!enabled)}
      class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
      class:bg-accent={enabled && !locked}
      class:bg-border={!enabled || locked}
      class:cursor-not-allowed={locked}
      aria-label="Toggle {name}"
      disabled={locked}
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-ivory transition-transform duration-200"
        style="transform: translateX({enabled ? '20px' : '0px'})"
      ></span>
    </button>
  </div>

  <p class="text-ivory-muted/60 text-xs mt-2 ml-12 leading-relaxed">{description}</p>
  {#if locked && lockReason}
    <p class="text-ivory-muted/40 text-[10px] mt-1 ml-12">{lockReason}</p>
  {/if}
</div>
