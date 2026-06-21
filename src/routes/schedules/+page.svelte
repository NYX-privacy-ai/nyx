<script lang="ts">
  import { onMount } from 'svelte';

  // Types matching Rust CronJob struct
  interface CronSchedule {
    kind: 'every' | 'cron';
    everyMs?: number;
    expr?: string;
    tz?: string;
  }

  interface CronPayload {
    kind: 'agentTurn';
    message: string;
  }

  interface CronJob {
    id: string;
    agentId: string;
    name: string;
    schedule: CronSchedule;
    sessionTarget: string;
    payload: CronPayload;
    state: Record<string, unknown>;
    enabled: boolean;
    delivery: Record<string, unknown>;
  }

  // Built-in job IDs that can't be deleted
  const BUILTIN_IDS = new Set([
    'nyx-heartbeat',
    'daily-defi-report',
    'hourly-email-triage',
    'daily-email-digest',
  ]);

  // State
  let jobs: CronJob[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  // Form state
  let showForm = $state(false);
  let editingId = $state<string | null>(null);
  let formName = $state('');
  let formScheduleKind = $state<'every' | 'cron'>('cron');
  let formCronExpr = $state('0 9 * * *');
  let formIntervalHours = $state(4);
  let formTimezone = $state('Europe/London');
  let formMessage = $state('');
  let formSaving = $state(false);

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  // Helpers
  function formatSchedule(s: CronSchedule): string {
    if (s.kind === 'every' && s.everyMs) {
      const hours = s.everyMs / 3600000;
      if (hours >= 1) return `Every ${hours}h`;
      const mins = s.everyMs / 60000;
      return `Every ${mins}m`;
    }
    if (s.kind === 'cron' && s.expr) {
      return describeCron(s.expr) + (s.tz ? ` (${s.tz})` : '');
    }
    return 'Unknown';
  }

  function describeCron(expr: string): string {
    const parts = expr.trim().split(/\s+/);
    if (parts.length < 5) return expr;
    const [min, hour, dom, mon, dow] = parts;

    // Common patterns
    if (dom === '*' && mon === '*' && dow === '*') {
      if (hour.includes('-')) return `Hourly ${hour}`;
      if (hour !== '*') return `Daily at ${hour}:${min.padStart(2, '0')}`;
      return `Every hour at :${min.padStart(2, '0')}`;
    }
    return expr;
  }

  function getPayloadMessage(p: CronPayload): string {
    return p.message;
  }

  // Data loading
  async function loadJobs() {
    if (!isTauri) return;
    loading = true;
    error = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      jobs = await invoke('list_scheduled_tasks');
    } catch (e: any) {
      error = e?.message || 'Failed to load schedules';
      jobs = [];
    }
    loading = false;
  }

  // Actions
  async function toggleJob(id: string, enabled: boolean) {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('update_scheduled_task', { id, enabled });
      await loadJobs();
    } catch (e: any) {
      error = e?.message || 'Failed to update';
    }
  }

  async function deleteJob(id: string) {
    if (!isTauri || BUILTIN_IDS.has(id)) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_scheduled_task', { id });
      await loadJobs();
    } catch (e: any) {
      error = e?.message || 'Failed to delete';
    }
  }

  function startEdit(job: CronJob) {
    editingId = job.id;
    formName = job.name;
    if (job.schedule.kind === 'every') {
      formScheduleKind = 'every';
      formIntervalHours = (job.schedule.everyMs || 3600000) / 3600000;
    } else {
      formScheduleKind = 'cron';
      formCronExpr = job.schedule.expr || '0 9 * * *';
      formTimezone = job.schedule.tz || 'Europe/London';
    }
    formMessage = getPayloadMessage(job.payload);
    showForm = true;
  }

  function startCreate() {
    editingId = null;
    formName = '';
    formScheduleKind = 'cron';
    formCronExpr = '0 9 * * *';
    formIntervalHours = 4;
    formTimezone = 'Europe/London';
    formMessage = '';
    showForm = true;
  }

  function cancelForm() {
    showForm = false;
    editingId = null;
  }

  async function saveForm() {
    if (!isTauri || !formName.trim() || !formMessage.trim()) return;
    formSaving = true;
    error = '';

    const scheduleKind = formScheduleKind;
    const scheduleValue = scheduleKind === 'every'
      ? String(Math.round(formIntervalHours * 3600000))
      : formCronExpr;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (editingId) {
        await invoke('update_scheduled_task', {
          id: editingId,
          name: formName.trim(),
          scheduleKind,
          scheduleValue,
          timezone: scheduleKind === 'cron' ? formTimezone : null,
          message: formMessage.trim(),
        });
      } else {
        await invoke('create_scheduled_task', {
          name: formName.trim(),
          scheduleKind,
          scheduleValue,
          timezone: scheduleKind === 'cron' ? formTimezone : null,
          message: formMessage.trim(),
        });
      }
      showForm = false;
      editingId = null;
      await loadJobs();
    } catch (e: any) {
      error = e?.message || 'Failed to save';
    }
    formSaving = false;
  }

  onMount(() => {
    loadJobs();
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="px-8 py-6 border-b border-border shrink-0">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="font-display text-2xl text-ivory font-light tracking-wide">Scheduled Tasks</h1>
        <p class="text-ivory-muted text-sm mt-1">Recurring agent tasks and automations</p>
      </div>
      <button
        onclick={startCreate}
        class="px-4 py-2 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200"
      >
        + New Schedule
      </button>
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
  <div class="flex-1 overflow-y-auto px-8 py-6">
    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div class="w-6 h-6 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></div>
      </div>
    {:else if showForm}
      <!-- Create/Edit Form -->
      <div class="max-w-2xl mx-auto bg-surface border border-border rounded-xl p-6 space-y-5">
        <h2 class="font-display text-lg text-ivory font-light">
          {editingId ? 'Edit Schedule' : 'New Schedule'}
        </h2>

        <!-- Name -->
        <div>
          <label for="sched-name" class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Name</label>
          <input
            id="sched-name"
            type="text"
            bind:value={formName}
            placeholder="e.g. Morning Briefing"
            class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
          />
        </div>

        <!-- Schedule Type -->
        <div>
          <span class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Schedule Type</span>
          <div class="flex gap-3">
            <button
              onclick={() => formScheduleKind = 'cron'}
              class="px-4 py-2 text-sm rounded-lg border transition-all duration-200 {formScheduleKind === 'cron' ? 'bg-gold/10 border-gold/40 text-gold' : 'bg-surface-raised border-border text-ivory-muted'}"
            >
              Cron (specific times)
            </button>
            <button
              onclick={() => formScheduleKind = 'every'}
              class="px-4 py-2 text-sm rounded-lg border transition-all duration-200 {formScheduleKind === 'every' ? 'bg-gold/10 border-gold/40 text-gold' : 'bg-surface-raised border-border text-ivory-muted'}"
            >
              Interval (repeating)
            </button>
          </div>
        </div>

        <!-- Schedule Value -->
        {#if formScheduleKind === 'cron'}
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label for="sched-cron" class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Cron Expression</label>
              <input
                id="sched-cron"
                type="text"
                bind:value={formCronExpr}
                placeholder="0 9 * * *"
                class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm font-mono placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
              />
              <p class="text-ivory-muted/50 text-xs mt-1">min hour dom mon dow</p>
            </div>
            <div>
              <label for="sched-tz" class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">Timezone</label>
              <input
                id="sched-tz"
                type="text"
                bind:value={formTimezone}
                placeholder="Europe/London"
                class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors"
              />
            </div>
          </div>
        {:else}
          <div>
            <label for="sched-interval" class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">
              Interval (hours)
            </label>
            <input
              id="sched-interval"
              type="number"
              bind:value={formIntervalHours}
              min="0.5"
              step="0.5"
              class="w-32 px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm focus:outline-none focus:border-gold/50 transition-colors"
            />
            <p class="text-ivory-muted/50 text-xs mt-1">Minimum: 0.5 hours (30 minutes)</p>
          </div>
        {/if}

        <!-- Message -->
        <div>
          <label for="sched-message" class="block text-ivory-muted text-xs tracking-wider uppercase mb-2">
            Agent Instruction
          </label>
          <textarea
            id="sched-message"
            bind:value={formMessage}
            rows="4"
            placeholder="What should the agent do when this fires?"
            class="w-full px-4 py-2.5 bg-surface-raised border border-border rounded-lg text-ivory text-sm placeholder:text-ivory-muted/40 focus:outline-none focus:border-gold/50 transition-colors resize-none"
          ></textarea>
        </div>

        <!-- Actions -->
        <div class="flex items-center justify-end gap-3 pt-2">
          <button
            onclick={cancelForm}
            class="px-4 py-2 text-ivory-muted text-sm hover:text-ivory transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={saveForm}
            disabled={formSaving || !formName.trim() || !formMessage.trim()}
            class="px-6 py-2 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {#if formSaving}
              <span class="inline-block w-4 h-4 border-2 border-gold/30 border-t-gold rounded-full animate-spin"></span>
            {:else}
              {editingId ? 'Save Changes' : 'Create Schedule'}
            {/if}
          </button>
        </div>
      </div>
    {:else if jobs.length === 0}
      <div class="flex flex-col items-center justify-center py-20 text-center">
        <svg class="w-12 h-12 text-ivory-muted/30 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
          <path d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <p class="text-ivory-muted text-sm">No scheduled tasks yet</p>
        <p class="text-ivory-muted/50 text-xs mt-1">Create one to automate recurring agent tasks</p>
      </div>
    {:else}
      <div class="space-y-3 max-w-3xl">
        {#each jobs as job (job.id)}
          <div
            class="group bg-surface border border-border rounded-xl px-5 py-4 transition-colors hover:border-border-bright"
            class:opacity-50={!job.enabled}
          >
            <div class="flex items-start justify-between">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2.5">
                  <!-- Status indicator -->
                  <div
                    class="w-2 h-2 rounded-full shrink-0 {job.enabled ? 'bg-gold' : 'bg-ivory-muted/30'}"
                  ></div>

                  <h3 class="text-ivory text-sm font-medium truncate">{job.name}</h3>

                  {#if BUILTIN_IDS.has(job.id)}
                    <span class="text-ivory-muted/40 text-[10px] tracking-wider uppercase border border-border rounded px-1.5 py-0.5 shrink-0">
                      Built-in
                    </span>
                  {/if}
                </div>

                <p class="text-ivory-muted text-xs mt-1.5 ml-[18px]">
                  {formatSchedule(job.schedule)}
                </p>

                <p class="text-ivory-muted/50 text-xs mt-1 ml-[18px] truncate max-w-lg">
                  {getPayloadMessage(job.payload)}
                </p>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-2 ml-4 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                <!-- Toggle -->
                <button
                  onclick={() => toggleJob(job.id, !job.enabled)}
                  class="p-1.5 rounded hover:bg-surface-raised transition-colors"
                  title={job.enabled ? 'Disable' : 'Enable'}
                >
                  {#if job.enabled}
                    <svg class="w-4 h-4 text-gold" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d="M14.25 9v6m-4.5 0V9M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  {:else}
                    <svg class="w-4 h-4 text-ivory-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      <path d="M15.91 11.672a.375.375 0 010 .656l-5.603 3.113a.375.375 0 01-.557-.328V8.887c0-.286.307-.466.557-.327l5.603 3.112z" />
                    </svg>
                  {/if}
                </button>

                <!-- Edit -->
                <button
                  onclick={() => startEdit(job)}
                  class="p-1.5 rounded hover:bg-surface-raised transition-colors"
                  title="Edit"
                >
                  <svg class="w-4 h-4 text-ivory-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125" />
                  </svg>
                </button>

                <!-- Delete (non-builtin only) -->
                {#if !BUILTIN_IDS.has(job.id)}
                  <button
                    onclick={() => deleteJob(job.id)}
                    class="p-1.5 rounded hover:bg-negative/10 transition-colors"
                    title="Delete"
                  >
                    <svg class="w-4 h-4 text-ivory-muted hover:text-negative" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                    </svg>
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
