<script lang="ts">
  import { onMount } from 'svelte';

  interface KnowledgeEntry {
    id: number;
    title: string;
    content: string;
    tags: string[];
    category: string;
    source: string | null;
    source_ref: string | null;
    related_ids: number[];
    created_at: string;
    updated_at: string;
  }

  interface KnowledgeStats {
    total_entries: number;
    categories: { category: string; count: number }[];
    recent_count_7d: number;
    top_tags: { tag: string; count: number }[];
  }

  let entries: KnowledgeEntry[] = $state([]);
  let stats: KnowledgeStats | null = $state(null);
  let loading = $state(true);
  let error = $state('');
  let searchQuery = $state('');
  let categoryFilter = $state<string | null>(null);
  let selectedEntry = $state<KnowledgeEntry | null>(null);

  // Form state
  let showForm = $state(false);
  let editingId = $state<number | null>(null);
  let formTitle = $state('');
  let formContent = $state('');
  let formTags = $state('');
  let formCategory = $state('note');
  let formSaving = $state(false);

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  const categories = ['entity', 'concept', 'document', 'note', 'meeting', 'project'];

  const categoryColors: Record<string, string> = {
    entity: 'bg-blue-500/10 text-blue-400 border-blue-500/30',
    concept: 'bg-purple-500/10 text-purple-400 border-purple-500/30',
    document: 'bg-green-500/10 text-green-400 border-green-500/30',
    note: 'bg-gold/10 text-gold border-gold/30',
    meeting: 'bg-orange-500/10 text-orange-400 border-orange-500/30',
    project: 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30',
  };

  function formatDate(d: string): string {
    try {
      const date = new Date(d);
      return date.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
    } catch {
      return d;
    }
  }

  async function loadEntries() {
    if (!isTauri) return;
    loading = true;
    error = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [e, s] = await Promise.all([
        invoke<KnowledgeEntry[]>('list_knowledge', { category: categoryFilter, limit: 200 }),
        invoke<KnowledgeStats>('get_knowledge_stats'),
      ]);
      entries = e;
      stats = s;
    } catch (e: any) {
      error = e?.message || 'Failed to load';
    }
    loading = false;
  }

  async function doSearch() {
    if (!isTauri || !searchQuery.trim()) {
      await loadEntries();
      return;
    }
    loading = true;
    error = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      entries = await invoke<KnowledgeEntry[]>('search_knowledge', { query: searchQuery.trim() });
    } catch (e: any) {
      error = e?.message || 'Search failed';
    }
    loading = false;
  }

  async function deleteEntry(id: number) {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_knowledge_entry', { id });
      if (selectedEntry?.id === id) selectedEntry = null;
      await loadEntries();
    } catch (e: any) {
      error = e?.message || 'Failed to delete';
    }
  }

  function startCreate() {
    editingId = null;
    formTitle = '';
    formContent = '';
    formTags = '';
    formCategory = 'note';
    showForm = true;
    selectedEntry = null;
  }

  function startEdit(entry: KnowledgeEntry) {
    editingId = entry.id;
    formTitle = entry.title;
    formContent = entry.content;
    formTags = entry.tags.join(', ');
    formCategory = entry.category;
    showForm = true;
  }

  function cancelForm() {
    showForm = false;
    editingId = null;
  }

  async function saveForm() {
    if (!isTauri || !formTitle.trim()) return;
    formSaving = true;
    error = '';
    const tags = formTags.split(',').map(t => t.trim()).filter(Boolean);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (editingId) {
        const updated = await invoke<KnowledgeEntry>('update_knowledge_entry', {
          id: editingId,
          input: {
            title: formTitle.trim(),
            content: formContent.trim(),
            tags,
            category: formCategory,
          },
        });
        selectedEntry = updated;
      } else {
        const created = await invoke<KnowledgeEntry>('create_knowledge_entry', {
          input: {
            title: formTitle.trim(),
            content: formContent.trim(),
            tags,
            category: formCategory,
          },
        });
        selectedEntry = created;
      }
      showForm = false;
      editingId = null;
      await loadEntries();
    } catch (e: any) {
      error = e?.message || 'Failed to save';
    }
    formSaving = false;
  }

  function setCategory(c: string | null) {
    categoryFilter = c;
    searchQuery = '';
    loadEntries();
  }

  function handleSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter') doSearch();
  }

  onMount(() => {
    loadEntries();
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="px-8 py-6 border-b border-border shrink-0">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="font-display text-2xl text-ivory font-light tracking-wide">Wiki</h1>
        <p class="text-ivory-muted text-sm mt-1">Your personal knowledge base</p>
      </div>
      <button
        onclick={startCreate}
        class="px-4 py-2 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200"
      >
        + New Entry
      </button>
    </div>

    <!-- Search + filters -->
    <div class="flex items-center gap-4 mt-4">
      <div class="flex-1 relative">
        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-ivory-muted/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
        </svg>
        <input
          type="text"
          bind:value={searchQuery}
          onkeydown={handleSearchKey}
          placeholder="Search knowledge..."
          class="w-full pl-10 pr-4 py-2 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
        />
      </div>
      <div class="flex gap-2 text-xs">
        <button
          onclick={() => setCategory(null)}
          class="px-2.5 py-1 rounded-md border transition-all {categoryFilter === null ? 'bg-gold/10 border-gold/40 text-gold' : 'border-border text-ivory-muted hover:text-ivory'}"
        >
          All{stats ? ` (${stats.total_entries})` : ''}
        </button>
        {#each categories as cat}
          <button
            onclick={() => setCategory(cat)}
            class="px-2.5 py-1 rounded-md border transition-all {categoryFilter === cat ? 'bg-gold/10 border-gold/40 text-gold' : 'border-border text-ivory-muted hover:text-ivory'}"
          >
            {cat}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <!-- Error banner -->
  {#if error}
    <div class="mx-8 mt-4 px-4 py-2 bg-negative/10 border border-negative/30 rounded text-negative text-sm">
      {error}
      <button onclick={() => error = ''} class="ml-2 text-negative/60 hover:text-negative">&times;</button>
    </div>
  {/if}

  <!-- Content -->
  <div class="flex-1 overflow-hidden flex">
    {#if loading}
      <div class="flex-1 flex items-center justify-center">
        <div class="w-6 h-6 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></div>
      </div>
    {:else if showForm}
      <!-- Create/Edit Form -->
      <div class="flex-1 overflow-y-auto px-8 py-6">
        <div class="max-w-2xl mx-auto bg-surface border border-border rounded-xl p-6 space-y-5">
          <h2 class="font-display text-lg text-ivory font-light">
            {editingId ? 'Edit Entry' : 'New Entry'}
          </h2>

          <div>
            <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Title</label>
            <input
              type="text"
              bind:value={formTitle}
              placeholder="Entry title"
              class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
            />
          </div>

          <div>
            <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Content</label>
            <textarea
              bind:value={formContent}
              rows="10"
              placeholder="Write your knowledge entry..."
              class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors resize-none font-mono"
            ></textarea>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Category</label>
              <select
                bind:value={formCategory}
                class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm focus:outline-none focus:border-gold/50 transition-colors"
              >
                {#each categories as cat}
                  <option value={cat}>{cat}</option>
                {/each}
              </select>
            </div>
            <div>
              <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Tags</label>
              <input
                type="text"
                bind:value={formTags}
                placeholder="comma-separated tags"
                class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
              />
            </div>
          </div>

          <div class="flex items-center justify-end gap-3 pt-2">
            <button onclick={cancelForm} class="px-4 py-2 text-ivory-muted text-sm hover:text-ivory transition-colors">
              Cancel
            </button>
            <button
              onclick={saveForm}
              disabled={formSaving || !formTitle.trim()}
              class="px-6 py-2 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {#if formSaving}
                <span class="inline-block w-4 h-4 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>
              {:else}
                {editingId ? 'Save Changes' : 'Create Entry'}
              {/if}
            </button>
          </div>
        </div>
      </div>
    {:else}
      <!-- List + Detail split -->
      <div class="flex-1 flex overflow-hidden">
        <!-- Entry list -->
        <div class="w-[340px] border-r border-border overflow-y-auto px-4 py-4 space-y-2 shrink-0">
          {#if entries.length === 0}
            <div class="flex flex-col items-center justify-center py-16 text-center">
              <svg class="w-10 h-10 text-ivory-muted/30 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
                <path d="M12 6.042A8.967 8.967 0 006 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 016 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 016-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0018 18a8.967 8.967 0 00-6 2.292m0-14.25v14.25" />
              </svg>
              <p class="text-ivory-muted text-sm">No entries found</p>
            </div>
          {:else}
            {#each entries as entry (entry.id)}
              <button
                onclick={() => selectedEntry = entry}
                class="w-full text-left px-4 py-3 rounded-lg border transition-all duration-200 {selectedEntry?.id === entry.id ? 'bg-surface-raised border-gold/30' : 'bg-surface border-border hover:border-border-bright'}"
              >
                <div class="flex items-center gap-2 mb-1">
                  <span class="text-[10px] tracking-wider uppercase px-1.5 py-0.5 rounded border {categoryColors[entry.category] || 'bg-surface-raised text-ivory-muted border-border'}">{entry.category}</span>
                </div>
                <h3 class="text-ivory text-sm truncate">{entry.title}</h3>
                <p class="text-ivory-muted/50 text-xs mt-1 line-clamp-2">{entry.content.slice(0, 120)}</p>
                {#if entry.tags.length > 0}
                  <div class="flex gap-1 mt-2 flex-wrap">
                    {#each entry.tags.slice(0, 3) as tag}
                      <span class="text-ivory-muted/40 text-[10px]">#{tag}</span>
                    {/each}
                  </div>
                {/if}
              </button>
            {/each}
          {/if}
        </div>

        <!-- Detail panel -->
        <div class="flex-1 overflow-y-auto px-8 py-6">
          {#if selectedEntry}
            <div class="max-w-2xl">
              <div class="flex items-start justify-between mb-4">
                <div>
                  <div class="flex items-center gap-2 mb-2">
                    <span class="text-[10px] tracking-wider uppercase px-1.5 py-0.5 rounded border {categoryColors[selectedEntry.category] || 'bg-surface-raised text-ivory-muted border-border'}">{selectedEntry.category}</span>
                    <span class="text-ivory-muted/40 text-xs">{formatDate(selectedEntry.updated_at)}</span>
                  </div>
                  <h2 class="font-display text-xl text-ivory font-light">{selectedEntry.title}</h2>
                </div>
                <div class="flex gap-2">
                  <button
                    onclick={() => startEdit(selectedEntry!)}
                    class="px-3 py-1.5 text-ivory-muted text-xs border border-border rounded hover:border-gold/30 hover:text-gold transition-all"
                  >
                    Edit
                  </button>
                  <button
                    onclick={() => deleteEntry(selectedEntry!.id)}
                    class="px-3 py-1.5 text-ivory-muted text-xs border border-border rounded hover:border-negative/30 hover:text-negative transition-all"
                  >
                    Delete
                  </button>
                </div>
              </div>

              {#if selectedEntry.tags.length > 0}
                <div class="flex gap-2 mb-4 flex-wrap">
                  {#each selectedEntry.tags as tag}
                    <span class="text-ivory-muted/60 text-xs bg-surface-raised px-2 py-0.5 rounded">#{tag}</span>
                  {/each}
                </div>
              {/if}

              <div class="prose prose-invert prose-sm max-w-none">
                <pre class="whitespace-pre-wrap text-ivory/80 text-sm leading-relaxed font-sans">{selectedEntry.content}</pre>
              </div>

              {#if selectedEntry.source}
                <div class="mt-6 pt-4 border-t border-border">
                  <span class="text-ivory-muted/40 text-xs">Source: {selectedEntry.source}{selectedEntry.source_ref ? ` — ${selectedEntry.source_ref}` : ''}</span>
                </div>
              {/if}
            </div>
          {:else}
            <div class="flex flex-col items-center justify-center h-full text-center">
              <svg class="w-10 h-10 text-ivory-muted/20 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
                <path d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
              </svg>
              <p class="text-ivory-muted/40 text-sm">Select an entry to view</p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
