<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let terminalContainer: HTMLDivElement;
  let terminal: any = null;
  let fitAddon: any = null;
  let sessionId = $state<string | null>(null);
  let status = $state<'connecting' | 'connected' | 'disconnected' | 'error'>('disconnected');
  let errorMessage = $state('');
  let unlisten: (() => void) | null = null;

  async function startTerminal() {
    status = 'connecting';
    errorMessage = '';

    try {
      // Dynamically import xterm.js
      const { Terminal } = await import('@xterm/xterm');
      const { FitAddon } = await import('@xterm/addon-fit');
      const { WebLinksAddon } = await import('@xterm/addon-web-links');

      // Create terminal with Nyx theme
      terminal = new Terminal({
        theme: {
          background: '#0A0A0A',
          foreground: '#F5F1EB',
          cursor: '#C9A84C',
          cursorAccent: '#0A0A0A',
          selectionBackground: 'rgba(201, 168, 76, 0.3)',
          selectionForeground: '#F5F1EB',
          black: '#0A0A0A',
          red: '#E54D42',
          green: '#5BB98B',
          yellow: '#C9A84C',
          blue: '#6B8AED',
          magenta: '#B07ACC',
          cyan: '#56C2D6',
          white: '#F5F1EB',
          brightBlack: '#3A3A3A',
          brightRed: '#EF6B63',
          brightGreen: '#7DD4A7',
          brightYellow: '#D4B966',
          brightBlue: '#8FA8F0',
          brightMagenta: '#C499DB',
          brightCyan: '#78D5E3',
          brightWhite: '#FFFFFF',
        },
        fontFamily: '"SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", Menlo, Monaco, "Courier New", monospace',
        fontSize: 13,
        lineHeight: 1.4,
        cursorBlink: true,
        cursorStyle: 'bar',
        allowTransparency: true,
        scrollback: 10000,
      });

      fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.loadAddon(new WebLinksAddon());

      // Attach to DOM
      terminal.open(terminalContainer);
      fitAddon.fit();

      const cols = terminal.cols;
      const rows = terminal.rows;

      // Spawn PTY via Tauri
      const { invoke } = await import('@tauri-apps/api/core');
      const { listen } = await import('@tauri-apps/api/event');

      const sid: string = await invoke('pty_spawn', { command: 'claude', cols, rows });
      sessionId = sid;

      // Listen for PTY output
      unlisten = await listen<[string, string]>('pty:output', (event) => {
        const [eventSid, data] = event.payload;
        if (eventSid === sessionId && terminal) {
          terminal.write(data);
        }
      });

      // Listen for PTY exit
      const unlistenExit = await listen<string>('pty:exit', (event) => {
        if (event.payload === sessionId) {
          status = 'disconnected';
          terminal?.write('\r\n\x1b[38;2;201;168;76m[Session ended]\x1b[0m\r\n');
        }
      });

      // Forward keystrokes to PTY
      terminal.onData((data: string) => {
        if (sessionId) {
          invoke('pty_write', { sessionId, data }).catch(() => {});
        }
      });

      // Handle terminal resize
      terminal.onResize(({ cols, rows }: { cols: number; rows: number }) => {
        if (sessionId) {
          invoke('pty_resize', { sessionId, cols, rows }).catch(() => {});
        }
      });

      // Observe container size changes for fit
      const resizeObserver = new ResizeObserver(() => {
        if (fitAddon) {
          try { fitAddon.fit(); } catch { /* ignore */ }
        }
      });
      resizeObserver.observe(terminalContainer);

      status = 'connected';
      terminal.focus();

      // Store cleanup for exit listener
      const origUnlisten = unlisten;
      unlisten = () => {
        origUnlisten?.();
        unlistenExit?.();
        resizeObserver.disconnect();
      };

    } catch (e: any) {
      status = 'error';
      errorMessage = e?.toString() || 'Failed to start terminal';
    }
  }

  async function killSession() {
    if (sessionId) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('pty_kill', { sessionId });
      } catch { /* ignore */ }
      sessionId = null;
      status = 'disconnected';
    }
  }

  async function restartSession() {
    await killSession();
    if (terminal) {
      terminal.clear();
    }
    await startTerminal();
  }

  onMount(() => {
    // xterm CSS is loaded dynamically to avoid SSR issues
    import('@xterm/xterm/css/xterm.css');
    startTerminal();
  });

  onDestroy(() => {
    unlisten?.();
    // Capture sessionId before async operations to avoid stale state
    const sid = sessionId;
    if (sid) {
      import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke('pty_kill', { sessionId: sid }).catch(() => {});
      });
    }
    terminal?.dispose();
  });
