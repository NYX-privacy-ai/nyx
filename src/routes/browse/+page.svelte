<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // Types
  interface ActivityItem {
    id: number;
    action: string;
    detail: string;
    timestamp: number;
    status: 'running' | 'done' | 'error';
  }

  interface BrowserEvent {
    kind: string;
    url?: string;
    title?: string;
    message?: string;
  }

  interface BrowserAction {
    action: string;
    url?: string;
    selector?: string;
    text?: string;
  }

  // State
  let input = $state('');
  let urlInput = $state('');
  let loading = $state(false);
  let browserOpen = $state(false);
  let currentUrl = $state('');
  let currentTitle = $state('');
  let activities: ActivityItem[] = $state([]);
  let activityCounter = $state(0);
  let result = $state('');
  let webBrowsingEnabled = $state(true);
  let feedContainer = $state<HTMLElement | null>(null);
  let agentName = $state('Nyx');

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  // Event listener cleanup
  let unlistenEvent: (() => void) | null = null;
  let unlistenAction: (() => void) | null = null;

  function addActivity(action: string, detail: string, status: 'running' | 'done' | 'error' = 'running'): number {
    const id = ++activityCounter;
    activities = [...activities, { id, action, detail, timestamp: Date.now(), status }];
    // Auto-scroll feed
    setTimeout(() => {
      if (feedContainer) feedContainer.scrollTop = feedContainer.scrollHeight;
    }, 50);
    return id;
  }

  function updateActivity(id: number, status: 'done' | 'error') {
    activities = activities.map(a => a.id === id ? { ...a, status } : a);
  }

  function actionLabel(action: string): string {
    const labels: Record<string, string> = {
      navigate: 'Navigating',
      click: 'Clicking',
      type: 'Typing',
      scroll: 'Scrolling',
      read_page: 'Reading page',
      read_links: 'Reading links',
      read_forms: 'Reading forms',
      select: 'Selecting option',
      back: 'Going back',
      forward: 'Going forward',
      wait: 'Waiting',
      execute_js: 'Running script',
    };
    return labels[action] || action;
  }

  function actionDetail(data: BrowserAction): string {
    if (data.url) return data.url;
    if (data.selector && data.text) return `${data.selector}: "${data.text}"`;
    if (data.selector) return data.selector;
    if (data.text) return `"${data.text}"`;
    return '';
  }

  async function openBrowser() {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('browser_open');
      browserOpen = true;
    } catch (e: any) {
      console.error('Failed to open browser:', e);
    }
  }

  async function closeBrowser() {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('browser_close');
      browserOpen = false;
    } catch (e: any) {
      console.error('Failed to close browser:', e);
    }
  }

  async function navigateManual() {
    if (!isTauri || !urlInput.trim()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (!browserOpen) await invoke('browser_open');
      browserOpen = true;
      await invoke('browser_navigate', { url: urlInput.trim() });
      currentUrl = urlInput.trim();
      addActivity('navigate', urlInput.trim(), 'done');
    } catch (e: any) {
      addActivity('navigate', e?.message || 'Navigation failed', 'error');
    }
  }

  async function sendMessage() {
    if (!isTauri || !input.trim() || loading) return;

    const msg = input.trim();
    input = '';
    loading = true;
    result = '';

    // Open browser if not already open
    if (!browserOpen) {
      await openBrowser();
    }

    const thinkingId = addActivity('thinking', msg);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke('browser_send_message', {
        message: msg,
      }) as string;

      updateActivity(thinkingId, 'done');
      result = response;
      addActivity('complete', 'Task finished', 'done');
    } catch (e: any) {
      updateActivity(thinkingId, 'error');
      result = '';
      addActivity('error', e?.message || 'Request failed', 'error');
    } finally {
      loading = false;
    }
  }

  function clearActivities() {
    activities = [];
    result = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function handleUrlKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      navigateManual();
    }
  }

  onMount(async () => {
    if (!isTauri) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { listen } = await import('@tauri-apps/api/event');

      // Check current capability
      try {
        const config = await invoke('read_current_config') as any;
        webBrowsingEnabled = config?.capabilities?.web_browsing ?? true;
        agentName = config?.agent_name || 'Nyx';
      } catch { /* fallback to defaults */ }

      // Check existing browser state
      try {
        const state = await invoke('browser_state') as any;
        if (state) {
          browserOpen = true;
          currentUrl = state.current_url || '';
        }
      } catch { /* no browser open */ }

      // Listen for browser events
      unlistenEvent = await listen('browser:event', (event: any) => {
        const data = event.payload as BrowserEvent;
        if (data.kind === 'navigating') {
          currentUrl = data.url || '';
          addActivity('navigate', data.url || '');
        } else if (data.kind === 'loaded') {
          currentTitle = data.title || '';
          currentUrl = data.url || '';
        } else if (data.kind === 'thinking') {
          // Agent is processing — shown in activity feed
        } else if (data.kind === 'complete') {
          if (data.message) result = data.message;
        } else if (data.kind === 'closed') {
          browserOpen = false;
        }
      });

      // Listen for browser actions (from agent loop)
      unlistenAction = await listen('browser:action', (event: any) => {
        const data = event.payload as BrowserAction;
        addActivity(data.action, actionDetail(data));
      });
    } catch (e) {
      console.error('Browse page init error:', e);
    }
  });

  onDestroy(() => {
    if (unlistenEvent) unlistenEvent();
    if (unlistenAction) unlistenAction();
  });
