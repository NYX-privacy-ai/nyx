<script lang="ts">
  let {
    name,
    enabled,
    autonomy,
    onToggle,
    onAutonomyChange,
  }: {
    name: string;
    enabled: boolean;
    autonomy: string;
    onToggle: (enabled: boolean) => void;
    onAutonomyChange: (level: string) => void;
  } = $props();

  const channelColors: Record<string, string> = {
    Gmail: 'bg-red-500/20 text-red-400',
    WhatsApp: 'bg-green-500/20 text-green-400',
    Telegram: 'bg-blue-400/20 text-blue-300',
    Slack: 'bg-purple-500/20 text-purple-400',
  };

  const channelIcons: Record<string, string> = {
    Gmail: 'Gm',
    WhatsApp: 'Wa',
    Telegram: 'Tg',
    Slack: 'Sl',
  };

  const color = $derived(channelColors[name] || 'bg-ivory-muted/20 text-ivory-muted');
  const icon = $derived(channelIcons[name] || name.slice(0, 2));

  const autonomyLevels = [
    { value: 'draft_only', label: 'Draft Only', desc: 'Writes to drafts, never sends' },
    { value: 'send_with_confirm', label: 'Confirm', desc: 'Shows message, waits for approval' },
    { value: 'autonomous', label: 'Autonomous', desc: 'Sends directly, logs everything' },
  ];
</script>

<div class="rounded-lg border border-border bg-surface p-4 transition-colors duration-200">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <!-- Channel icon -->
      <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold {color}">
        {icon}
      </div>
      <span class="text-ivory text-sm">{name}</span>
    </div>

    <!-- Toggle -->
    <button
      onclick={() => onToggle(!enabled)}
      class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0 overflow-hidden"
      class:bg-accent={enabled}
      class:bg-border={!enabled}
      aria-label="Toggle {name}"
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-ivory transition-transform duration-200"
        style="transform: translateX({enabled ? '20px' : '0px'})"
      ></span>
    </button>
  </div>

  <!-- Autonomy selection (shown when enabled) -->
  {#if enabled}
    <div class="mt-4 ml-11 space-y-2">
      {#each autonomyLevels as level}
        <label class="flex items-start gap-2.5 cursor-pointer group">
          <input
            type="radio"
            name="autonomy-{name}"
            value={level.value}
            checked={autonomy === level.value}
            onchange={() => onAutonomyChange(level.value)}
            class="mt-0.5 accent-accent"
          />
          <div>
            <span class="text-ivory text-xs">{level.label}</span>
            <p class="text-ivory-muted text-[10px] leading-relaxed">{level.desc}</p>
            {#if level.value === 'autonomous'}
              <p class="text-negative text-[10px] mt-0.5">Messages sent without your approval. Use with caution.</p>
            {/if}
          </div>
        </label>
      {/each}
    </div>
  {/if}
</div>
