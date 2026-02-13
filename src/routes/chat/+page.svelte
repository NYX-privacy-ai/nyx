<script lang="ts">
  import { onMount } from 'svelte';
  import CapabilitySummary from '$lib/components/CapabilitySummary.svelte';

  // Types
  interface Message { role: string; content: string }
  interface SessionInfo {
    sessionKey: string;
    sessionId?: string;
    updatedAt?: number;
    totalTokens?: number;
    model?: string;
    title?: string;
    folder?: string;
  }
  interface ChatFolder { id: string; name: string; order: number }
  interface ChatFoldersData {
    folders: ChatFolder[];
    session_folders: Record<string, string>;
    session_titles: Record<string, string>;
  }

  // State
  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let chatContainer: HTMLElement;

  // Provider state
  let activeProvider = $state<'gateway' | 'ollama'>('gateway');
  let ollamaModel = $state('');
  let ollamaAvailable = $state(false);
  let agentName = $state('Nyx');

  // Session state
  let sessions: SessionInfo[] = $state([]);
  let activeSessionKey = $state('agent:default:main');
  let folders: ChatFolder[] = $state([]);
  let sidebarOpen = $state(true);
  let expandedFolders: Set<string> = $state(new Set(['_unfiled', '_all']));

  // Edit state
  let renamingSession = $state<string | null>(null);
  let renameValue = $state('');
  let creatingFolder = $state(false);
  let newFolderName = $state('');

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  // Helpers
  function timeAgo(ts?: number): string {
    if (!ts) return '';
    const diff = Date.now() - ts;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return new Date(ts).toLocaleDateString();
  }

  function toggleFolder(id: string) {
    const next = new Set(expandedFolders);
    if (next.has(id)) next.delete(id); else next.add(id);
    expandedFolders = next;
  }

  // Data loading
  async function loadSessions() {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      sessions = await invoke('list_chat_sessions');
    } catch { sessions = []; }
  }

  async function loadFolders() {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const data: ChatFoldersData = await invoke('get_chat_folders');
      folders = data.folders;
    } catch { folders = []; }
  }

  // Session actions
  async function createNewChat(folderId?: string) {
    if (!isTauri) return;
    const { invoke } = await import('@tauri-apps/api/core');
    const key: string = await invoke('create_chat_session', {
      title: 'New chat',
      folder: folderId ?? null
    });
    activeSessionKey = key;
    messages = [];
    await loadSessions();
  }

  async function switchSession(key: string) {
    activeSessionKey = key;
    messages = [];
  }

  async function startRename(key: string, currentTitle: string) {
    renamingSession = key;
    renameValue = currentTitle;
  }

  async function confirmRename() {
    if (!renamingSession || !isTauri) return;
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('rename_chat_session', { sessionKey: renamingSession, title: renameValue });
    renamingSession = null;
    await loadSessions();
  }

  async function moveToFolder(sessionKey: string, folderId: string | null) {
    if (!isTauri) return;
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('move_session_to_folder', { sessionKey, folderId });
    await loadSessions();
  }

  // Folder actions
  async function createFolder() {
    if (!newFolderName.trim() || !isTauri) return;
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('create_chat_folder', { name: newFolderName.trim() });
    newFolderName = '';
    creatingFolder = false;
    await loadFolders();
  }

  async function deleteFolder(folderId: string) {
    if (!isTauri) return;
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('delete_chat_folder', { folderId });
    await loadFolders();
    await loadSessions();
  }

  // Chat
  async function handleSubmit() {
    if (!input.trim() || loading) return;
    const userMessage = input.trim();
    messages.push({ role: 'user', content: userMessage });
    input = '';
    loading = true;

    const isFirstMessage = messages.filter(m => m.role === 'user').length === 1;

    try {
      if (!isTauri) {
        await new Promise(r => setTimeout(r, 800));
        messages.push({ role: 'assistant', content: 'Running in dev mode — Tauri backend not available.' });
      } else {
        const { invoke } = await import('@tauri-apps/api/core');
        let response: string;

        if (activeProvider === 'ollama' && ollamaAvailable && ollamaModel) {
          const history = messages.filter(m => m.role !== 'system').slice(0, -1)
            .map(m => ({ role: m.role, content: m.content }));
          response = await invoke('chat_ollama', { model: ollamaModel, message: userMessage, history });
        } else {
          response = await invoke('send_chat_message_to_session', {
            message: userMessage,
            sessionKey: activeSessionKey
          });
        }

        messages.push({ role: 'assistant', content: response });

        if (isFirstMessage && activeSessionKey !== 'agent:default:main') {
          const title = userMessage.length > 40 ? userMessage.slice(0, 40) + '...' : userMessage;
          await invoke('rename_chat_session', { sessionKey: activeSessionKey, title });
        }

        await loadSessions();
      }
    } catch (e: any) {
      messages.push({ role: 'assistant', content: `Error: ${e?.toString() || 'Unknown error'}` });
    } finally {
      loading = false;
    }

    setTimeout(() => chatContainer?.scrollTo({ top: chatContainer.scrollHeight, behavior: 'smooth' }), 50);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  // Derived: group sessions by folder
  let sessionsByFolder = $derived.by(() => {
    const map = new Map<string, SessionInfo[]>();
    map.set('_unfiled', []);
    for (const f of folders) map.set(f.id, []);
    for (const s of sessions) {
      const fid = s.folder || '_unfiled';
      if (!map.has(fid)) map.set(fid, []);
      map.get(fid)!.push(s);
    }
    return map;
  });

  // Init
  onMount(async () => {
    await loadFolders();
    await loadSessions();

    if (isTauri) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        try {
          agentName = await invoke('get_agent_name') as string;
        } catch { agentName = 'Nyx'; }
        const status: any = await invoke('check_ollama');
        ollamaAvailable = status.available;
        if (status.available) {
          const models: any[] = await invoke('list_ollama_models');
          if (models.length > 0) ollamaModel = models[0].name;
        }
      } catch { ollamaAvailable = false; }
    }
  });
