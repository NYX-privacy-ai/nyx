<script lang="ts">
  import { onMount } from 'svelte';

  interface Task {
    id: number;
    title: string;
    description: string | null;
    status: string;
    priority: string;
    category: string | null;
    due_date: string | null;
    source: string | null;
    source_ref: string | null;
    created_at: string;
    updated_at: string;
    completed_at: string | null;
  }

  interface TaskStats {
    total: number;
    pending: number;
    in_progress: number;
    completed_today: number;
    overdue: number;
  }

  let tasks: Task[] = $state([]);
  let stats: TaskStats | null = $state(null);
  let loading = $state(true);
  let error = $state('');
  let statusFilter = $state<string | null>(null);

  // Form state
  let showForm = $state(false);
  let editingId = $state<number | null>(null);
  let formTitle = $state('');
  let formDescription = $state('');
  let formPriority = $state('normal');
  let formCategory = $state('');
  let formDueDate = $state('');
  let formSaving = $state(false);

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  const priorityColors: Record<string, string> = {
    urgent: 'text-negative',
    high: 'text-orange-400',
    normal: 'text-gold',
    low: 'text-ivory-muted',
  };

  const statusIcons: Record<string, string> = {
    pending: 'bg-ivory-muted/30',
    in_progress: 'bg-gold animate-pulse',
    completed: 'bg-positive',
    cancelled: 'bg-negative/40',
  };

  function formatDate(d: string | null): string {
    if (!d) return '';
    try {
      const date = new Date(d);
      return date.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
    } catch {
      return d;
    }
  }

  function isOverdue(task: Task): boolean {
    if (!task.due_date || task.status === 'completed' || task.status === 'cancelled') return false;
    return new Date(task.due_date) < new Date();
  }

  async function loadTasks() {
    if (!isTauri) return;
    loading = true;
    error = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [t, s] = await Promise.all([
        invoke<Task[]>('list_tasks', { status: statusFilter, category: null }),
        invoke<TaskStats>('get_task_stats'),
      ]);
      tasks = t;
      stats = s;
    } catch (e: any) {
      error = e?.message || 'Failed to load tasks';
    }
    loading = false;
  }

  async function toggleStatus(task: Task) {
    if (!isTauri) return;
    const next = task.status === 'completed' ? 'pending' : 'completed';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('update_task', { id: task.id, input: { status: next } });
      await loadTasks();
    } catch (e: any) {
      error = e?.message || 'Failed to update';
    }
  }

  async function deleteTask(id: number) {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_task', { id });
      await loadTasks();
    } catch (e: any) {
      error = e?.message || 'Failed to delete';
    }
  }

  function startCreate() {
    editingId = null;
    formTitle = '';
    formDescription = '';
    formPriority = 'normal';
    formCategory = '';
    formDueDate = '';
    showForm = true;
  }

  function startEdit(task: Task) {
    editingId = task.id;
    formTitle = task.title;
    formDescription = task.description || '';
    formPriority = task.priority;
    formCategory = task.category || '';
    formDueDate = task.due_date || '';
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
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (editingId) {
        await invoke('update_task', {
          id: editingId,
          input: {
            title: formTitle.trim(),
            description: formDescription.trim() || null,
            priority: formPriority,
            category: formCategory.trim() || null,
            due_date: formDueDate || null,
          },
        });
      } else {
        await invoke('create_task', {
          input: {
            title: formTitle.trim(),
            description: formDescription.trim() || null,
            priority: formPriority,
            category: formCategory.trim() || null,
            due_date: formDueDate || null,
          },
        });
      }
      showForm = false;
      editingId = null;
      await loadTasks();
    } catch (e: any) {
      error = e?.message || 'Failed to save';
    }
    formSaving = false;
  }

  function setFilter(s: string | null) {
    statusFilter = s;
    loadTasks();
  }

  onMount(() => {
    loadTasks();
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="px-8 py-6 border-b border-border shrink-0">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="font-display text-2xl text-ivory font-light tracking-wide">Tasks</h1>
        <p class="text-ivory-muted text-sm mt-1">Track and manage your to-dos</p>
      </div>
      <button
        onclick={startCreate}
        class="px-4 py-2 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200"
      >
        + New Task
      </button>
    </div>

    <!-- Stats bar -->
    {#if stats}
      <div class="flex gap-6 mt-4 text-xs">
        <button onclick={() => setFilter(null)} class="transition-colors {statusFilter === null ? 'text-gold' : 'text-ivory-muted hover:text-ivory'}">
          All <span class="font-mono">{stats.total}</span>
        </button>
        <button onclick={() => setFilter('pending')} class="transition-colors {statusFilter === 'pending' ? 'text-gold' : 'text-ivory-muted hover:text-ivory'}">
          Pending <span class="font-mono">{stats.pending}</span>
        </button>
        <button onclick={() => setFilter('in_progress')} class="transition-colors {statusFilter === 'in_progress' ? 'text-gold' : 'text-ivory-muted hover:text-ivory'}">
          In Progress <span class="font-mono">{stats.in_progress}</span>
        </button>
        <button onclick={() => setFilter('completed')} class="transition-colors {statusFilter === 'completed' ? 'text-gold' : 'text-ivory-muted hover:text-ivory'}">
          Done Today <span class="font-mono">{stats.completed_today}</span>
        </button>
        {#if stats.overdue > 0}
          <span class="text-negative">
            Overdue <span class="font-mono">{stats.overdue}</span>
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Error banner -->
  {#if error}
    <div class="mx-8 mt-4 px-4 py-2 bg-negative/10 border border-negative/30 rounded text-negative text-sm">
      {error}
      <button onclick={() => error = ''} class="ml-2 text-negative/60 hover:text-negative">&times;</button>
    </div>
  {/if}

  <!-- Content -->
  <div class="flex-1 overflow-y-auto px-8 py-6">
    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div class="w-6 h-6 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></div>
      </div>
    {:else if showForm}
      <!-- Create/Edit Form -->
      <div class="max-w-2xl mx-auto bg-surface border border-border rounded-xl p-6 space-y-5">
        <h2 class="font-display text-lg text-ivory font-light">
          {editingId ? 'Edit Task' : 'New Task'}
        </h2>

        <div>
          <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Title</label>
          <input
            type="text"
            bind:value={formTitle}
            placeholder="What needs to be done?"
            class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
          />
        </div>

        <div>
          <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Description</label>
          <textarea
            bind:value={formDescription}
            rows="3"
            placeholder="Optional details..."
            class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors resize-none"
          ></textarea>
        </div>

        <div class="grid grid-cols-3 gap-4">
          <div>
            <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Priority</label>
            <select
              bind:value={formPriority}
              class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm focus:outline-none focus:border-gold/50 transition-colors"
            >
              <option value="low">Low</option>
              <option value="normal">Normal</option>
              <option value="high">High</option>
              <option value="urgent">Urgent</option>
            </select>
          </div>
          <div>
            <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Category</label>
            <input
              type="text"
              bind:value={formCategory}
              placeholder="e.g. work, personal"
              class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
            />
          </div>
          <div>
            <label class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Due Date</label>
            <input
              type="date"
              bind:value={formDueDate}
              class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm focus:outline-none focus:border-gold/50 transition-colors"
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
              {editingId ? 'Save Changes' : 'Create Task'}
            {/if}
          </button>
        </div>
      </div>
    {:else if tasks.length === 0}
      <div class="flex flex-col items-center justify-center py-20 text-center">
        <svg class="w-12 h-12 text-ivory-muted/30 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
          <path d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <p class="text-ivory-muted text-sm">No tasks yet</p>
        <p class="text-ivory-muted/50 text-xs mt-1">Create one to start tracking your to-dos</p>
      </div>
    {:else}
      <div class="space-y-2 max-w-3xl">
        {#each tasks as task (task.id)}
          <div
            class="group bg-surface border border-border rounded-xl px-5 py-3.5 transition-colors hover:border-border-bright"
            class:opacity-50={task.status === 'completed' || task.status === 'cancelled'}
          >
            <div class="flex items-start gap-3">
              <!-- Checkbox -->
              <button
                onclick={() => toggleStatus(task)}
                class="mt-0.5 w-5 h-5 rounded border-2 flex items-center justify-center shrink-0 transition-all duration-200 {task.status === 'completed' ? 'bg-gold/20 border-gold' : 'border-border hover:border-gold/50'}"
              >
                {#if task.status === 'completed'}
                  <svg class="w-3 h-3 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                    <path d="M5 13l4 4L19 7" />
                  </svg>
                {/if}
              </button>

              <!-- Content -->
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <h3 class="text-ivory text-sm truncate {task.status === 'completed' ? 'line-through text-ivory-muted' : ''}">{task.title}</h3>
                  {#if task.priority !== 'normal'}
                    <span class="text-[10px] tracking-wider uppercase {priorityColors[task.priority] || 'text-ivory-muted'}">{task.priority}</span>
                  {/if}
                  {#if isOverdue(task)}
                    <span class="text-[10px] tracking-wider uppercase text-negative">Overdue</span>
                  {/if}
                </div>
                {#if task.description}
                  <p class="text-ivory-muted/60 text-xs mt-1 truncate">{task.description}</p>
                {/if}
                <div class="flex items-center gap-3 mt-1.5">
                  {#if task.category}
                    <span class="text-ivory-muted/40 text-[10px] tracking-wider uppercase">{task.category}</span>
                  {/if}
                  {#if task.due_date}
                    <span class="text-ivory-muted/40 text-[10px]">{formatDate(task.due_date)}</span>
                  {/if}
                  {#if task.status === 'in_progress'}
                    <span class="flex items-center gap-1 text-gold text-[10px] tracking-wider uppercase">
                      <span class="w-1.5 h-1.5 rounded-full bg-gold animate-pulse"></span>
                      In progress
                    </span>
                  {/if}
                </div>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onclick={() => startEdit(task)}
                  class="p-1.5 rounded hover:bg-surface-raised transition-colors"
                  title="Edit"
                >
                  <svg class="w-3.5 h-3.5 text-ivory-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125" />
                  </svg>
                </button>
                <button
                  onclick={() => deleteTask(task.id)}
                  class="p-1.5 rounded hover:bg-negative/10 transition-colors"
                  title="Delete"
                >
                  <svg class="w-3.5 h-3.5 text-ivory-muted hover:text-negative" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
