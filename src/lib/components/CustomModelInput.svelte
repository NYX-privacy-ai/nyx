<script lang="ts">
  let {
    onPull,
    pulling = false,
  }: {
    onPull: (tag: string) => void;
    pulling?: boolean;
  } = $props();

  let modelTag = $state('');
  let validationError = $state('');

  const tagPattern = /^[a-z0-9._-]+(?::[a-z0-9._-]+)?$/;

  function validate(tag: string): boolean {
    if (!tag.trim()) {
      validationError = '';
      return false;
    }
    if (!tagPattern.test(tag.trim())) {
      validationError = 'Format: model-name or model-name:tag (e.g. llama3.2:3b)';
      return false;
    }
    validationError = '';
    return true;
  }

  function submit() {
    const tag = modelTag.trim();
    if (validate(tag)) {
      onPull(tag);
      modelTag = '';
    }
  }
</script>

<div class="space-y-3">
  <div>
    <p class="text-ivory-muted text-xs mb-2">Pull any model from the Ollama library:</p>
    <div class="flex items-center gap-2">
      <input
        type="text"
        bind:value={modelTag}
        placeholder="e.g. phi3:mini, gemma2:2b, codestral:latest"
        disabled={pulling}
        class="flex-1 bg-surface text-ivory text-sm px-3 py-2 rounded border border-border focus:border-accent focus:outline-none transition-colors selectable disabled:opacity-50"
        oninput={() => validate(modelTag)}
        onkeydown={(e) => { if (e.key === 'Enter') submit(); }}
      />
      <button
        onclick={submit}
        disabled={pulling || !modelTag.trim() || !!validationError}
        class="px-4 py-2 text-xs tracking-wider uppercase rounded border transition-all duration-200 {pulling ? 'border-accent/30 text-accent/50 cursor-wait' : !modelTag.trim() || validationError ? 'border-border text-ivory-muted/40 cursor-not-allowed' : 'border-accent text-accent hover:bg-accent/10'}"
      >
        {#if pulling}
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 border-2 border-accent/40 border-t-accent rounded-full animate-spin"></div>
            Pulling...
          </div>
        {:else}
          Pull Model
        {/if}
      </button>
    </div>
    {#if validationError}
      <p class="text-negative text-[10px] mt-1">{validationError}</p>
    {/if}
  </div>

  <!-- Coming soon banner -->
  <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent/5 border border-accent/15">
    <svg class="w-4 h-4 text-accent flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
      <path d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z" />
    </svg>
    <div>
      <p class="text-ivory text-xs">More one-click local models coming soon</p>
      <p class="text-ivory-muted/50 text-[10px]">Browse available models at <button onclick={async () => { try { const { open } = await import('@tauri-apps/plugin-shell'); await open('https://ollama.com/library'); } catch { window.open('https://ollama.com/library', '_blank'); } }} class="text-accent hover:underline">ollama.com/library</button></p>
    </div>
  </div>
</div>
