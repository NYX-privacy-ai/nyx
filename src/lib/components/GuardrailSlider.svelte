<script lang="ts">
  let {
    label,
    value,
    min,
    max,
    step,
    unit,
    onchange,
  }: {
    label: string;
    value: number;
    min: number;
    max: number;
    step: number;
    unit: string;
    onchange: (value: number) => void;
  } = $props();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    onchange(parseFloat(target.value));
  }

  const displayValue = $derived(
    unit === '$'
      ? `$${value.toLocaleString()}`
      : unit === '%'
        ? `${value}%`
        : `${value}`
  );

  const pct = $derived(((value - min) / (max - min)) * 100);
</script>

<div>
  <div class="flex justify-between items-center mb-1.5">
    <span class="text-ivory-muted text-xs">{label}</span>
    <span class="text-ivory text-xs font-mono">{displayValue}</span>
  </div>
  <input
    type="range"
    {min}
    {max}
    {step}
    {value}
    oninput={handleInput}
    class="slider w-full"
    style="--pct: {pct}%"
  />
</div>

<style>
  .slider {
    -webkit-appearance: none;
    appearance: none;
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      #6366F1 0%,
      #6366F1 var(--pct),
      #303036 var(--pct),
      #303036 100%
    );
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #6366F1;
    border: 2px solid #18181B;
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #6366F1;
    border: 2px solid #18181B;
    cursor: pointer;
  }
</style>