</script>

<div class="flex h-full">
  <!-- Chat Sidebar -->
  {#if sidebarOpen}
    <div class="w-[260px] border-r border-border flex flex-col shrink-0 bg-black/50">
      <!-- Sidebar header -->
      <div class="px-3 py-3 border-b border-border flex items-center justify-between">
        <span class="text-ivory-muted text-xs uppercase tracking-wider">Conversations</span>
        <div class="flex items-center gap-1">
          <button
            onclick={() => createNewChat()}
            class="p-1.5 rounded-md hover:bg-surface-raised text-ivory-muted hover:text-ivory transition-colors"
            aria-label="New chat"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
          </button>
          <button
            onclick={() => sidebarOpen = false}
            class="p-1.5 rounded-md hover:bg-surface-raised text-ivory-muted hover:text-ivory transition-colors"
            aria-label="Close sidebar"
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M15.75 19.5L8.25 12l7.5-7.5" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Session list -->
      <div class="flex-1 overflow-y-auto py-2">
        <!-- Folders -->
        {#each folders as folder}
          {@const folderSessions = sessionsByFolder.get(folder.id) || []}
          {#if folderSessions.length > 0}
            <div class="mb-1">
              <button
                onclick={() => toggleFolder(folder.id)}
                class="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs text-ivory-muted hover:text-ivory transition-colors"
              >
                <svg class="w-3 h-3 transition-transform" class:rotate-90={expandedFolders.has(folder.id)} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path d="M8.25 4.5l7.5 7.5-7.5 7.5" />
                </svg>
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
                </svg>
                <span class="font-medium">{folder.name}</span>
                <span class="ml-auto text-ivory-muted/40">{folderSessions.length}</span>
              </button>
              {#if expandedFolders.has(folder.id)}
                {#each folderSessions as session}
                  <button
                    onclick={() => switchSession(session.sessionKey)}
                    class="w-full text-left px-3 py-2 pl-8 transition-colors duration-150 group"
                    class:bg-surface-raised={session.sessionKey === activeSessionKey}
                    class:hover:bg-surface={session.sessionKey !== activeSessionKey}
                  >
                    {#if renamingSession === session.sessionKey}
                      <input
                        type="text"
                        bind:value={renameValue}
                        onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') renamingSession = null; }}
                        onblur={() => confirmRename()}
                        class="w-full bg-transparent text-ivory text-sm border-b border-gold-dim outline-none selectable"
                      />
                    {:else}
                      <div class="text-ivory text-sm truncate">{session.title || 'Untitled'}</div>
                      <div class="text-ivory-muted/50 text-xs mt-0.5">{timeAgo(session.updatedAt)}</div>
                    {/if}
                  </button>
                {/each}
              {/if}
            </div>
          {/if}
        {/each}

        <!-- Unfiled sessions -->
        {#if (sessionsByFolder.get('_unfiled') || []).length > 0}
          {@const unfiledSessions = sessionsByFolder.get('_unfiled') || []}
          <div class="mb-1">
            <button
              onclick={() => toggleFolder('_unfiled')}
              class="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs text-ivory-muted hover:text-ivory transition-colors"
            >
              <svg class="w-3 h-3 transition-transform" class:rotate-90={expandedFolders.has('_unfiled')} fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path d="M8.25 4.5l7.5 7.5-7.5 7.5" />
              </svg>
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                <path d="M8.625 12a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H8.25m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H12m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0h-.375M21 12c0 4.556-4.03 8.25-9 8.25a9.764 9.764 0 01-2.555-.337A5.972 5.972 0 015.41 20.97a5.969 5.969 0 01-.474-.065 4.48 4.48 0 00.978-2.025c.09-.457-.133-.901-.467-1.226C3.93 16.178 3 14.189 3 12c0-4.556 4.03-8.25 9-8.25s9 3.694 9 8.25z" />
              </svg>
              <span class="font-medium">Chats</span>
              <span class="ml-auto text-ivory-muted/40">{unfiledSessions.length}</span>
            </button>
            {#if expandedFolders.has('_unfiled')}
              {#each unfiledSessions as session}
                <button
                  onclick={() => switchSession(session.sessionKey)}
                  class="w-full text-left px-3 py-2 pl-8 transition-colors duration-150 group"
                  class:bg-surface-raised={session.sessionKey === activeSessionKey}
                  class:hover:bg-surface={session.sessionKey !== activeSessionKey}
                >
                  {#if renamingSession === session.sessionKey}
                    <input
                      type="text"
                      bind:value={renameValue}
                      onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') renamingSession = null; }}
                      onblur={() => confirmRename()}
                      class="w-full bg-transparent text-ivory text-sm border-b border-gold-dim outline-none selectable"
                    />
                  {:else}
                    <div class="flex items-center justify-between">
                      <div class="text-ivory text-sm truncate flex-1">{session.title || 'Untitled'}</div>
                      <span
                        role="button"
                        tabindex="-1"
                        onclick={(e) => { e.stopPropagation(); startRename(session.sessionKey, session.title || ''); }}
                        onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); startRename(session.sessionKey, session.title || ''); } }}
                        class="opacity-0 group-hover:opacity-100 p-0.5 text-ivory-muted/40 hover:text-ivory transition-all cursor-pointer"
                        aria-label="Rename"
                      >
                        <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                          <path d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125" />
                        </svg>
                      </span>
                    </div>
                    <div class="text-ivory-muted/50 text-xs mt-0.5">{timeAgo(session.updatedAt)}</div>
                  {/if}
                </button>
              {/each}
            {/if}
          </div>
        {/if}

        <!-- Empty state -->
        {#if sessions.length === 0}
          <div class="px-4 py-8 text-center">
            <p class="text-ivory-muted/40 text-xs">No conversations yet</p>
          </div>
        {/if}
      </div>

      <!-- Sidebar footer -->
      <div class="px-3 py-2 border-t border-border">
        {#if creatingFolder}
          <div class="flex items-center gap-1">
            <input
              type="text"
              bind:value={newFolderName}
              onkeydown={(e) => { if (e.key === 'Enter') createFolder(); if (e.key === 'Escape') creatingFolder = false; }}
              placeholder="Folder name"
              class="flex-1 bg-transparent text-ivory text-xs px-2 py-1 border border-border rounded focus:border-gold-dim outline-none selectable"
            />
            <button onclick={() => createFolder()} class="text-xs text-gold-dim hover:text-ivory transition-colors px-1">Add</button>
          </div>
        {:else}
          <button
            onclick={() => creatingFolder = true}
            class="flex items-center gap-1.5 text-xs text-ivory-muted/50 hover:text-ivory-muted transition-colors"
          >
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M12 10.5v6m3-3H9m4.06-7.19l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
            </svg>
            New folder
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Main chat area -->
  <div class="flex-1 flex flex-col min-w-0">
    <!-- Messages -->
    <div class="flex-1 overflow-y-auto px-10 py-8" bind:this={chatContainer}>
      {#if !sidebarOpen}
        <button
          onclick={() => sidebarOpen = true}
          class="fixed left-[72px] top-4 z-10 p-1.5 rounded-md bg-surface border border-border hover:bg-surface-raised text-ivory-muted hover:text-ivory transition-colors"
          aria-label="Open sidebar"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path d="M8.25 4.5l7.5 7.5-7.5 7.5" />
          </svg>
        </button>
      {/if}

      <div class="max-w-2xl mx-auto space-y-6">
        {#if messages.length === 0}
          <div class="flex flex-col items-center justify-center h-full min-h-[400px] text-center">
            <div class="font-display text-3xl font-light tracking-wider text-ivory/20 mb-4">{agentName}</div>
            <p class="text-ivory-muted/50 text-sm mb-6">Ask anything — draft emails, schedule meetings, research travel, manage your portfolio, or just think out loud.</p>
            <CapabilitySummary condensed />
          </div>
        {:else}
          {#each messages as msg}
            <div class="flex" class:justify-end={msg.role === 'user'}>
              <div
                class="max-w-[80%] px-4 py-3 rounded-lg text-sm leading-relaxed selectable"
                class:bg-surface-raised={msg.role === 'user'}
                class:text-ivory={true}
                class:bg-surface={msg.role === 'assistant'}
                class:border-l-2={msg.role === 'assistant'}
                class:border-gold-dim={msg.role === 'assistant'}
              >
                {msg.content}
              </div>
            </div>
          {/each}
          {#if loading}
            <div class="flex">
              <div class="max-w-[80%] px-4 py-3 rounded-lg text-sm bg-surface border-l-2 border-gold-dim">
                <div class="flex items-center gap-1.5">
                  <div class="w-1.5 h-1.5 bg-ivory-muted/40 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                  <div class="w-1.5 h-1.5 bg-ivory-muted/40 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                  <div class="w-1.5 h-1.5 bg-ivory-muted/40 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                </div>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Input -->
    <div class="px-10 py-6 border-t border-border">
      <div class="max-w-2xl mx-auto">
        {#if ollamaAvailable && ollamaModel}
          <div class="flex items-center gap-2 mb-3">
            <button
              onclick={() => activeProvider = 'gateway'}
              class="text-xs px-2.5 py-1 rounded-md transition-colors duration-200 {activeProvider === 'gateway' ? 'bg-surface-raised text-ivory border border-border' : 'text-ivory-muted/50 hover:text-ivory-muted'}"
            >
              {agentName} Gateway
            </button>
            <button
              onclick={() => activeProvider = 'ollama'}
              class="text-xs px-2.5 py-1 rounded-md transition-colors duration-200 {activeProvider === 'ollama' ? 'bg-accent/10 text-accent border border-accent/30' : 'text-ivory-muted/50 hover:text-ivory-muted'}"
            >
              Local ({ollamaModel})
            </button>
          </div>
        {/if}

        <div class="relative">
          <input
            type="text"
            bind:value={input}
            onkeydown={handleKeydown}
            placeholder={activeProvider === 'ollama' ? `Ask ${ollamaModel}...` : `Ask ${agentName}...`}
            class="w-full bg-surface text-ivory text-sm px-4 py-3 rounded-lg border border-border focus:border-gold-dim focus:outline-none transition-colors duration-300 placeholder:text-ivory-muted/50 selectable"
            disabled={loading}
          />
          <button
            onclick={handleSubmit}
            class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-ivory-muted hover:text-gold transition-colors duration-200"
            class:opacity-0={!input.trim() || loading}
            class:opacity-100={input.trim() && !loading}
            aria-label="Send message"
            disabled={loading}
          >
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