</script>

<div class="h-full flex flex-col">
  <!-- Top bar with URL -->
  <div class="flex items-center gap-3 px-5 py-3 border-b border-border shrink-0">
    <!-- Browser toggle -->
    <button
      onclick={browserOpen ? closeBrowser : openBrowser}
      class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs transition-all duration-200 {browserOpen ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30' : 'bg-surface-raised text-ivory-muted border border-border hover:text-ivory'}"
      title={browserOpen ? 'Close browser' : 'Open browser'}
    >
      <!-- Globe icon -->
      <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
      </svg>
      {browserOpen ? 'Browser Open' : 'Open Browser'}
    </button>

    <!-- URL bar -->
    <div class="flex-1 flex items-center gap-2 bg-surface rounded-lg border border-border px-3 py-1.5">
      <!-- Back / Forward -->
      <button
        onclick={async () => {
          if (!isTauri) return;
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('browser_go_back');
        }}
        class="text-ivory-muted hover:text-ivory transition-colors"
        title="Back"
        disabled={!browserOpen}
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path d="M15.75 19.5L8.25 12l7.5-7.5" />
        </svg>
      </button>
      <button
        onclick={async () => {
          if (!isTauri) return;
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('browser_go_forward');
        }}
        class="text-ivory-muted hover:text-ivory transition-colors"
        title="Forward"
        disabled={!browserOpen}
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path d="M8.25 4.5l7.5 7.5-7.5 7.5" />
        </svg>
      </button>

      <!-- URL input -->
      <input
        type="text"
        bind:value={urlInput}
        onkeydown={handleUrlKeydown}
        placeholder={currentUrl || 'Enter URL or search...'}
        class="flex-1 bg-transparent text-ivory text-xs outline-none placeholder:text-ivory-muted/40"
        disabled={!browserOpen}
      />
      <button
        onclick={navigateManual}
        class="text-ivory-muted hover:text-gold transition-colors"
        title="Go"
        disabled={!browserOpen || !urlInput.trim()}
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
        </svg>
      </button>
    </div>
  </div>

  {#if !webBrowsingEnabled}
    <!-- Disabled state -->
    <div class="flex-1 flex items-center justify-center">
      <div class="text-center max-w-sm">
        <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-surface-raised flex items-center justify-center">
          <svg class="w-8 h-8 text-ivory-muted/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
          </svg>
        </div>
        <h2 class="font-display text-ivory text-lg font-light mb-2">Web Browsing Disabled</h2>
        <p class="text-ivory-muted/60 text-xs leading-relaxed mb-4">
          Enable Web Browsing in Settings to let {agentName} navigate websites on your behalf.
        </p>
        <a href="/settings" class="text-gold text-xs hover:text-gold/80 transition-colors">
          Go to Settings
        </a>
      </div>
    </div>
  {:else}
    <!-- Main content area -->
    <div class="flex-1 flex overflow-hidden">
      <!-- Activity feed (left panel) -->
      <div class="flex-1 flex flex-col min-w-0">
        <div class="flex items-center justify-between px-5 py-2 border-b border-border/50">
          <span class="text-ivory-muted/60 text-[10px] uppercase tracking-wider">Activity</span>
          <button
            onclick={clearActivities}
            class="text-ivory-muted/40 text-[10px] hover:text-ivory-muted transition-colors"
          >
            Clear
          </button>
        </div>

        <!-- Feed -->
        <div bind:this={feedContainer} class="flex-1 overflow-y-auto px-5 py-3 space-y-2">
          {#if activities.length === 0 && !result}
            <div class="flex flex-col items-center justify-center h-full text-center">
              <div class="w-12 h-12 mb-3 rounded-xl bg-indigo-500/10 flex items-center justify-center">
                <svg class="w-6 h-6 text-indigo-400/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
                </svg>
              </div>
              <p class="text-ivory-muted/40 text-xs mb-1">Tell {agentName} what to do</p>
              <p class="text-ivory-muted/25 text-[10px] max-w-[240px]">
                "Search for flights to London on Skyscanner"<br />
                "Order groceries from Ocado"<br />
                "Find the best-rated Italian restaurant nearby"
              </p>
            </div>
          {:else}
            {#each activities as item (item.id)}
              <div class="flex items-start gap-2 py-1">
                <!-- Status indicator -->
                {#if item.status === 'running'}
                  <div class="w-3 h-3 mt-0.5 shrink-0">
                    <div class="w-3 h-3 border border-indigo-400/50 border-t-indigo-400 rounded-full animate-spin"></div>
                  </div>
                {:else if item.status === 'done'}
                  <svg class="w-3 h-3 mt-0.5 text-emerald-400/80 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path d="M4.5 12.75l6 6 9-13.5" />
                  </svg>
                {:else}
                  <svg class="w-3 h-3 mt-0.5 text-red-400/80 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path d="M6 18L18 6M6 6l12 12" />
                  </svg>
                {/if}
                <div class="min-w-0">
                  <span class="text-ivory/80 text-[11px] font-medium">{actionLabel(item.action)}</span>
                  {#if item.detail}
                    <span class="text-ivory-muted/50 text-[10px] ml-1.5 truncate">{item.detail}</span>
                  {/if}
                </div>
              </div>
            {/each}

            <!-- Result -->
            {#if result}
              <div class="mt-4 p-3 rounded-lg bg-surface border border-border">
                <div class="text-ivory-muted/50 text-[10px] uppercase tracking-wider mb-1.5">Result</div>
                <div class="text-ivory/90 text-xs leading-relaxed whitespace-pre-wrap selectable">{result}</div>
              </div>
            {/if}
          {/if}
        </div>

        <!-- Input area -->
        <div class="px-5 py-3 border-t border-border shrink-0">
          <div class="flex items-end gap-2">
            <textarea
              bind:value={input}
              onkeydown={handleKeydown}
              placeholder={loading ? `${agentName} is browsing...` : `Tell ${agentName} what to browse...`}
              rows={1}
              disabled={loading}
              class="flex-1 bg-surface border border-border rounded-lg px-3 py-2 text-ivory text-xs outline-none resize-none placeholder:text-ivory-muted/30 focus:border-gold/40 transition-colors disabled:opacity-50"
            ></textarea>

            {#if loading}
              <!-- Stop button -->
              <button
                onclick={() => { loading = false; }}
                class="px-3 py-2 rounded-lg bg-red-500/15 border border-red-500/30 text-red-400 text-xs hover:bg-red-500/25 transition-all duration-200"
              >
                Stop
              </button>
            {:else}
              <!-- Send button -->
              <button
                onclick={sendMessage}
                disabled={!input.trim()}
                aria-label="Send message"
                class="px-3 py-2 rounded-lg bg-gold/10 border border-gold/30 text-gold text-xs hover:bg-gold/20 transition-all duration-200 disabled:opacity-30 disabled:pointer-events-none"
              >
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
                </svg>
              </button>
            {/if}
          </div>

          <!-- Safety notice -->
          <p class="text-ivory-muted/25 text-[9px] mt-2 leading-relaxed">
            {agentName} will never enter passwords, payment details, or sensitive information.
            A browser window opens separately so you can watch and intervene at any time.
          </p>
        </div>
      </div>
    </div>
  {/if}
</div>
