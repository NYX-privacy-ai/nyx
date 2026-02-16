<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();

  let setupComplete = $state<boolean | null>(null);

  // Update state
  let updateAvailable = $state(false);
  let updateVersion = $state('');
  let updateInstalling = $state(false);
  let updateProgress = $state('');
  let updateError = $state('');

  const nav = [
    { href: '/', icon: 'dashboard', label: 'Dashboard' },
    { href: '/chat', icon: 'chat', label: 'Chat' },
    { href: '/verify', icon: 'verify', label: 'Verify' },
    { href: '/privacy', icon: 'privacy', label: 'Privacy' }
  ];

  function isActive(href: string, pathname: string) {
    if (href === '/') return pathname === '/';
    return pathname.startsWith(href);
  }

  async function checkForUpdates() {
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (update) {
        updateAvailable = true;
        updateVersion = update.version;
        // Store the update object for later install
        (window as any).__nyx_update = update;
      }
    } catch (e) {
      console.log('Update check skipped:', e);
    }
  }

  async function installUpdate() {
    const update = (window as any).__nyx_update;
    if (!update) return;
    updateInstalling = true;
    updateError = '';
    updateProgress = 'Downloading...';
    try {
      let downloaded = 0;
      let contentLength = 0;
      await update.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            updateProgress = 'Downloading...';
            break;
          case 'Progress':
            downloaded += event.data.chunkLength || 0;
            if (contentLength > 0) {
              const pct = Math.round((downloaded / contentLength) * 100);
              updateProgress = `Downloading... ${pct}%`;
            }
            break;
          case 'Finished':
            updateProgress = 'Installing...';
            break;
        }
      });
      // Update installed — prompt restart
      updateProgress = 'Update installed — restart to apply.';
      updateInstalling = false;
    } catch (e: any) {
      updateError = e?.message || 'Update failed';
      updateInstalling = false;
      updateProgress = '';
    }
  }

  function dismissUpdate() {
    updateAvailable = false;
  }

  onMount(async () => {
    // Check if running inside Tauri (vs plain browser in dev)
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const complete = await invoke('check_setup_complete');
        setupComplete = complete as boolean;
        // Auto-redirect to setup if not complete and not already there
        if (!complete && !$page.url.pathname.startsWith('/setup')) {
          goto('/setup');
        }
      } catch {
        // If check fails, assume not set up
        setupComplete = false;
        if (!$page.url.pathname.startsWith('/setup')) {
          goto('/setup');
        }
      }

      // Check for updates in the background (3s after startup)
      setTimeout(() => checkForUpdates(), 3000);
    } else {
      // Running in plain browser (dev without Tauri) — show UI normally
      setupComplete = false;
    }
  });
</script>

<div class="flex h-screen bg-black overflow-hidden">
  <!-- Sidebar -->
  <nav class="w-[60px] flex flex-col items-center py-6 border-r border-border shrink-0">
    <!-- Monogram -->
    <div class="font-display text-gold text-2xl tracking-wider mb-10 font-light">N</div>

    <!-- Nav items -->
    <div class="flex flex-col gap-6 flex-1">
      {#each nav as item}
        <a
          href={item.href}
          class="group relative flex items-center justify-center w-10 h-10 rounded-lg transition-colors duration-300"
          class:bg-surface-raised={isActive(item.href, $page.url.pathname)}
          aria-label={item.label}
        >
          {#if item.icon === 'dashboard'}
            <svg class="w-5 h-5 transition-colors duration-300" class:text-gold={isActive(item.href, $page.url.pathname)} class:text-ivory-muted={!isActive(item.href, $page.url.pathname)} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25a2.25 2.25 0 01-2.25-2.25v-2.25z" />
            </svg>
          {:else if item.icon === 'chat'}
            <svg class="w-5 h-5 transition-colors duration-300" class:text-gold={isActive(item.href, $page.url.pathname)} class:text-ivory-muted={!isActive(item.href, $page.url.pathname)} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M8.625 12a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H8.25m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H12m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0h-.375M21 12c0 4.556-4.03 8.25-9 8.25a9.764 9.764 0 01-2.555-.337A5.972 5.972 0 015.41 20.97a5.969 5.969 0 01-.474-.065 4.48 4.48 0 00.978-2.025c.09-.457-.133-.901-.467-1.226C3.93 16.178 3 14.189 3 12c0-4.556 4.03-8.25 9-8.25s9 3.694 9 8.25z" />
            </svg>
          {:else if item.icon === 'verify'}
            <svg class="w-5 h-5 transition-colors duration-300" class:text-gold={isActive(item.href, $page.url.pathname)} class:text-ivory-muted={!isActive(item.href, $page.url.pathname)} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" />
            </svg>
          {:else if item.icon === 'privacy'}
            <svg class="w-5 h-5 transition-colors duration-300" class:text-gold={isActive(item.href, $page.url.pathname)} class:text-ivory-muted={!isActive(item.href, $page.url.pathname)} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
          {/if}

          <!-- Tooltip -->
          <span class="absolute left-14 px-2 py-1 bg-surface-raised text-ivory text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap">
            {item.label}
          </span>
        </a>
      {/each}
    </div>

    <!-- Settings at bottom -->
    <a
      href="/settings"
      class="group relative flex items-center justify-center w-10 h-10 rounded-lg transition-colors duration-300 hover:bg-surface-raised"
      class:bg-surface-raised={$page.url.pathname.startsWith('/settings')}
      aria-label="Settings"
    >
      <svg class="w-5 h-5 transition-colors duration-300" class:text-gold={$page.url.pathname.startsWith('/settings')} class:text-ivory-muted={!$page.url.pathname.startsWith('/settings')} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z" />
        <path d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
      <!-- Tooltip -->
      <span class="absolute left-14 px-2 py-1 bg-surface-raised text-ivory text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none whitespace-nowrap">
        Settings
      </span>
    </a>
  </nav>

  <!-- Main content -->
  <main class="flex-1 overflow-hidden relative">
    {@render children()}

    <!-- Update notification banner -->
    {#if updateAvailable}
      <div class="absolute bottom-0 left-0 right-0 bg-surface border-t border-gold/30 px-6 py-3 flex items-center justify-between z-50 animate-slide-up">
        {#if updateInstalling}
          <div class="flex items-center gap-3">
            <div class="w-4 h-4 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
            <span class="text-ivory text-sm">{updateProgress}</span>
          </div>
          {#if updateError}
            <span class="text-negative text-xs">{updateError}</span>
          {/if}
        {:else}
          <div class="flex items-center gap-3">
            <svg class="w-4 h-4 text-gold shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" />
            </svg>
            <span class="text-ivory text-sm">Nyx <span class="text-gold font-medium">v{updateVersion}</span> is available</span>
          </div>
          <div class="flex items-center gap-3">
            <button
              onclick={dismissUpdate}
              class="text-ivory-muted text-xs hover:text-ivory transition-colors"
            >
              Later
            </button>
            <button
              onclick={installUpdate}
              class="px-4 py-1.5 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200"
            >
              Update Now
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<style>
  @keyframes slide-up {
    from { transform: translateY(100%); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  .animate-slide-up {
    animation: slide-up 0.3s ease-out;
  }
</style>