</script>

<div class="flex flex-col h-full bg-black">
  <!-- Header bar -->
  <div class="flex items-center justify-between px-5 py-2.5 border-b border-border shrink-0">
    <div class="flex items-center gap-3">
      <svg class="w-4 h-4 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M6.75 7.5l3 2.25-3 2.25m4.5 0h3m-9 8.25h12a2.25 2.25 0 002.25-2.25V5.25A2.25 2.25 0 0018 3H6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 006 21z" />
      </svg>
      <span class="font-display text-ivory text-sm tracking-wider">Claude Code</span>
      <!-- Status dot -->
      <div class="flex items-center gap-1.5">
        <div class="w-1.5 h-1.5 rounded-full {status === 'connected' ? 'bg-positive' : status === 'connecting' ? 'bg-gold animate-pulse' : 'bg-ivory-muted/30'}"></div>
        <span class="text-[10px] tracking-wider uppercase {status === 'connected' ? 'text-positive' : status === 'connecting' ? 'text-gold' : 'text-ivory-muted/40'}">
          {status === 'connected' ? 'Connected' : status === 'connecting' ? 'Connecting...' : status === 'error' ? 'Error' : 'Disconnected'}
        </span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      {#if status === 'connected'}
        <button
          onclick={restartSession}
          class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-border text-ivory-muted hover:text-ivory hover:border-ivory-muted/30 transition-colors"
        >
          Restart
        </button>
        <button
          onclick={killSession}
          class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-negative/30 text-negative/70 hover:text-negative hover:border-negative/50 transition-colors"
        >
          Kill
        </button>
      {:else if status === 'disconnected' || status === 'error'}
        <button
          onclick={startTerminal}
          class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-positive/30 text-positive/70 hover:text-positive hover:border-positive/50 transition-colors"
        >
          Start
        </button>
      {:else}
        <div class="w-4 h-4 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
      {/if}
    </div>
  </div>

  <!-- Error message -->
  {#if errorMessage}
    <div class="px-5 py-2 bg-negative/5 border-b border-negative/20">
      <p class="text-negative text-xs">{errorMessage}</p>
    </div>
  {/if}

  <!-- Terminal area -->
  <div
    class="flex-1 overflow-hidden"
    bind:this={terminalContainer}
  ></div>

  <!-- Status bar -->
  <div class="flex items-center justify-between px-5 py-1.5 border-t border-border/50 shrink-0">
    <span class="text-ivory-muted/30 text-[10px]">
      {sessionId ? `Session ${sessionId.slice(0, 8)}` : 'No active session'}
    </span>
    <span class="text-ivory-muted/20 text-[10px]">
      Nyx Terminal
    </span>
  </div>
</div>

<style>
  /* Ensure xterm fills the container */
  :global(.xterm) {
    height: 100%;
    padding: 8px 4px;
  }
  :global(.xterm-viewport) {
    scrollbar-width: thin;
    scrollbar-color: rgba(201, 168, 76, 0.2) transparent;
  }
  :global(.xterm-viewport::-webkit-scrollbar) {
    width: 6px;
  }
  :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: rgba(201, 168, 76, 0.2);
    border-radius: 3px;
  }
  :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
    background: rgba(201, 168, 76, 0.4);
  }
</style>
