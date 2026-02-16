<script lang="ts">
  import { onMount } from 'svelte';
  import SettingsSection from '$lib/components/SettingsSection.svelte';
  import ApiKeyStatus from '$lib/components/ApiKeyStatus.svelte';
  import CustomModelInput from '$lib/components/CustomModelInput.svelte';
  import SaveBar from '$lib/components/SaveBar.svelte';
  import SecurityPresetCard from '$lib/components/SecurityPresetCard.svelte';
  import ChannelCard from '$lib/components/ChannelCard.svelte';
  import LocalModelCard from '$lib/components/LocalModelCard.svelte';

  // ── Loading state ──
  let loading = $state(true);
  let loadError = $state('');

  // ── Settings state (populated from backend) ──
  let agentName = $state('Nyx');
  let hasAnthropicKey = $state(false);
  let hasOpenaiKey = $state(false);
  let hasVeniceKey = $state(false);
  let hasNearaiKey = $state(false);
  let hasTelegramToken = $state(false);
  let hasSlackToken = $state(false);
  let whatsappPhone = $state('');
  let defaultLlmProvider = $state('anthropic');

  // Guardrails
  let securityPreset = $state('custom');
  let guardrails = $state({
    maxTransactionUsd: 500,
    dailyLossPercent: 5,
    weeklyLossPercent: 15,
    dailyTxLimit: 20,
    requireConfirmation: false,
    maxSlippagePercent: 2,
    maxConcentrationPercent: 40,
    minHealthFactor: 1.5,
  });

  // Messaging
  let messaging = $state({
    gmail: { enabled: false, autonomy: 'draft_only' },
    whatsapp: { enabled: false, autonomy: 'draft_only' },
    telegram: { enabled: false, autonomy: 'draft_only' },
    slack: { enabled: false, autonomy: 'draft_only' },
  });

  // Email notifications
  let emailEnabled = $state(true);
  let emailTimezone = $state('Europe/London');
  let emailDigestHour = $state(8);
  let emailDigestMinute = $state(30);
  let emailTriageStartHour = $state(8);
  let emailTriageEndHour = $state(22);

  // Capabilities
  let capabilities = $state({
    defi_crypto: true,
    travel: true,
    google_workspace: true,
    email_intelligence: true,
    communications: true,
    source_intelligence: true,
    activity_intelligence: false,
  });
  let googleAuthenticated = $state(false);
  let ollamaModel = $state('');

  // Ollama state
  let ollamaStatus = $state<'checking' | 'installed' | 'not_installed' | 'running' | 'installing'>('checking');
  let ollamaModels = $state<{name: string, size: number}[]>([]);
  let downloadingModel = $state<string | null>(null);
  let systemRam = $state(0);
  let customModelPulling = $state(false);

  // Docker state
  let dockerStatus = $state('checking');

  // ClawdTalk (voice calling) state
  let clawdtalkConfigured = $state(false);
  let clawdtalkConnected = $state(false);
  let clawdtalkHasKey = $state(false);
  let clawdtalkServer = $state('https://clawdtalk.com');
  let clawdtalkPid = $state<number | null>(null);
  let clawdtalkLogs = $state<string[]>([]);
  let clawdtalkApiKeyEditing = $state(false);
  let clawdtalkApiKeyValue = $state('');
  let clawdtalkSaving = $state(false);
  let clawdtalkStarting = $state(false);
  let clawdtalkError = $state('');

  // Claude Code state
  let claudeCodeInstalled = $state(false);
  let claudeCodeVersion = $state('');
  let claudeCodeMcpRegistered = $state(false);
  let claudeCodeBinaryPath = $state('');
  let claudeCodeLoading = $state(false);
  let claudeCodeError = $state('');
  let claudeCodeRegistering = $state(false);

  // Activity Intelligence autonomy
  let autonomySettings = $state<{activity_type: string, level: string, total_accepted: number, total_dismissed: number}[]>([]);
  let autonomyLoading = $state(false);

  // Update state
  let updateAvailable = $state(false);
  let updateVersion = $state('');
  let updateInstalling = $state(false);
  let updateProgress = $state('');
  let updateError = $state('');
  let appVersion = $state('');
  let checkingUpdates = $state(false);

  // Save state
  let saving = $state(false);
  let saveError = $state('');

  // Snapshot for change detection
  let snapshot = $state('');

  const recommendedModels = [
    { name: 'Qwen3 4B', tag: 'qwen3:4b', size: '2.7 GB', description: 'Fast reasoning, great for chat', minRam: 8 },
    { name: 'Llama 3.2 3B', tag: 'llama3.2:3b', size: '2.0 GB', description: 'Compact, good all-rounder', minRam: 8 },
    { name: 'Llama 3.1 8B', tag: 'llama3.1:8b', size: '4.7 GB', description: 'Strong general-purpose model', minRam: 16 },
    { name: 'Qwen 2.5 Coder 7B', tag: 'qwen2.5-coder:7b', size: '4.7 GB', description: 'Coding specialist, 92 languages', minRam: 16 },
    { name: 'Mistral 7B', tag: 'mistral:7b', size: '4.1 GB', description: 'Reliable workhorse', minRam: 16 },
    { name: 'DeepSeek Coder 6.7B', tag: 'deepseek-coder:6.7b', size: '3.8 GB', description: 'Coding-focused, compact', minRam: 16 },
  ];

  // ── Derived state ──
  function currentState(): string {
    return JSON.stringify({
      agentName, defaultLlmProvider, whatsappPhone, ollamaModel,
      guardrails, messaging, capabilities,
      emailEnabled, emailTimezone, emailDigestHour, emailDigestMinute,
      emailTriageStartHour, emailTriageEndHour,
    });
  }

  let hasChanges = $derived(snapshot !== '' && currentState() !== snapshot);

  let restartRequired = $derived(() => {
    if (!snapshot) return false;
    try {
      const snap = JSON.parse(snapshot);
      if (agentName !== snap.agentName) return true;
      if (defaultLlmProvider !== snap.defaultLlmProvider) return true;
      if (JSON.stringify(guardrails) !== JSON.stringify(snap.guardrails)) return true;
      if (JSON.stringify(messaging) !== JSON.stringify(snap.messaging)) return true;
      if (JSON.stringify(capabilities) !== JSON.stringify(snap.capabilities)) return true;
      return false;
    } catch { return false; }
  });

  // ── Detect preset from guardrails values ──
  function detectPreset(g: typeof guardrails): string {
    if (g.maxTransactionUsd === 100 && g.dailyLossPercent === 2 && g.weeklyLossPercent === 5 && g.requireConfirmation) return 'conservative';
    if (g.maxTransactionUsd === 500 && g.dailyLossPercent === 5 && g.weeklyLossPercent === 15 && !g.requireConfirmation) return 'balanced';
    if (g.maxTransactionUsd >= 1000000 && g.dailyLossPercent >= 100 && !g.requireConfirmation) return 'autonomous';
    return 'custom';
  }

  // ── Load config from backend ──
  async function loadConfig() {
    loading = true;
    loadError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      const config: any = await invoke('read_current_config');
      agentName = config.agent_name;
      hasAnthropicKey = config.has_anthropic_key;
      hasOpenaiKey = config.has_openai_key;
      hasVeniceKey = config.has_venice_key;
      hasNearaiKey = config.has_nearai_key;
      hasTelegramToken = config.has_telegram_token;
      hasSlackToken = config.has_slack_token;
      whatsappPhone = config.whatsapp_phone || '';
      defaultLlmProvider = config.default_llm_provider || 'anthropic';

      // Guardrails
      const g = config.guardrails;
      guardrails = {
        maxTransactionUsd: g.max_transaction_usd,
        dailyLossPercent: g.daily_loss_percent,
        weeklyLossPercent: g.weekly_loss_percent,
        dailyTxLimit: g.daily_tx_limit,
        requireConfirmation: g.require_confirmation,
        maxSlippagePercent: g.max_slippage_percent,
        maxConcentrationPercent: g.max_concentration_percent,
        minHealthFactor: g.min_health_factor,
      };
      securityPreset = detectPreset(guardrails);

      // Messaging
      const m = config.messaging;
      messaging = {
        gmail: { enabled: m.gmail.enabled, autonomy: mapAutonomy(m.gmail.autonomy) },
        whatsapp: { enabled: m.whatsapp.enabled, autonomy: mapAutonomy(m.whatsapp.autonomy) },
        telegram: { enabled: m.telegram.enabled, autonomy: mapAutonomy(m.telegram.autonomy) },
        slack: { enabled: m.slack.enabled, autonomy: mapAutonomy(m.slack.autonomy) },
      };

      // Email
      const e = config.email_notifications;
      emailEnabled = e.enabled;
      emailTimezone = e.timezone;
      emailDigestHour = e.digest_hour;
      emailDigestMinute = e.digest_minute;
      emailTriageStartHour = e.triage_start_hour;
      emailTriageEndHour = e.triage_end_hour;

      // Capabilities
      const c = config.capabilities;
      capabilities = {
        defi_crypto: c.defi_crypto,
        travel: c.travel,
        google_workspace: c.google_workspace,
        email_intelligence: c.email_intelligence,
        communications: c.communications,
        source_intelligence: c.source_intelligence,
        activity_intelligence: c.activity_intelligence ?? false,
      };
      googleAuthenticated = config.google_authenticated;
      ollamaModel = c.ollama_model || '';

      // Take snapshot after loading
      snapshot = currentState();

      // Load Ollama and Docker status in parallel
      try {
        const [ollama, docker, ram]: any = await Promise.all([
          invoke('check_ollama'),
          invoke('docker_status'),
          invoke('get_system_ram'),
        ]);
        ollamaStatus = ollama.available ? 'running' : 'not_installed';
        dockerStatus = docker.toLowerCase().includes('up') ? 'running' : 'stopped';
        systemRam = ram;
        if (ollama.available) {
          const models: any[] = await invoke('list_ollama_models');
          ollamaModels = models;
        }
      } catch {
        ollamaStatus = 'not_installed';
        dockerStatus = 'unknown';
      }

      // Get app version
      try {
        const { getVersion } = await import('@tauri-apps/api/app');
        appVersion = await getVersion();
      } catch {
        appVersion = '';
      }

      // Load ClawdTalk and Claude Code status
      await Promise.all([loadClawdTalkStatus(), loadClaudeCodeStatus(), loadAutonomySettings()]);

    } catch (e: any) {
      loadError = e?.toString() || 'Failed to load settings';
    }
    loading = false;
  }

  function mapAutonomy(a: any): string {
    if (typeof a === 'string') {
      const lower = a.toLowerCase();
      if (lower === 'draftonly' || lower === 'draft_only') return 'draft_only';
      if (lower === 'sendwithconfirm' || lower === 'send_with_confirm') return 'send_with_confirm';
      if (lower === 'autonomous') return 'autonomous';
    }
    return 'draft_only';
  }

  onMount(() => { loadConfig(); });

  // ── Apply security preset ──
  async function applyPreset(preset: string) {
    securityPreset = preset;
    const presets: Record<string, typeof guardrails> = {
      conservative: { maxTransactionUsd: 100, dailyLossPercent: 2, weeklyLossPercent: 5, dailyTxLimit: 10, requireConfirmation: true, maxSlippagePercent: 1, maxConcentrationPercent: 25, minHealthFactor: 2.0 },
      balanced: { maxTransactionUsd: 500, dailyLossPercent: 5, weeklyLossPercent: 15, dailyTxLimit: 20, requireConfirmation: false, maxSlippagePercent: 2, maxConcentrationPercent: 40, minHealthFactor: 1.5 },
      autonomous: { maxTransactionUsd: 1000000, dailyLossPercent: 100, weeklyLossPercent: 100, dailyTxLimit: 1000, requireConfirmation: false, maxSlippagePercent: 50, maxConcentrationPercent: 100, minHealthFactor: 1.0 },
    };
    if (presets[preset]) guardrails = { ...presets[preset] };
  }

  // ── Save settings ──
  async function saveSettings() {
    saving = true;
    saveError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      const update: any = {};

      // Only include changed fields
      const snap = JSON.parse(snapshot);
      if (agentName !== snap.agentName) update.agent_name = agentName;
      if (whatsappPhone !== snap.whatsappPhone) update.whatsapp_phone = whatsappPhone;

      // Guardrails
      if (JSON.stringify(guardrails) !== JSON.stringify(snap.guardrails)) {
        update.guardrails = {
          preset: securityPreset === 'conservative' ? 'Conservative' :
                  securityPreset === 'balanced' ? 'Balanced' :
                  securityPreset === 'autonomous' ? 'Autonomous' : 'Custom',
          max_transaction_usd: guardrails.maxTransactionUsd,
          daily_loss_percent: guardrails.dailyLossPercent,
          weekly_loss_percent: guardrails.weeklyLossPercent,
          daily_tx_limit: guardrails.dailyTxLimit,
          require_confirmation: guardrails.requireConfirmation,
          max_slippage_percent: guardrails.maxSlippagePercent,
          max_concentration_percent: guardrails.maxConcentrationPercent,
          min_health_factor: guardrails.minHealthFactor,
        };
      }

      // Messaging
      if (JSON.stringify(messaging) !== JSON.stringify(snap.messaging)) {
        update.messaging = {
          gmail: { enabled: messaging.gmail.enabled, autonomy: mapAutonomyToEnum(messaging.gmail.autonomy) },
          whatsapp: { enabled: messaging.whatsapp.enabled, autonomy: mapAutonomyToEnum(messaging.whatsapp.autonomy) },
          telegram: { enabled: messaging.telegram.enabled, autonomy: mapAutonomyToEnum(messaging.telegram.autonomy) },
          slack: { enabled: messaging.slack.enabled, autonomy: mapAutonomyToEnum(messaging.slack.autonomy) },
        };
      }

      // Email notifications
      if (emailEnabled !== snap.emailEnabled || emailTimezone !== snap.emailTimezone ||
          emailDigestHour !== snap.emailDigestHour || emailDigestMinute !== snap.emailDigestMinute ||
          emailTriageStartHour !== snap.emailTriageStartHour || emailTriageEndHour !== snap.emailTriageEndHour) {
        update.email_notifications = {
          enabled: emailEnabled,
          timezone: emailTimezone,
          digest_hour: emailDigestHour,
          digest_minute: emailDigestMinute,
          triage_start_hour: emailTriageStartHour,
          triage_end_hour: emailTriageEndHour,
        };
      }

      // Capabilities
      if (JSON.stringify(capabilities) !== JSON.stringify(snap.capabilities) ||
          defaultLlmProvider !== snap.defaultLlmProvider || ollamaModel !== snap.ollamaModel) {
        update.capabilities = {
          defi_crypto: capabilities.defi_crypto,
          travel: capabilities.travel,
          google_workspace: capabilities.google_workspace,
          email_intelligence: capabilities.email_intelligence,
          communications: capabilities.communications,
          source_intelligence: capabilities.source_intelligence,
          activity_intelligence: capabilities.activity_intelligence,
          default_llm_provider: defaultLlmProvider,
          ollama_model: ollamaModel || null,
        };
      }

      const result: any = await invoke('save_settings', { update });

      if (result.success) {
        // If restart required, do it
        if (result.restart_required) {
          try {
            await invoke('restart_container');
          } catch {
            // Container might not be running — that's ok
          }
        }
        // Reload config to refresh snapshot
        await loadConfig();
        saveError = '';
      } else {
        saveError = result.message || 'Save failed';
      }
    } catch (e: any) {
      saveError = e?.toString() || 'Save failed';
    }
    saving = false;
  }

  function mapAutonomyToEnum(a: string): string {
    if (a === 'draft_only') return 'DraftOnly';
    if (a === 'send_with_confirm') return 'SendWithConfirm';
    if (a === 'autonomous') return 'Autonomous';
    return 'DraftOnly';
  }

  function discardChanges() {
    loadConfig();
  }

  // ── Immediate API key save (bypasses save bar) ──
  async function saveApiKey(field: string, value: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const update: any = {};
      update[field] = value;
      await invoke('save_settings', { update });
      // Reload to refresh snapshot
      await loadConfig();
    } catch (e: any) {
      saveError = e?.toString() || 'Failed to save key';
    }
  }

  // ── Ollama functions ──
  async function installOllama() {
    ollamaStatus = 'installing';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('install_ollama');
      let attempts = 0;
      const pollInterval = setInterval(async () => {
        attempts++;
        try {
          const ollama: any = await invoke('check_ollama');
          if (ollama.available) {
            ollamaStatus = 'running';
            clearInterval(pollInterval);
            const models: any[] = await invoke('list_ollama_models');
            ollamaModels = models;
          } else if (attempts >= 20) {
            ollamaStatus = 'not_installed';
            clearInterval(pollInterval);
          }
        } catch {
          if (attempts >= 20) {
            ollamaStatus = 'not_installed';
            clearInterval(pollInterval);
          }
        }
      }, 3000);
    } catch {
      ollamaStatus = 'not_installed';
    }
  }

  async function pullOllamaModel(tag: string) {
    downloadingModel = tag;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('pull_ollama_model', { model: tag });
      const models: any[] = await invoke('list_ollama_models');
      ollamaModels = models;
      if (!ollamaModel) ollamaModel = tag;
    } catch (e: any) {
      console.error('Failed to pull model:', e);
    } finally {
      downloadingModel = null;
    }
  }

  async function pullCustomModel(tag: string) {
    customModelPulling = true;
    await pullOllamaModel(tag);
    customModelPulling = false;
  }

  async function deleteOllamaModel(tag: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_ollama_model', { model: tag });
      const models: any[] = await invoke('list_ollama_models');
      ollamaModels = models;
      if (ollamaModel === tag) {
        ollamaModel = models.length > 0 ? models[0].name : '';
      }
    } catch (e: any) {
      console.error('Failed to delete model:', e);
    }
  }

  // ── Docker controls ──
  async function startDocker() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      dockerStatus = 'starting';
      await invoke('docker_start');
      dockerStatus = 'running';
    } catch {
      dockerStatus = 'error';
    }
  }

  async function stopDocker() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      dockerStatus = 'stopping';
      await invoke('docker_stop');
      dockerStatus = 'stopped';
    } catch {
      dockerStatus = 'error';
    }
  }

  async function restartDocker() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      dockerStatus = 'restarting';
      await invoke('restart_container');
      dockerStatus = 'running';
    } catch {
      dockerStatus = 'error';
    }
  }

  // ── App update functions ──
  async function checkForUpdates() {
    checkingUpdates = true;
    updateError = '';
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (update) {
        updateAvailable = true;
        updateVersion = update.version;
        (window as any).__nyx_update = update;
      } else {
        updateAvailable = false;
        updateVersion = '';
      }
    } catch (e: any) {
      updateError = e?.message || 'Update check failed';
    }
    checkingUpdates = false;
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
      updateProgress = 'Update installed — restart app to apply.';
      updateInstalling = false;
    } catch (e: any) {
      updateError = e?.message || 'Update failed';
      updateInstalling = false;
      updateProgress = '';
    }
  }

  // ── Google auth ──
  async function reauthenticateGoogle() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('run_gog_auth');
      const result: any = await invoke('check_gog_authenticated');
      googleAuthenticated = result;
    } catch {
      // Failed silently
    }
  }

  // ── ClawdTalk voice functions ──
  async function loadClawdTalkStatus() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status: any = await invoke('clawdtalk_status');
      clawdtalkConfigured = status.configured;
      clawdtalkConnected = status.connected;
      clawdtalkHasKey = status.has_api_key;
      clawdtalkServer = status.server;
      clawdtalkPid = status.pid ?? null;
      if (status.configured) {
        try {
          const logs: string[] = await invoke('clawdtalk_logs');
          clawdtalkLogs = logs;
        } catch { /* no logs yet */ }
      }
    } catch {
      // ClawdTalk not available
    }
  }

  async function configureClawdTalk(apiKey: string) {
    clawdtalkSaving = true;
    clawdtalkError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clawdtalk_configure', { apiKey });
      clawdtalkApiKeyEditing = false;
      clawdtalkApiKeyValue = '';
      await loadClawdTalkStatus();
    } catch (e: any) {
      clawdtalkError = e?.toString() || 'Configuration failed';
    }
    clawdtalkSaving = false;
  }

  async function removeClawdTalk() {
    clawdtalkError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clawdtalk_remove');
      await loadClawdTalkStatus();
    } catch (e: any) {
      clawdtalkError = e?.toString() || 'Remove failed';
    }
  }

  async function startClawdTalk() {
    clawdtalkStarting = true;
    clawdtalkError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status: any = await invoke('clawdtalk_start');
      clawdtalkConnected = status.connected;
      clawdtalkPid = status.pid ?? null;
      if (status.configured) {
        const logs: string[] = await invoke('clawdtalk_logs');
        clawdtalkLogs = logs;
      }
    } catch (e: any) {
      clawdtalkError = e?.toString() || 'Start failed';
    }
    clawdtalkStarting = false;
  }

  async function stopClawdTalk() {
    clawdtalkError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status: any = await invoke('clawdtalk_stop');
      clawdtalkConnected = status.connected;
      clawdtalkPid = status.pid ?? null;
    } catch (e: any) {
      clawdtalkError = e?.toString() || 'Stop failed';
    }
  }

  // ── Claude Code functions ──
  async function loadClaudeCodeStatus() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status: any = await invoke('claude_code_status');
      claudeCodeInstalled = status.installed;
      claudeCodeVersion = status.version ?? '';
      claudeCodeMcpRegistered = status.mcp_registered;
      claudeCodeBinaryPath = status.binary_path ?? '';
    } catch {
      // Claude Code detection not available
    }
  }

  async function registerClaudeCodeMcp() {
    claudeCodeRegistering = true;
    claudeCodeError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('claude_code_register_mcp');
      await loadClaudeCodeStatus();
    } catch (e: any) {
      claudeCodeError = e?.toString() || 'Registration failed';
    }
    claudeCodeRegistering = false;
  }

  async function unregisterClaudeCodeMcp() {
    claudeCodeError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('claude_code_unregister_mcp');
      await loadClaudeCodeStatus();
    } catch (e: any) {
      claudeCodeError = e?.toString() || 'Unregister failed';
    }
  }

  // ── Activity Intelligence autonomy ──
  async function loadAutonomySettings() {
    if (!capabilities.activity_intelligence) return;
    autonomyLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const settings: any[] = await invoke('get_autonomy_settings');
      autonomySettings = settings;
    } catch {}
    autonomyLoading = false;
  }

  async function updateAutonomyLevel(activityType: string, level: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_autonomy_level', { activityType, level });
      autonomySettings = autonomySettings.map(s =>
        s.activity_type === activityType ? { ...s, level } : s
      );
    } catch {}
  }

  // ── Helper to open URLs ──
  async function openExternal(url: string) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    } catch {
      window.open(url, '_blank');
    }
  }

  // SVG icon paths
  const icons = {
    user: 'M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z',
    key: 'M15.75 5.25a3 3 0 013 3m3 0a6 6 0 01-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1121.75 8.25z',
    shield: 'M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z',
    chat: 'M8.625 12a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H8.25m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0H12m4.125 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm0 0h-.375M21 12c0 4.556-4.03 8.25-9 8.25a9.764 9.764 0 01-2.555-.337A5.972 5.972 0 015.41 20.97a5.969 5.969 0 01-.474-.065 4.48 4.48 0 00.978-2.025c.09-.457-.133-.901-.467-1.226C3.93 16.178 3 14.189 3 12c0-4.556 4.03-8.25 9-8.25s9 3.694 9 8.25z',
    email: 'M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75',
    capabilities: 'M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z',
    update: 'M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3',
    server: 'M21.75 17.25v-.228a4.5 4.5 0 00-.12-1.03l-2.268-9.64a3.375 3.375 0 00-3.285-2.602H7.923a3.375 3.375 0 00-3.285 2.602l-2.268 9.64a4.5 4.5 0 00-.12 1.03v.228m19.5 0a3 3 0 01-3 3H5.25a3 3 0 01-3-3m19.5 0a3 3 0 00-3-3H5.25a3 3 0 00-3 3m16.5 0h.008v.008h-.008v-.008zm-3 0h.008v.008h-.008v-.008z',
    phone: 'M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 002.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 01-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.363-.271.527-.734.417-1.173L6.963 3.102a1.125 1.125 0 00-1.091-.852H4.5A2.25 2.25 0 002.25 4.5v2.25z',
    terminal: 'M6.75 7.5l3 2.25-3 2.25m4.5 0h3m-9 8.25h12a2.25 2.25 0 002.25-2.25V5.25A2.25 2.25 0 0018 3H6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 006 21z',
  };
