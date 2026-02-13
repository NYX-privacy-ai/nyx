<script lang="ts">
  let {
    score,
    label,
    size = 'sm',
  }: {
    score: number;
    label: string;
    size?: 'sm' | 'md';
  } = $props();

  const radius = $derived(size === 'md' ? 54 : 36);
  const stroke = $derived(size === 'md' ? 7 : 5);
  const viewSize = $derived((radius + stroke) * 2);
  const circumference = $derived(2 * Math.PI * radius);
  const arcLength = $derived(circumference * 0.75); // 270-degree arc
  const dashOffset = $derived(arcLength - (arcLength * Math.min(Math.max(score, 0), 100)) / 100);

  const color = $derived(
    score >= 70
      ? 'text-positive'
      : score >= 40
        ? 'text-warning'
        : 'text-negative'
  );

  const trackColor = $derived(
    score >= 70
      ? 'text-positive/20'
      : score >= 40
        ? 'text-warning/20'
        : 'text-negative/20'
  );

  const fontSize = $derived(size === 'md' ? 'text-2xl' : 'text-sm');
  const labelSize = $derived(size === 'md' ? 'text-xs' : 'text-[10px]');
</script>

<div class="flex flex-col items-center gap-1">
  <div class="relative" style="width: {viewSize}px; height: {viewSize}px;">
    <svg
      width={viewSize}
      height={viewSize}
      viewBox="0 0 {viewSize} {viewSize}"
      class="transform -rotate-[135deg]"
    >
      <!-- Track -->
      <circle
        cx={viewSize / 2}
        cy={viewSize / 2}
        r={radius}
        fill="none"
        stroke="currentColor"
        stroke-width={stroke}
        stroke-dasharray="{arcLength} {circumference}"
        stroke-linecap="round"
        class={trackColor}
      />
      <!-- Fill -->
      <circle
        cx={viewSize / 2}
        cy={viewSize / 2}
        r={radius}
        fill="none"
        stroke="currentColor"
        stroke-width={stroke}
        stroke-dasharray="{arcLength} {circumference}"
        stroke-dashoffset={dashOffset}
        stroke-linecap="round"
        class="{color} transition-all duration-700 ease-out"
      />
    </svg>

    <!-- Score overlay -->
    <div class="absolute inset-0 flex items-center justify-center">
      <span class="{fontSize} font-bold {color}">{score}</span>
    </div>
  </div>

  <!-- Label -->
  <span class="{labelSize} text-ivory-muted text-center leading-tight mt-0.5 whitespace-pre-line">{label}</span>
</div>