</script>

<div class="h-full overflow-y-auto">
  <div class="max-w-2xl mx-auto px-6 py-8 pb-24">
    <!-- Header -->
    <div class="mb-8">
      <h1 class="font-display text-2xl font-light tracking-wider text-ivory mb-1">Settings</h1>
      <p class="text-ivory-muted text-sm">Configure your agent, providers, security, and integrations.</p>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div class="w-6 h-6 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
        <span class="text-ivory-muted text-sm ml-3">Loading settings...</span>
      </div>
    {:else if loadError}
      <div class="text-center py-20">
        <p class="text-negative text-sm mb-3">{loadError}</p>
        <button onclick={loadConfig} class="text-gold text-xs hover:underline">Retry</button>
      </div>
    {:else}
      <div class="space-y-3">

        <!-- ═══════════════ 1. Agent Identity ═══════════════ -->
        <SettingsSection title="Agent Identity" icon={icons.user} expanded={true}>
          <div class="space-y-3">
            <div>
              <label class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">Agent Name</label>
              <input
                type="text"
                bind:value={agentName}
                placeholder="Nyx"
                maxlength="32"
                class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors duration-300 selectable"
              />
              <p class="text-ivory-muted/40 text-[10px] mt-1">Changing the agent name requires a container restart.</p>
            </div>
          </div>
        </SettingsSection>

        <!-- ═══════════════ 2. LLM Providers ═══════════════ -->
        <SettingsSection title="LLM Providers" icon={icons.key} expanded={true}>
          <div class="space-y-5">
            <!-- API key statuses -->
            <div>
              <p class="text-ivory-muted/50 text-xs mb-2">API keys are stored securely and never displayed.</p>
              <div class="divide-y divide-border/50">
                <ApiKeyStatus
                  provider="Anthropic Claude"
                  description="Recommended"
                  configured={hasAnthropicKey}
                  helpUrl="https://console.anthropic.com/settings/keys"
                  placeholder="sk-ant-..."
                  onSave={(key) => { hasAnthropicKey = true; saveApiKey('anthropic_key', key); }}
                />
                <ApiKeyStatus
                  provider="OpenAI"
                  description="Voice, GPT-4o"
                  configured={hasOpenaiKey}
                  helpUrl="https://platform.openai.com/api-keys"
                  placeholder="sk-proj-..."
                  onSave={(key) => { hasOpenaiKey = true; saveApiKey('openai_key', key); }}
                />
                <ApiKeyStatus
                  provider="Venice AI"
                  description="Privacy-first"
                  configured={hasVeniceKey}
                  helpUrl="https://venice.ai/settings/api"
                  placeholder="venice-..."
                  onSave={(key) => { hasVeniceKey = true; saveApiKey('venice_key', key); }}
                />
                <ApiKeyStatus
                  provider="NEAR.ai"
                  description="TEE Confidential"
                  configured={hasNearaiKey}
                  helpUrl="https://cloud.near.ai"
                  placeholder="NEAR.ai API key"
                  onSave={(key) => { hasNearaiKey = true; saveApiKey('nearai_key', key); }}
                />
              </div>
            </div>

            <!-- Default LLM selector -->
            <div>
              <p class="text-ivory-muted text-xs tracking-wider uppercase mb-2">Default LLM</p>
              <div class="space-y-1.5">
                {#each [
                  { id: 'anthropic', label: 'Anthropic Claude', abbr: 'An', hasKey: hasAnthropicKey, badge: 'Recommended' },
                  { id: 'openai', label: 'OpenAI GPT-4o', abbr: 'Oa', hasKey: hasOpenaiKey },
                  { id: 'venice', label: 'Venice AI', abbr: 'Ve', hasKey: hasVeniceKey },
                  { id: 'nearai', label: 'NEAR.ai', abbr: 'Na', hasKey: hasNearaiKey },
                ] as provider}
                  <button
                    onclick={() => { defaultLlmProvider = provider.id; }}
                    class="w-full flex items-center gap-3 px-3 py-2 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === provider.id ? 'border-gold bg-gold/5' : 'border-border hover:border-ivory-muted/30'}"
                  >
                    <div class="w-6 h-6 rounded-md flex items-center justify-center text-[10px] font-bold {defaultLlmProvider === provider.id ? 'bg-gold/20 text-gold' : 'bg-ivory-muted/10 text-ivory-muted'}">{provider.abbr}</div>
                    <div class="flex-1">
                      <span class="text-ivory text-sm">{provider.label}</span>
                      {#if provider.badge && provider.hasKey}
                        <span class="text-gold text-[10px] uppercase tracking-wider ml-1">{provider.badge}</span>
                      {:else if !provider.hasKey}
                        <span class="text-ivory-muted/40 text-[10px] ml-1">Add key above to use</span>
                      {/if}
                    </div>
                    <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === provider.id ? 'border-gold' : 'border-border'}">
                      {#if defaultLlmProvider === provider.id}<div class="w-1.5 h-1.5 rounded-full bg-gold"></div>{/if}
                    </div>
                  </button>
                {/each}
                <!-- Ollama option -->
                <button
                  onclick={() => { defaultLlmProvider = 'ollama'; }}
                  class="w-full flex items-center gap-3 px-3 py-2 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'ollama' ? 'border-accent bg-accent/5' : 'border-border hover:border-ivory-muted/30'}"
                >
                  <div class="w-6 h-6 rounded-md flex items-center justify-center {defaultLlmProvider === 'ollama' ? 'bg-accent/20 text-accent' : 'bg-ivory-muted/10 text-ivory-muted'}">
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d="M8.25 3v1.5M4.5 8.25H3m18 0h-1.5M4.5 12H3m18 0h-1.5m-15 3.75H3m18 0h-1.5M8.25 19.5V21M12 3v1.5m0 15V21m3.75-18v1.5m0 15V21m-9-1.5h9a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0015.75 4.5h-9A2.25 2.25 0 004.5 6.75v10.5A2.25 2.25 0 006.75 19.5z" />
                    </svg>
                  </div>
                  <div class="flex-1">
                    <span class="text-ivory text-sm">Local (Ollama)</span>
                    {#if ollamaModels.length > 0}
                      <span class="text-accent text-[10px] uppercase tracking-wider ml-1">Private</span>
                    {:else if ollamaStatus === 'running'}
                      <span class="text-ivory-muted/40 text-[10px] ml-1">Download a model below</span>
                    {:else}
                      <span class="text-ivory-muted/40 text-[10px] ml-1">Install Ollama below</span>
                    {/if}
                  </div>
                  <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'ollama' ? 'border-accent' : 'border-border'}">
                    {#if defaultLlmProvider === 'ollama'}<div class="w-1.5 h-1.5 rounded-full bg-accent"></div>{/if}
                  </div>
                </button>
              </div>
            </div>

            <!-- Ollama section (when Ollama is selected or models exist) -->
            {#if defaultLlmProvider === 'ollama' || ollamaModels.length > 0}
              <div class="border-l-2 border-accent/20 pl-3 space-y-3">
                {#if ollamaStatus !== 'running' && ollamaStatus !== 'installing'}
                  <div class="p-3 rounded-lg border border-accent/20 bg-surface">
                    <p class="text-ivory-muted text-xs mb-2">Ollama is not installed. Install it to use local AI models on your Mac.</p>
                    <button
                      onclick={installOllama}
                      class="px-3 py-1.5 text-[10px] tracking-wider uppercase rounded border border-accent text-accent hover:bg-accent/10 transition-colors"
                    >
                      Install Ollama
                    </button>
                  </div>
                {:else if ollamaStatus === 'installing'}
                  <div class="p-3 rounded-lg border border-accent/20 bg-surface">
                    <div class="flex items-center gap-2">
                      <div class="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin"></div>
                      <p class="text-ivory-muted text-xs">Installing Ollama...</p>
                    </div>
                  </div>
                {:else}
                  <!-- Active model selector (when models exist) -->
                  {#if ollamaModels.length > 0}
                    <div>
                      <p class="text-ivory-muted text-xs mb-1.5">Active model:</p>
                      <select
                        bind:value={ollamaModel}
                        class="w-full bg-surface text-ivory text-sm px-3 py-2 rounded border border-border focus:border-accent focus:outline-none transition-colors"
                      >
                        {#each ollamaModels as model}
                          <option value={model.name}>{model.name} ({(model.size / 1e9).toFixed(1)} GB)</option>
                        {/each}
                      </select>
                    </div>
                  {/if}

                  <!-- Installed models list -->
                  {#if ollamaModels.length > 0}
                    <div>
                      <p class="text-ivory-muted text-xs mb-1.5">Installed models:</p>
                      <div class="space-y-1">
                        {#each ollamaModels as model}
                          <div class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface border border-accent/15">
                            <div class="flex items-center gap-2">
                              <svg class="w-3.5 h-3.5 text-positive" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                              </svg>
                              <span class="text-ivory text-xs">{model.name}</span>
                              <span class="text-ivory-muted/40 text-[10px]">{(model.size / 1e9).toFixed(1)} GB</span>
                            </div>
                            <button
                              onclick={() => deleteOllamaModel(model.name)}
                              class="text-ivory-muted/30 hover:text-negative text-xs transition-colors p-1"
                              title="Remove model"
                            >
                              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                                <path d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                              </svg>
                            </button>
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}

                  <!-- Recommended models grid -->
                  <div>
                    <p class="text-ivory-muted text-xs mb-1.5">Recommended models:</p>
                    <div class="space-y-1">
                      {#each recommendedModels as model}
                        <LocalModelCard
                          name={model.name}
                          tag={model.tag}
                          size={model.size}
                          description={model.description}
                          minRam={model.minRam}
                          systemRam={systemRam}
                          installed={ollamaModels.some(m => m.name.startsWith(model.tag.split(':')[0]) && m.name.includes(model.tag.split(':')[1] || ''))}
                          downloading={downloadingModel === model.tag}
                          onDownload={() => pullOllamaModel(model.tag)}
                          onDelete={() => deleteOllamaModel(model.tag)}
                        />
                      {/each}
                    </div>
                  </div>

                  <!-- Custom model input -->
                  <CustomModelInput
                    onPull={pullCustomModel}
                    pulling={customModelPulling}
                  />

                  <!-- RAM guidance -->
                  {#if systemRam > 0}
                    <div class="px-3 py-2 rounded-lg bg-surface border border-accent/15">
                      <div class="flex items-center gap-2 text-xs">
                        <svg class="w-3.5 h-3.5 text-accent flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                          <path d="M8.25 3v1.5M4.5 8.25H3m18 0h-1.5M4.5 12H3m18 0h-1.5m-15 3.75H3m18 0h-1.5M8.25 19.5V21M12 3v1.5m0 15V21m3.75-18v1.5m0 15V21m-9-1.5h9a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0015.75 4.5h-9A2.25 2.25 0 004.5 6.75v10.5A2.25 2.25 0 006.75 19.5z" />
                        </svg>
                        <span class="text-ivory-muted">
                          {systemRam}GB RAM detected.
                          {#if systemRam < 8}
                            Your system may not have enough RAM for local models.
                          {:else if systemRam <= 16}
                            Suitable for smaller models (1-8B parameters).
                          {:else}
                            Great for local models.
                          {/if}
                        </span>
                      </div>
                    </div>
                  {/if}
                {/if}
              </div>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 3. DeFi Security ═══════════════ -->
        <SettingsSection title="DeFi Security" icon={icons.shield}>
          <div class="space-y-4">
            <p class="text-ivory-muted/50 text-xs">Transaction limits and guardrails for crypto operations.</p>
            <div class="grid grid-cols-4 gap-2">
              <SecurityPresetCard
                name="Conservative"
                description="Tight limits, always confirms."
                selected={securityPreset === 'conservative'}
                onclick={() => applyPreset('conservative')}
                metrics={{ maxTx: '$100', dailyLoss: '2%', confirmation: 'Required' }}
              />
              <SecurityPresetCard
                name="Balanced"
                description="Moderate limits."
                selected={securityPreset === 'balanced'}
                onclick={() => applyPreset('balanced')}
                metrics={{ maxTx: '$500', dailyLoss: '5%', confirmation: 'Off' }}
              />
              <SecurityPresetCard
                name="Autonomous"
                description="No limits."
                selected={securityPreset === 'autonomous'}
                onclick={() => applyPreset('autonomous')}
                metrics={{ maxTx: 'No limit', dailyLoss: 'No limit', confirmation: 'Off' }}
              />
              <SecurityPresetCard
                name="Custom"
                description="Set your own limits."
                selected={securityPreset === 'custom'}
                onclick={() => { securityPreset = 'custom'; }}
                metrics={{ maxTx: `$${guardrails.maxTransactionUsd.toLocaleString()}`, dailyLoss: `${guardrails.dailyLossPercent}%`, confirmation: guardrails.requireConfirmation ? 'Required' : 'Off' }}
              />
            </div>

            {#if securityPreset === 'custom'}
              <div class="p-4 rounded-lg border border-gold/20 bg-surface-raised space-y-3">
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Max Transaction (USD)</label>
                    <input type="number" bind:value={guardrails.maxTransactionUsd} min="0" step="100"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Daily Loss Limit (%)</label>
                    <input type="number" bind:value={guardrails.dailyLossPercent} min="0" max="100" step="1"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Weekly Loss Limit (%)</label>
                    <input type="number" bind:value={guardrails.weeklyLossPercent} min="0" max="100" step="1"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Max Daily Transactions</label>
                    <input type="number" bind:value={guardrails.dailyTxLimit} min="1" step="1"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Max Slippage (%)</label>
                    <input type="number" bind:value={guardrails.maxSlippagePercent} min="0" max="100" step="0.5"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Max Concentration (%)</label>
                    <input type="number" bind:value={guardrails.maxConcentrationPercent} min="0" max="100" step="5"
                      class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold focus:outline-none" />
                  </div>
                </div>
                <div class="flex items-center gap-2 pt-1">
                  <input type="checkbox" id="settings-require-confirm" bind:checked={guardrails.requireConfirmation}
                    class="accent-gold w-3.5 h-3.5" />
                  <label for="settings-require-confirm" class="text-ivory-muted text-xs">Require confirmation before transactions</label>
                </div>
              </div>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 4. Messaging Channels ═══════════════ -->
        <SettingsSection title="Messaging Channels" icon={icons.chat}>
          <div class="space-y-3">
            <p class="text-ivory-muted/50 text-xs">Enable channels for the agent to send messages on your behalf.</p>
            <ChannelCard
              name="Gmail"
              enabled={messaging.gmail.enabled}
              autonomy={messaging.gmail.autonomy}
              onToggle={(v) => messaging.gmail.enabled = v}
              onAutonomyChange={(v) => messaging.gmail.autonomy = v}
            />
            <ChannelCard
              name="WhatsApp"
              enabled={messaging.whatsapp.enabled}
              autonomy={messaging.whatsapp.autonomy}
              onToggle={(v) => messaging.whatsapp.enabled = v}
              onAutonomyChange={(v) => messaging.whatsapp.autonomy = v}
            />
            {#if messaging.whatsapp.enabled}
              <div class="ml-11 -mt-1">
                <label class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">
                  Your Phone Number <span class="text-gold">*</span>
                </label>
                <input
                  type="tel"
                  bind:value={whatsappPhone}
                  placeholder="+44..."
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {/^\+\d{7,15}$/.test(whatsappPhone) ? 'border-positive/50' : 'border-border focus:border-gold-dim'}"
                />
                <p class="text-ivory-muted/50 text-xs mt-1">E.164 format with country code.</p>
              </div>
            {/if}
            <ChannelCard
              name="Telegram"
              enabled={messaging.telegram.enabled}
              autonomy={messaging.telegram.autonomy}
              onToggle={(v) => messaging.telegram.enabled = v}
              onAutonomyChange={(v) => messaging.telegram.autonomy = v}
            />
            {#if messaging.telegram.enabled}
              <div class="ml-11 -mt-1">
                <ApiKeyStatus
                  provider="Telegram Bot Token"
                  configured={hasTelegramToken}
                  helpUrl="https://t.me/BotFather"
                  placeholder="123456:ABC-..."
                  onSave={(key) => { hasTelegramToken = true; saveApiKey('telegram_token', key); }}
                />
              </div>
            {/if}
            <ChannelCard
              name="Slack"
              enabled={messaging.slack.enabled}
              autonomy={messaging.slack.autonomy}
              onToggle={(v) => messaging.slack.enabled = v}
              onAutonomyChange={(v) => messaging.slack.autonomy = v}
            />
            {#if messaging.slack.enabled}
              <div class="ml-11 -mt-1">
                <ApiKeyStatus
                  provider="Slack Bot Token"
                  configured={hasSlackToken}
                  helpUrl="https://api.slack.com/apps"
                  placeholder="xoxb-..."
                  onSave={(key) => { hasSlackToken = true; saveApiKey('slack_token', key); }}
                />
              </div>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 5. Voice Calling (ClawdTalk) ═══════════════ -->
        <SettingsSection title="Voice Calling" icon={icons.phone}>
          <div class="space-y-4">
            <!-- Privacy Warning Banner -->
            <div class="p-3 rounded-lg border border-warning/20 bg-warning/5">
              <div class="flex items-start gap-2.5">
                <svg class="w-4 h-4 text-warning flex-shrink-0 mt-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126z" />
                  <path d="M12 15.75h.008v.008H12v-.008z" />
                </svg>
                <div>
                  <p class="text-warning text-xs font-medium mb-1">Privacy Notice</p>
                  <p class="text-ivory-muted/70 text-[10px] leading-relaxed">
                    Voice calls are processed through Telnyx cloud infrastructure (ClawdTalk). Your voice audio, transcribed speech, and agent responses pass through their servers for speech-to-text and text-to-speech processing. Tool execution and secrets remain local. Do not discuss private keys, seed phrases, or sensitive credentials on voice calls.
                  </p>
                  <button
                    onclick={() => openExternal('https://clawdtalk.com')}
                    class="text-gold-dim hover:text-gold text-[10px] mt-1.5 transition-colors"
                  >
                    Learn more at clawdtalk.com
                  </button>
                </div>
              </div>
            </div>

            {#if !clawdtalkConfigured}
              <!-- Not configured: show setup prompt -->
              <div class="p-4 rounded-lg bg-surface border border-border">
                <p class="text-ivory text-sm mb-1">Enable voice calling</p>
                <p class="text-ivory-muted/50 text-xs mb-4">Let your agent make and receive phone calls. Requires a free ClawdTalk API key.</p>

                <div>
                  <div class="flex items-center justify-between mb-1.5">
                    <label class="text-ivory-muted text-xs">ClawdTalk API Key</label>
                    <button
                      onclick={() => openExternal('https://clawdtalk.com')}
                      class="flex items-center gap-1 text-gold-dim hover:text-gold text-xs transition-colors"
                    >
                      <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                      </svg>
                      Get Key
                    </button>
                  </div>
                  <div class="flex gap-2">
                    <input
                      type="password"
                      bind:value={clawdtalkApiKeyValue}
                      placeholder="Your ClawdTalk API key"
                      class="flex-1 bg-surface text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors duration-300 selectable"
                    />
                    <button
                      onclick={() => configureClawdTalk(clawdtalkApiKeyValue)}
                      disabled={!clawdtalkApiKeyValue || clawdtalkSaving}
                      class="px-4 py-2.5 text-xs tracking-wider uppercase rounded border transition-all duration-200 {clawdtalkApiKeyValue && !clawdtalkSaving ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted/30 cursor-not-allowed'}"
                    >
                      {#if clawdtalkSaving}
                        <div class="w-4 h-4 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                      {:else}
                        Enable
                      {/if}
                    </button>
                  </div>
                </div>
              </div>
            {:else}
              <!-- Configured: show status and controls -->
              <div class="space-y-3">
                <!-- Connection status -->
                <div class="flex items-center justify-between px-3 py-2.5 rounded-lg bg-surface border border-border/50">
                  <div class="flex items-center gap-3">
                    <div class="w-7 h-7 rounded-lg flex items-center justify-center text-[10px] font-bold flex-shrink-0 {clawdtalkConnected ? 'bg-positive/15 text-positive' : 'bg-ivory-muted/10 text-ivory-muted'}">
                      <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                        <path d={icons.phone} />
                      </svg>
                    </div>
                    <div class="min-w-0">
                      <div class="text-ivory text-xs font-medium">Voice Connection</div>
                      <div class="text-[10px] {clawdtalkConnected ? 'text-positive' : 'text-ivory-muted/50'}">
                        {clawdtalkConnected ? `Connected (PID: ${clawdtalkPid})` : 'Disconnected'}
                      </div>
                    </div>
                  </div>
                  <div class="flex items-center gap-2">
                    {#if clawdtalkConnected}
                      <button
                        onclick={stopClawdTalk}
                        class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-negative/30 text-negative/70 hover:text-negative hover:border-negative/50 transition-colors"
                      >
                        Disconnect
                      </button>
                    {:else if clawdtalkStarting}
                      <div class="flex items-center gap-2 text-xs text-ivory-muted">
                        <div class="w-3 h-3 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                        Connecting...
                      </div>
                    {:else}
                      <button
                        onclick={startClawdTalk}
                        class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-positive/30 text-positive/70 hover:text-positive hover:border-positive/50 transition-colors"
                      >
                        Connect
                      </button>
                    {/if}
                  </div>
                </div>

                <!-- API Key status -->
                <div class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface-raised">
                  <div class="flex items-center gap-2 text-xs">
                    <span class="text-ivory-muted">API Key:</span>
                    {#if clawdtalkHasKey}
                      <span class="text-positive flex items-center gap-1">
                        <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                          <path d="M4.5 12.75l6 6 9-13.5" />
                        </svg>
                        Configured
                      </span>
                    {:else}
                      <span class="text-warning text-[10px]">Not set — add key in docker.env</span>
                    {/if}
                  </div>
                  {#if !clawdtalkApiKeyEditing}
                    <button
                      onclick={() => clawdtalkApiKeyEditing = true}
                      class="text-[10px] tracking-wider uppercase text-gold-dim hover:text-gold transition-colors"
                    >
                      Update Key
                    </button>
                  {/if}
                </div>

                <!-- Update key form (inline) -->
                {#if clawdtalkApiKeyEditing}
                  <div class="flex gap-2 px-3">
                    <input
                      type="password"
                      bind:value={clawdtalkApiKeyValue}
                      placeholder="New ClawdTalk API key"
                      class="flex-1 bg-surface text-ivory text-xs px-3 py-2 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors selectable"
                    />
                    <button
                      onclick={() => configureClawdTalk(clawdtalkApiKeyValue)}
                      disabled={!clawdtalkApiKeyValue || clawdtalkSaving}
                      class="px-3 py-2 text-[10px] tracking-wider uppercase rounded border border-gold/40 text-gold hover:bg-gold/10 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                    >
                      Save
                    </button>
                    <button
                      onclick={() => { clawdtalkApiKeyEditing = false; clawdtalkApiKeyValue = ''; }}
                      class="px-3 py-2 text-[10px] text-ivory-muted hover:text-ivory transition-colors"
                    >
                      Cancel
                    </button>
                  </div>
                {/if}

                <!-- Recent logs -->
                {#if clawdtalkLogs.length > 0}
                  <div class="px-3">
                    <p class="text-ivory-muted/40 text-[10px] tracking-wider uppercase mb-1">Recent Activity</p>
                    <div class="bg-black/30 rounded p-2 max-h-24 overflow-y-auto">
                      {#each clawdtalkLogs.slice(-5) as line}
                        <p class="text-ivory-muted/50 text-[10px] font-mono leading-relaxed truncate">{line}</p>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Remove -->
                <div class="px-3 pt-1">
                  <button
                    onclick={removeClawdTalk}
                    class="text-[10px] text-ivory-muted/30 hover:text-negative transition-colors"
                  >
                    Remove voice calling
                  </button>
                </div>
              </div>
            {/if}

            {#if clawdtalkError}
              <p class="text-negative text-xs px-1">{clawdtalkError}</p>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 5b. Claude Code (MCP Integration) ═══════════════ -->
        <SettingsSection title="Claude Code" icon={icons.terminal}>
          <div class="space-y-4">
            <p class="text-ivory-muted/50 text-xs leading-relaxed">
              Register Nyx as an MCP server so Claude Code can access your portfolio, chat with Atlas, verify sources, and more.
            </p>

            <!-- CLI status -->
            <div class="flex items-center justify-between px-3 py-2.5 rounded-lg bg-surface border border-border/50">
              <div class="flex items-center gap-3">
                <div class="w-7 h-7 rounded-lg flex items-center justify-center text-[10px] font-bold flex-shrink-0 {claudeCodeInstalled ? 'bg-positive/15 text-positive' : 'bg-ivory-muted/10 text-ivory-muted'}">
                  <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d={icons.terminal} />
                  </svg>
                </div>
                <div class="min-w-0">
                  <div class="text-ivory text-xs font-medium">Claude Code CLI</div>
                  <div class="text-[10px] {claudeCodeInstalled ? 'text-positive' : 'text-ivory-muted/50'}">
                    {claudeCodeInstalled ? `Installed${claudeCodeVersion ? ` (${claudeCodeVersion})` : ''}` : 'Not found'}
                  </div>
                </div>
              </div>
              {#if !claudeCodeInstalled}
                <button
                  onclick={() => openExternal('https://docs.anthropic.com/en/docs/claude-code')}
                  class="flex items-center gap-1 text-gold-dim hover:text-gold text-xs transition-colors"
                >
                  <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                  </svg>
                  Install
                </button>
              {/if}
            </div>

            {#if claudeCodeInstalled}
              <!-- MCP registration status -->
              <div class="flex items-center justify-between px-3 py-2.5 rounded-lg bg-surface border border-border/50">
                <div class="flex items-center gap-3">
                  <div class="w-7 h-7 rounded-lg flex items-center justify-center text-[10px] font-bold flex-shrink-0 {claudeCodeMcpRegistered ? 'bg-positive/15 text-positive' : 'bg-ivory-muted/10 text-ivory-muted'}">
                    <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                      <path d={icons.server} />
                    </svg>
                  </div>
                  <div class="min-w-0">
                    <div class="text-ivory text-xs font-medium">Nyx MCP Server</div>
                    <div class="text-[10px] {claudeCodeMcpRegistered ? 'text-positive' : 'text-ivory-muted/50'}">
                      {claudeCodeMcpRegistered ? 'Registered' : 'Not registered'}
                    </div>
                  </div>
                </div>
                <div class="flex items-center gap-2">
                  {#if claudeCodeRegistering}
                    <div class="flex items-center gap-2 text-xs text-ivory-muted">
                      <div class="w-3 h-3 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                      Registering...
                    </div>
                  {:else if claudeCodeMcpRegistered}
                    <button
                      onclick={unregisterClaudeCodeMcp}
                      class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-negative/30 text-negative/70 hover:text-negative hover:border-negative/50 transition-colors"
                    >
                      Unregister
                    </button>
                  {:else}
                    <button
                      onclick={registerClaudeCodeMcp}
                      class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-positive/30 text-positive/70 hover:text-positive hover:border-positive/50 transition-colors"
                    >
                      Register
                    </button>
                  {/if}
                </div>
              </div>

              <!-- Tools info -->
              {#if claudeCodeMcpRegistered}
                <div class="px-3">
                  <p class="text-ivory-muted/40 text-[10px] tracking-wider uppercase mb-1.5">Available Tools</p>
                  <div class="grid grid-cols-2 gap-1.5">
                    {#each [
                      { name: 'nyx_chat', desc: 'Chat with Atlas' },
                      { name: 'nyx_portfolio', desc: 'DeFi portfolio' },
                      { name: 'nyx_verify_source', desc: 'Source verification' },
                      { name: 'nyx_docker_status', desc: 'Container status' },
                      { name: 'nyx_sessions', desc: 'Session management' },
                      { name: 'nyx_zec_quote', desc: 'ZEC shield quotes' },
                    ] as tool}
                      <div class="flex items-center gap-1.5 text-[10px]">
                        <span class="text-positive">&#x2713;</span>
                        <span class="text-ivory-muted font-mono">{tool.name}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            {/if}

            {#if claudeCodeError}
              <p class="text-negative text-xs px-1">{claudeCodeError}</p>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 6. Email Schedule ═══════════════ -->
        <SettingsSection title="Email Notifications" icon={icons.email}>
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-ivory text-sm">Email intelligence</p>
                <p class="text-ivory-muted/50 text-xs">Daily digest and hourly triage of your inbox.</p>
              </div>
              <button
                onclick={() => emailEnabled = !emailEnabled}
                class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0 overflow-hidden"
                class:bg-accent={emailEnabled}
                class:bg-border={!emailEnabled}
              >
                <span
                  class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-ivory transition-transform duration-200"
                  style="transform: translateX({emailEnabled ? '20px' : '0px'})"
                ></span>
              </button>
            </div>

            {#if emailEnabled}
              <div class="space-y-3 pl-0">
                <div>
                  <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Timezone</label>
                  <input type="text" bind:value={emailTimezone} placeholder="Europe/London"
                    class="w-full px-3 py-1.5 text-xs bg-surface border border-border rounded text-ivory focus:border-gold-dim focus:outline-none selectable" />
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Daily Digest Time</label>
                    <div class="flex items-center gap-1">
                      <input type="number" bind:value={emailDigestHour} min="0" max="23" class="w-16 px-2 py-1.5 text-xs bg-surface border border-border rounded text-ivory text-center focus:border-gold-dim focus:outline-none" />
                      <span class="text-ivory-muted">:</span>
                      <input type="number" bind:value={emailDigestMinute} min="0" max="59" class="w-16 px-2 py-1.5 text-xs bg-surface border border-border rounded text-ivory text-center focus:border-gold-dim focus:outline-none" />
                    </div>
                  </div>
                  <div>
                    <label class="text-ivory-muted/70 text-[10px] tracking-wider uppercase block mb-1">Triage Window</label>
                    <div class="flex items-center gap-1">
                      <input type="number" bind:value={emailTriageStartHour} min="0" max="23" class="w-16 px-2 py-1.5 text-xs bg-surface border border-border rounded text-ivory text-center focus:border-gold-dim focus:outline-none" />
                      <span class="text-ivory-muted text-xs">to</span>
                      <input type="number" bind:value={emailTriageEndHour} min="0" max="23" class="w-16 px-2 py-1.5 text-xs bg-surface border border-border rounded text-ivory text-center focus:border-gold-dim focus:outline-none" />
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 6. Capabilities ═══════════════ -->
        <SettingsSection title="Capabilities" icon={icons.capabilities}>
          <div class="space-y-3">
            <p class="text-ivory-muted/50 text-xs">Enable or disable capability domains for your agent.</p>
            {#each [
              { key: 'defi_crypto', abbr: 'Fi', name: 'DeFi & Crypto', desc: 'Cross-chain swaps, staking, lending, portfolio management', color: 'bg-emerald-500/15 text-emerald-400' },
              { key: 'travel', abbr: 'Tr', name: 'Travel', desc: 'Flight, hotel, and transport research', color: 'bg-blue-400/15 text-blue-300' },
              { key: 'google_workspace', abbr: 'Gw', name: 'Google Workspace', desc: 'Gmail, Calendar, Drive, Docs integration', color: 'bg-red-500/15 text-red-400' },
              { key: 'email_intelligence', abbr: 'Em', name: 'Email Intelligence', desc: 'Priority triage, daily digest, inbox awareness', color: 'bg-amber-500/15 text-amber-300' },
              { key: 'communications', abbr: 'Co', name: 'Communications', desc: 'Telegram, WhatsApp, Slack with autonomy controls', color: 'bg-purple-500/15 text-purple-400' },
              { key: 'source_intelligence', abbr: 'Ve', name: 'Source Verification', desc: 'Credibility analysis and fact-checking', color: 'bg-cyan-500/15 text-cyan-300' },
              { key: 'activity_intelligence', abbr: 'Ai', name: 'Activity Intelligence', desc: 'Observe calendar & email patterns to offer proactive suggestions', color: 'bg-rose-500/15 text-rose-300' },
            ] as cap}
              <div class="flex items-center justify-between px-3 py-2.5 rounded-lg bg-surface border border-border/50">
                <div class="flex items-center gap-3">
                  <div class="w-7 h-7 rounded-lg flex items-center justify-center text-[10px] font-bold flex-shrink-0 {cap.color}">
                    {cap.abbr}
                  </div>
                  <div class="min-w-0">
                    <div class="text-ivory text-xs font-medium">{cap.name}</div>
                    <div class="text-ivory-muted/50 text-[10px] truncate">{cap.desc}</div>
                  </div>
                </div>
                <button
                  onclick={() => capabilities[cap.key as keyof typeof capabilities] = !capabilities[cap.key as keyof typeof capabilities]}
                  class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0 overflow-hidden"
                  class:bg-accent={capabilities[cap.key as keyof typeof capabilities]}
                  class:bg-border={!capabilities[cap.key as keyof typeof capabilities]}
                >
                  <span
                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-ivory transition-transform duration-200"
                    style="transform: translateX({capabilities[cap.key as keyof typeof capabilities] ? '20px' : '0px'})"
                  ></span>
                </button>
              </div>
            {/each}

            <!-- Google Workspace auth status -->
            {#if capabilities.google_workspace}
              <div class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface-raised">
                <div class="flex items-center gap-2 text-xs">
                  <span class="text-ivory-muted">Google Workspace:</span>
                  {#if googleAuthenticated}
                    <span class="text-positive flex items-center gap-1">
                      <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path d="M4.5 12.75l6 6 9-13.5" />
                      </svg>
                      Authenticated
                    </span>
                  {:else}
                    <span class="text-ivory-muted/50">Not authenticated</span>
                  {/if}
                </div>
                <button
                  onclick={reauthenticateGoogle}
                  class="text-[10px] tracking-wider uppercase text-gold-dim hover:text-gold transition-colors"
                >
                  {googleAuthenticated ? 'Re-authenticate' : 'Authenticate'}
                </button>
              </div>
            {/if}

            <!-- Activity Intelligence privacy notice + autonomy controls -->
            {#if capabilities.activity_intelligence}
              <div class="px-3 py-2.5 rounded-lg bg-rose-500/5 border border-rose-500/20">
                <div class="flex items-start gap-2">
                  <svg class="w-3.5 h-3.5 text-rose-300 mt-0.5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
                  </svg>
                  <div>
                    <p class="text-ivory-muted/70 text-[10px] leading-relaxed">
                      When enabled, Nyx periodically reads your calendar events and email metadata (sender, subject, timestamps &mdash; <strong class="text-ivory-muted">not email bodies</strong>) to build a local behavioural model. All data stays on your machine in <code class="text-rose-300/60">~/.nyx/intelligence.db</code>. Disable at any time to stop observation.
                    </p>
                    <button
                      onclick={async () => { if (confirm('Delete all collected Activity Intelligence data? This cannot be undone.')) { try { const { invoke } = await import('@tauri-apps/api/core'); await invoke('clear_intelligence_data'); } catch {} } }}
                      class="mt-1.5 text-[9px] tracking-wider uppercase text-rose-300/50 hover:text-rose-300 transition-colors"
                    >
                      Clear collected data
                    </button>
                  </div>
                </div>
              </div>

              <!-- Autonomy levels -->
              {#if autonomySettings.length > 0}
                <div class="px-3 py-3 rounded-lg bg-surface border border-border/50">
                  <h4 class="text-ivory-muted text-[10px] tracking-widest uppercase mb-3">Autonomy Levels</h4>
                  <div class="space-y-3">
                    {#each [
                      { type: 'scheduling', label: 'Scheduling', desc: 'Create calendar events and meeting invites' },
                      { type: 'email_reply', label: 'Email Replies', desc: 'Draft and send email responses' },
                      { type: 'follow_up', label: 'Follow-ups', desc: 'Send follow-up messages and reminders' },
                      { type: 'outreach', label: 'Outreach', desc: 'Initiate contact and introductions' },
                    ] as activity}
                      {@const setting = autonomySettings.find(s => s.activity_type === activity.type)}
                      {@const currentLevel = setting?.level || 'suggest'}
                      {@const levels = ['observe', 'suggest', 'draft', 'act']}
                      {@const currentIdx = levels.indexOf(currentLevel)}
                      <div>
                        <div class="flex items-center justify-between mb-1.5">
                          <div>
                            <span class="text-ivory text-xs">{activity.label}</span>
                            <span class="text-ivory-muted/30 text-[9px] ml-2">{activity.desc}</span>
                          </div>
                          <span class="text-[9px] tracking-wider uppercase {currentLevel === 'act' ? 'text-negative' : currentLevel === 'draft' ? 'text-gold' : currentLevel === 'suggest' ? 'text-rose-300/60' : 'text-ivory-muted/40'}">
                            {currentLevel}
                          </span>
                        </div>
                        <div class="flex gap-1">
                          {#each levels as level, i}
                            <button
                              onclick={() => updateAutonomyLevel(activity.type, level)}
                              class="flex-1 h-1.5 rounded-full transition-colors duration-200 {i <= currentIdx
                                ? (currentLevel === 'act' ? 'bg-negative/60' : currentLevel === 'draft' ? 'bg-gold/50' : 'bg-rose-400/40')
                                : 'bg-border/30'
                              } hover:opacity-80"
                              title="{level.charAt(0).toUpperCase() + level.slice(1)}"
                            ></button>
                          {/each}
                        </div>
                        {#if currentLevel === 'act'}
                          <p class="text-negative/50 text-[9px] mt-1">Nyx will take autonomous action for {activity.label.toLowerCase()}</p>
                        {/if}
                      </div>
                    {/each}
                  </div>
                  <p class="text-ivory-muted/25 text-[9px] mt-3">
                    Observe &mdash; collect data silently &bull; Suggest &mdash; show recommendation cards &bull; Draft &mdash; create drafts for review &bull; Act &mdash; execute autonomously
                  </p>
                </div>
              {/if}
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 7. App Updates ═══════════════ -->
        <SettingsSection title="App Updates" icon={icons.update}>
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-ivory text-sm">Current version</p>
                <p class="text-ivory-muted/50 text-xs">{appVersion ? `v${appVersion}` : 'Unknown'}</p>
              </div>
              <button
                onclick={checkForUpdates}
                disabled={checkingUpdates}
                class="px-4 py-1.5 text-xs tracking-wider uppercase rounded border transition-all duration-200 {checkingUpdates ? 'border-border text-ivory-muted/50 cursor-wait' : 'border-gold/40 text-gold hover:bg-gold/10 hover:border-gold'}"
              >
                {#if checkingUpdates}
                  <div class="flex items-center gap-2">
                    <div class="w-3 h-3 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                    Checking...
                  </div>
                {:else}
                  Check for Updates
                {/if}
              </button>
            </div>

            {#if updateAvailable}
              <div class="flex items-center justify-between px-4 py-3 rounded-lg bg-gold/5 border border-gold/20">
                <div class="flex items-center gap-2">
                  <svg class="w-4 h-4 text-gold flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                    <path d={icons.update} />
                  </svg>
                  <span class="text-ivory text-sm">Nyx <span class="text-gold font-medium">v{updateVersion}</span> is available</span>
                </div>
                {#if updateInstalling}
                  <div class="flex items-center gap-2 text-xs text-ivory-muted">
                    <div class="w-3 h-3 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                    {updateProgress}
                  </div>
                {:else if updateProgress.includes('restart')}
                  <span class="text-positive text-xs">{updateProgress}</span>
                {:else}
                  <button
                    onclick={installUpdate}
                    class="px-4 py-1.5 bg-gold/10 border border-gold/40 text-gold text-xs tracking-wider uppercase rounded hover:bg-gold/20 hover:border-gold transition-all duration-200"
                  >
                    Update Now
                  </button>
                {/if}
              </div>
            {:else if !checkingUpdates && updateError}
              <p class="text-negative text-xs">{updateError}</p>
            {:else if !checkingUpdates && !updateAvailable && appVersion}
              <p class="text-positive text-xs flex items-center gap-1">
                <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path d="M4.5 12.75l6 6 9-13.5" />
                </svg>
                You're on the latest version.
              </p>
            {/if}
          </div>
        </SettingsSection>

        <!-- ═══════════════ 8. System Status ═══════════════ -->
        <SettingsSection title="System Status" icon={icons.server}>
          <div class="space-y-4">
            <!-- Docker container -->
            <div class="flex items-center justify-between">
              <div>
                <p class="text-ivory text-sm">Docker Container</p>
                <p class="text-xs {dockerStatus === 'running' ? 'text-positive' : dockerStatus === 'stopped' ? 'text-ivory-muted/50' : 'text-warning'}">
                  {#if dockerStatus === 'running'}Running
                  {:else if dockerStatus === 'stopped'}Stopped
                  {:else if dockerStatus === 'starting' || dockerStatus === 'restarting'}Starting...
                  {:else if dockerStatus === 'stopping'}Stopping...
                  {:else}Unknown
                  {/if}
                </p>
              </div>
              <div class="flex items-center gap-2">
                {#if dockerStatus === 'running'}
                  <button onclick={restartDocker} class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-border text-ivory-muted hover:text-ivory hover:border-ivory-muted/30 transition-colors">
                    Restart
                  </button>
                  <button onclick={stopDocker} class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-negative/30 text-negative/70 hover:text-negative hover:border-negative/50 transition-colors">
                    Stop
                  </button>
                {:else if dockerStatus === 'stopped'}
                  <button onclick={startDocker} class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-positive/30 text-positive/70 hover:text-positive hover:border-positive/50 transition-colors">
                    Start
                  </button>
                {:else if dockerStatus === 'starting' || dockerStatus === 'restarting' || dockerStatus === 'stopping'}
                  <div class="w-4 h-4 border-2 border-gold/40 border-t-gold rounded-full animate-spin"></div>
                {/if}
              </div>
            </div>

            <!-- Ollama -->
            <div class="flex items-center justify-between">
              <div>
                <p class="text-ivory text-sm">Ollama</p>
                <p class="text-xs {ollamaStatus === 'running' ? 'text-positive' : 'text-ivory-muted/50'}">
                  {ollamaStatus === 'running' ? 'Running' : ollamaStatus === 'installing' ? 'Installing...' : 'Not installed'}
                </p>
              </div>
              {#if ollamaStatus !== 'running' && ollamaStatus !== 'installing'}
                <button onclick={installOllama} class="px-3 py-1 text-[10px] tracking-wider uppercase rounded border border-accent/30 text-accent/70 hover:text-accent hover:border-accent/50 transition-colors">
                  Install
                </button>
              {/if}
            </div>

            <!-- System info -->
            {#if systemRam > 0}
              <div class="flex items-center justify-between text-xs">
                <span class="text-ivory-muted">System RAM</span>
                <span class="text-ivory">{systemRam} GB</span>
              </div>
            {/if}
          </div>
        </SettingsSection>

      </div>
    {/if}
  </div>

  <!-- Save Bar -->
  <SaveBar
    {hasChanges}
    {saving}
    restartRequired={restartRequired()}
    {saveError}
    onSave={saveSettings}
    onDiscard={discardChanges}
  />
</div>
