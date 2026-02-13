<script lang="ts">
  import { goto } from '$app/navigation';
  import PrerequisiteCheck from '$lib/components/PrerequisiteCheck.svelte';
  import SecurityPresetCard from '$lib/components/SecurityPresetCard.svelte';
  import ChannelCard from '$lib/components/ChannelCard.svelte';
  import LocalModelCard from '$lib/components/LocalModelCard.svelte';
  import CapabilitySummary from '$lib/components/CapabilitySummary.svelte';

  let step = $state(0);
  let provisionStatus = $state('');
  let provisionError = $state('');

  const steps = ['Welcome', 'Prerequisites', 'Essentials', 'Launch', 'Complete'];

  // ── Step 0: Disclaimer ──
  let disclaimerAccepted = $state(false);

  // ── Step 1: Prerequisites ──
  let dockerStatus = $state<'checking' | 'installed' | 'not_installed' | 'running' | 'not_running' | 'installing'>('checking');
  let dockerVersion = $state<string | null>(null);
  let dockerDownloadUrl = $state<string | null>(null);
  let gogStatus = $state<'checking' | 'installed' | 'not_installed' | 'running' | 'installing'>('checking');
  let gogAuthenticated = $state(false);
  let dockerInstallError = $state('');
  let dockerStartupHint = $state('');

  // Ollama (local models)
  let ollamaStatus = $state<'checking' | 'installed' | 'not_installed' | 'running' | 'installing'>('checking');
  let ollamaInstallError = $state('');
  let ollamaVersion = $state<string | null>(null);
  let systemRam = $state(0);
  let ollamaModels = $state<{name: string, size: number}[]>([]);
  let downloadingModel = $state<string | null>(null);
  let selectedOllamaModel = $state('');

  const recommendedModels = [
    { name: 'Qwen3 4B', tag: 'qwen3:4b', size: '2.7 GB', description: 'Fast reasoning, great for chat', minRam: 8 },
    { name: 'Llama 3.2 3B', tag: 'llama3.2:3b', size: '2.0 GB', description: 'Compact, good all-rounder', minRam: 8 },
    { name: 'Llama 3.1 8B', tag: 'llama3.1:8b', size: '4.7 GB', description: 'Strong general-purpose model', minRam: 16 },
    { name: 'Qwen 2.5 Coder 7B', tag: 'qwen2.5-coder:7b', size: '4.7 GB', description: 'Coding specialist, 92 languages', minRam: 16 },
    { name: 'Mistral 7B', tag: 'mistral:7b', size: '4.1 GB', description: 'Reliable workhorse', minRam: 16 },
    { name: 'DeepSeek Coder 6.7B', tag: 'deepseek-coder:6.7b', size: '3.8 GB', description: 'Coding-focused, compact', minRam: 16 },
  ];

  let hasLocalModel = $derived(ollamaModels.length > 0);

  async function installDocker() {
    dockerStatus = 'installing';
    dockerInstallError = '';
    dockerStartupHint = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const version: string = await invoke('install_docker');
      dockerVersion = version;
      dockerStatus = 'not_running';
      dockerStartupHint = 'Waiting for Docker daemon to start...';
      let attempts = 0;
      const pollInterval = setInterval(async () => {
        attempts++;
        if (attempts <= 5) {
          dockerStartupHint = 'Waiting for Docker daemon to start...';
        } else if (attempts <= 12) {
          dockerStartupHint = 'Docker is initialising — this can take up to a minute on first launch...';
        } else {
          dockerStartupHint = 'Still waiting — Docker first launch can be slow. You can also try "Re-check".';
        }
        try {
          const docker: any = await invoke('check_docker_detailed');
          if (docker.running) {
            dockerStatus = 'running';
            dockerVersion = docker.version || dockerVersion;
            dockerStartupHint = '';
            clearInterval(pollInterval);
          } else if (attempts >= 30) {
            dockerStartupHint = 'Docker is installed but the daemon hasn\'t started yet. Launch Docker Desktop from your Applications folder, then click Re-check.';
            clearInterval(pollInterval);
          }
        } catch {
          if (attempts >= 30) {
            dockerStartupHint = 'Docker is installed but the daemon hasn\'t started yet. Launch Docker Desktop from your Applications folder, then click Re-check.';
            clearInterval(pollInterval);
          }
        }
      }, 3000);
    } catch (e: any) {
      dockerInstallError = e?.toString() || 'Install failed';
      dockerStatus = 'not_installed';
    }
  }

  async function installOllama() {
    ollamaStatus = 'installing';
    ollamaInstallError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('install_ollama');
      // Ollama installed — poll until the HTTP server responds
      let attempts = 0;
      const pollInterval = setInterval(async () => {
        attempts++;
        try {
          const ollama: any = await invoke('check_ollama');
          if (ollama.available) {
            ollamaStatus = 'running';
            ollamaVersion = ollama.version || null;
            clearInterval(pollInterval);
          } else if (attempts >= 20) {
            ollamaStatus = 'not_installed';
            ollamaInstallError = 'Ollama installed but server not responding. Try launching Ollama from Applications.';
            clearInterval(pollInterval);
          }
        } catch {
          if (attempts >= 20) {
            ollamaStatus = 'not_installed';
            ollamaInstallError = 'Ollama installed but server not responding. Try launching Ollama from Applications.';
            clearInterval(pollInterval);
          }
        }
      }, 3000);
    } catch (e: any) {
      ollamaInstallError = e?.toString() || 'Install failed';
      ollamaStatus = 'not_installed';
    }
  }

  async function checkPrerequisites() {
    dockerStatus = 'checking';
    gogStatus = 'checking';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const docker: any = await invoke('check_docker_detailed');
      if (docker.running) {
        dockerStatus = 'running';
      } else if (docker.installed) {
        dockerStatus = 'not_running';
      } else {
        dockerStatus = 'not_installed';
      }
      dockerVersion = docker.version;
      dockerDownloadUrl = docker.download_url;

      const gog: any = await invoke('check_gog_available');
      if (gog.installed) {
        gogStatus = gog.authenticated ? 'running' : 'installed';
        gogAuthenticated = gog.authenticated;
      } else {
        gogStatus = 'not_installed';
      }
      // Auto-install gog silently if not installed
      if (gogStatus === 'not_installed') {
        try {
          await invoke('install_gog');
          gogStatus = 'installed';
        } catch {
          // Silent fail — user can install later
        }
      }
      // Check Ollama (optional)
      try {
        const ollama: any = await invoke('check_ollama');
        ollamaStatus = ollama.available ? 'running' : 'not_installed';
        ollamaVersion = ollama.version || null;
      } catch {
        ollamaStatus = 'not_installed';
      }
    } catch {
      // Dev mode fallback
      dockerStatus = 'running';
      dockerVersion = 'Docker Desktop 4.x (dev)';
      gogStatus = 'not_installed';
      ollamaStatus = 'not_installed';
    }
  }

  // Fetch system RAM unconditionally + Ollama models when entering Step 2
  $effect(() => {
    if (step === 2) {
      (async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          systemRam = await invoke('get_system_ram');
          if (ollamaStatus === 'running') {
            const models: any[] = await invoke('list_ollama_models');
            ollamaModels = models;
            // Auto-select first installed model if none selected
            if (!selectedOllamaModel && models.length > 0) {
              selectedOllamaModel = models[0].name;
            }
          }
        } catch {
          // Dev mode or error — ignore
        }
      })();
    }
  });

  async function pullOllamaModel(tag: string) {
    downloadingModel = tag;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('pull_ollama_model', { model: tag });
      // Refresh model list
      const models: any[] = await invoke('list_ollama_models');
      ollamaModels = models;
      if (!selectedOllamaModel) selectedOllamaModel = tag;
    } catch (e: any) {
      console.error('Failed to pull model:', e);
    } finally {
      downloadingModel = null;
    }
  }

  async function deleteOllamaModel(tag: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_ollama_model', { model: tag });
      const models: any[] = await invoke('list_ollama_models');
      ollamaModels = models;
      if (selectedOllamaModel === tag) {
        selectedOllamaModel = models.length > 0 ? models[0].name : '';
      }
    } catch (e: any) {
      console.error('Failed to delete model:', e);
    }
  }

  // Auto-advance when Docker is running on Step 1
  $effect(() => {
    if (step === 1 && dockerStatus === 'running') {
      setTimeout(() => {
        if (step === 1 && dockerStatus === 'running') {
          step = 2;
        }
      }, 600);
    }
  });

  // ── Background Docker image pre-pull ──
  let imagePullStarted = $state(false);
  let imagePullDone = $state(false);

  $effect(() => {
    if (dockerStatus === 'running' && !imagePullStarted && step >= 1) {
      imagePullStarted = true;
      (async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('docker_prepull');
          imagePullDone = true;
        } catch {
          // Silently fail — provision will retry
        }
      })();
    }
  });

  // ── Step 2: Essentials ──
  let agentName = $state('Nyx');
  let anthropicKey = $state('');
  let openaiKey = $state('');
  let veniceKey = $state('');
  let nearaiKey = $state('');
  let telegramToken = $state('');
  let slackToken = $state('');
  let whatsappPhone = $state('');
  let defaultLlmProvider = $state<'anthropic' | 'venice' | 'openai' | 'nearai' | 'ollama'>('anthropic');

  // Validation states
  let anthropicValid = $derived(anthropicKey.startsWith('sk-ant-') && anthropicKey.length > 20);
  let openaiValid = $derived(openaiKey.startsWith('sk-') && openaiKey.length > 20);
  let veniceValid = $derived(veniceKey.length > 10);
  let nearaiValid = $derived(nearaiKey.length > 10);
  let telegramValid = $derived(/^\d+:[A-Za-z0-9_-]+$/.test(telegramToken));
  let slackValid = $derived(/^xoxb-[A-Za-z0-9-]+$/.test(slackToken));
  let whatsappPhoneValid = $derived(/^\+\d{7,15}$/.test(whatsappPhone));

  // At least one LLM provider key must be valid, or user has selected Ollama (will download in Step 5)
  let hasAnyLlmKey = $derived(anthropicValid || openaiValid || veniceValid || nearaiValid || defaultLlmProvider === 'ollama');

  let anthropicValidating = $state(false);
  let anthropicValidated = $state(false);
  let anthropicValidationError = $state('');

  // Auto-detect API key patterns from clipboard paste
  function handleApiKeyPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text')?.trim();
    if (!text) return;
    if (text.startsWith('sk-ant-') && !anthropicKey) {
      anthropicKey = text;
      e.preventDefault();
    } else if (text.startsWith('sk-proj-') && !openaiKey) {
      openaiKey = text;
      e.preventDefault();
    } else if (/^\d+:[A-Za-z0-9_-]+$/.test(text) && !telegramToken) {
      telegramToken = text;
      e.preventDefault();
    } else if (text.startsWith('xoxb-') && !slackToken) {
      slackToken = text;
      e.preventDefault();
    }
  }

  // Validate Anthropic key via lightweight API call
  async function validateAnthropicKey() {
    if (!anthropicValid) return;
    anthropicValidating = true;
    anthropicValidationError = '';
    try {
      const res = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: {
          'x-api-key': anthropicKey,
          'anthropic-version': '2023-06-01',
          'content-type': 'application/json',
          'anthropic-dangerous-direct-browser-access': 'true',
        },
        body: JSON.stringify({
          model: 'claude-sonnet-4-20250514',
          max_tokens: 1,
          messages: [{ role: 'user', content: 'hi' }],
        }),
      });
      if (res.ok || res.status === 200) {
        anthropicValidated = true;
      } else if (res.status === 401) {
        anthropicValidationError = 'Invalid key';
      } else if (res.status === 429) {
        anthropicValidated = true;
      } else {
        anthropicValidated = true;
      }
    } catch {
      anthropicValidated = true;
    }
    anthropicValidating = false;
  }

  $effect(() => {
    if (anthropicValid && !anthropicValidated && !anthropicValidating) {
      validateAnthropicKey();
    }
    if (!anthropicValid) {
      anthropicValidated = false;
      anthropicValidationError = '';
    }
  });

  // Open URL in default browser via Tauri shell
  async function openExternal(url: string) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    } catch {
      window.open(url, '_blank');
    }
  }

  // Note: Provider selection is always free — user can select any provider
  // and then enter the corresponding key. No auto-reset.

  // ── Security ──
  let securityPreset = $state('balanced');
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

  async function applyPreset(preset: string) {
    securityPreset = preset;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const g: any = await invoke('get_guardrails_preset', { preset });
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
    } catch {
      const presets: Record<string, typeof guardrails> = {
        conservative: { maxTransactionUsd: 100, dailyLossPercent: 2, weeklyLossPercent: 5, dailyTxLimit: 10, requireConfirmation: true, maxSlippagePercent: 1, maxConcentrationPercent: 25, minHealthFactor: 2.0 },
        balanced: { maxTransactionUsd: 500, dailyLossPercent: 5, weeklyLossPercent: 15, dailyTxLimit: 20, requireConfirmation: false, maxSlippagePercent: 2, maxConcentrationPercent: 40, minHealthFactor: 1.5 },
        autonomous: { maxTransactionUsd: 1000000, dailyLossPercent: 100, weeklyLossPercent: 100, dailyTxLimit: 1000, requireConfirmation: false, maxSlippagePercent: 50, maxConcentrationPercent: 100, minHealthFactor: 1.0 },
      };
      if (presets[preset]) guardrails = { ...presets[preset] };
    }
  }

  // ── Messaging ──
  let messaging = $state({
    gmail: { enabled: false, autonomy: 'draft_only' },
    whatsapp: { enabled: false, autonomy: 'draft_only' },
    telegram: { enabled: false, autonomy: 'draft_only' },
    slack: { enabled: false, autonomy: 'draft_only' },
    signal: { enabled: false, autonomy: 'draft_only' },
  });

  let signalPhone = $state('');
  let signalPhoneValid = $derived(/^\+\d{7,15}$/.test(signalPhone));

  // ── Email defaults ──
  const detectedTimezone = (() => {
    try {
      const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
      return tz || 'Europe/London';
    } catch { return 'Europe/London'; }
  })();
  let emailTimezone = $state(detectedTimezone);
  let emailDigestHour = $state(8);
  let emailDigestMinute = $state(30);
  let emailTriageStartHour = $state(8);
  let emailTriageEndHour = $state(22);

  // ── Wallet state for auto-generation ──
  type WalletEntry = {
    id: string;
    chain: string;
    address: string;
    label: string;
    hasPrivateKey: boolean;
    isActive: boolean;
    secretKey?: string;
  };

  let wallets = $state<WalletEntry[]>([]);
  let showSecretKeyModal = $state(false);
  let secretKeyModalData = $state<{ address: string; secretKey: string } | null>(null);
  let secretKeyCopied = $state(false);
  let secretKeyBackedUp = $state(false);

  async function copySecretKey() {
    if (!secretKeyModalData?.secretKey) return;
    try {
      await navigator.clipboard.writeText(secretKeyModalData.secretKey);
      secretKeyCopied = true;
      setTimeout(() => { secretKeyCopied = false; }, 2000);
    } catch {
      const el = document.querySelector('#secret-key-display') as HTMLInputElement;
      if (el) { el.select(); document.execCommand('copy'); secretKeyCopied = true; setTimeout(() => { secretKeyCopied = false; }, 2000); }
    }
  }

  // ── Provision ──
  async function runProvision() {
    provisionStatus = 'Creating directories...';
    provisionError = '';

    // Dev mode: no Tauri runtime available — simulate provision
    const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
    if (!isTauri) {
      provisionStatus = 'Simulating setup (dev mode)...';
      await new Promise(r => setTimeout(r, 1500));
      provisionStatus = 'Setup complete! (dev mode)';
      setTimeout(() => { step = 4; }, 1000);
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');

      // Auto-generate NEAR wallet if none exist
      if (wallets.length === 0) {
        provisionStatus = 'Generating NEAR wallet...';
        try {
          const result: any = await invoke('generate_near_wallet_full');
          const [info, config] = result;
          const newWallet: WalletEntry = {
            id: config.id,
            chain: config.chain,
            address: config.address,
            label: config.label,
            hasPrivateKey: true,
            isActive: true,
            secretKey: info.secret_key,
          };
          wallets = [newWallet];
          // Show secret key backup modal
          if (info.secret_key) {
            secretKeyModalData = { address: config.address, secretKey: info.secret_key };
            secretKeyBackedUp = false;
            secretKeyCopied = false;
            showSecretKeyModal = true;
            // Wait for user to acknowledge before continuing
            await new Promise<void>((resolve) => {
              const interval = setInterval(() => {
                if (!showSecretKeyModal) {
                  clearInterval(interval);
                  resolve();
                }
              }, 200);
            });
          }
        } catch {
          // Dev fallback
          const mockId = Math.random().toString(36).substring(2, 15);
          wallets = [{
            id: mockId,
            chain: 'NEAR',
            address: 'a'.repeat(64),
            label: 'NEAR wallet',
            hasPrivateKey: true,
            isActive: true,
          }];
        }
      }

      const activeWallet = wallets.find(w => w.isActive);
      const walletConfigs = wallets.map(w => ({
        id: w.id,
        chain: w.chain,
        address: w.address,
        label: w.label,
        has_private_key: w.hasPrivateKey,
        is_active: w.isActive,
      }));

      const messagingConfig = {
        gmail: { enabled: messaging.gmail.enabled, autonomy: autonomyToRust(messaging.gmail.autonomy) },
        whatsapp: { enabled: messaging.whatsapp.enabled, autonomy: autonomyToRust(messaging.whatsapp.autonomy) },
        telegram: { enabled: messaging.telegram.enabled, autonomy: autonomyToRust(messaging.telegram.autonomy) },
        slack: { enabled: messaging.slack.enabled, autonomy: autonomyToRust(messaging.slack.autonomy) },
        signal: { enabled: messaging.signal.enabled, autonomy: autonomyToRust(messaging.signal.autonomy) },
      };

      provisionStatus = imagePullDone
        ? 'Writing configuration...'
        : 'Downloading container image & writing configuration — this may take a few minutes...';

      await invoke('run_setup_v2', {
        agentName: agentName.trim() || 'Nyx',
        anthropicKey: anthropicKey,
        openaiKey: openaiKey || null,
        veniceKey: veniceKey || null,
        nearaiKey: nearaiKey || null,
        telegramToken: telegramToken || null,
        slackToken: slackToken || null,
        whatsappPhone: whatsappPhone || null,
        wallets: walletConfigs,
        activeWalletId: activeWallet?.id || null,
        guardrailsPreset: securityPreset,
        guardrailsCustom: securityPreset === 'custom' ? {
          preset: 'Custom',
          max_transaction_usd: guardrails.maxTransactionUsd,
          daily_loss_percent: guardrails.dailyLossPercent,
          weekly_loss_percent: guardrails.weeklyLossPercent,
          daily_tx_limit: guardrails.dailyTxLimit,
          require_confirmation: guardrails.requireConfirmation,
          max_slippage_percent: guardrails.maxSlippagePercent,
          max_concentration_percent: guardrails.maxConcentrationPercent,
          min_health_factor: guardrails.minHealthFactor,
        } : null,
        messaging: messagingConfig,
        googleAuthenticated: gogAuthenticated,
        emailNotifications: {
          enabled: gogAuthenticated,
          timezone: emailTimezone,
          digest_hour: emailDigestHour,
          digest_minute: emailDigestMinute,
          triage_start_hour: emailTriageStartHour,
          triage_end_hour: emailTriageEndHour,
        },
        capabilities: {
          defi_crypto: true,
          travel: true,
          google_workspace: true,
          email_intelligence: true,
          communications: true,
          source_intelligence: true,
          default_llm_provider: defaultLlmProvider,
          ollama_model: defaultLlmProvider === 'ollama' ? (selectedOllamaModel || null) : null,
        },
      });

      provisionStatus = 'Setup complete!';
      setTimeout(() => {
        step = 4;
      }, 1500);
    } catch (e: any) {
      provisionError = e?.toString() || 'Setup failed';
    }
  }

  function autonomyToRust(level: string): string {
    if (level === 'send_with_confirm') return 'SendWithConfirm';
    if (level === 'autonomous') return 'Autonomous';
    return 'DraftOnly';
  }

  // ── Navigation ──
  function canAdvance(s: number): boolean {
    switch (s) {
      case 0: return disclaimerAccepted;
      case 1: return dockerStatus === 'running';
      case 2: {
        if (defaultLlmProvider === 'ollama') {
          return hasLocalModel; // Must download at least one model
        }
        return hasAnyLlmKey;
      }
      default: return true;
    }
  }

  function nextStep() {
    if (step === 0) {
      step = 1;
      checkPrerequisites();
    } else if (step === 1) {
      step = 2;
    } else if (step === 2) {
      step = 3;
      runProvision();
    }
  }

  function prevStep() {
    if (step === 2) step = 1;
    else if (step === 1) step = 0;
  }
</script>

<div class="h-full overflow-y-auto py-8 pb-16 flex justify-center">
  <div class="w-full max-w-xl px-6">
    <!-- Step indicator -->
    <div class="flex justify-center gap-2 mb-10">
      {#each steps as s, i}
        <button
          onclick={() => { if (i < step) step = i; }}
          class="w-1.5 h-1.5 rounded-full transition-colors duration-300 cursor-pointer"
          class:bg-gold={i <= step}
          class:bg-border={i > step}
          disabled={i >= step}
          aria-label="Step {i + 1}: {s}"
        ></button>
      {/each}
    </div>

    <!-- ═══════════════ STEP 0: Welcome + Disclaimer ═══════════════ -->
    {#if step === 0}
      <div class="text-center mb-8">
        <h1 class="font-display text-4xl font-light tracking-wider text-ivory mb-4">Nyx</h1>
        <p class="text-ivory-muted text-sm leading-relaxed mb-2">Your private AI chief of staff</p>
        <p class="text-ivory-muted/50 text-xs leading-relaxed max-w-sm mx-auto">
          Built on <span class="text-gold/60">OpenClaw</span>. Communications, calendar, documents, and crypto — all from one secure interface, running locally on your machine.
        </p>
      </div>

      <!-- Compact alpha notice -->
      <div class="bg-surface rounded-lg p-4 mb-6 border border-negative/20">
        <div class="flex items-start gap-2.5 mb-3">
          <svg class="w-4 h-4 text-negative flex-shrink-0 mt-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126z" />
            <path d="M12 15.75h.008v.008H12v-.008z" />
          </svg>
          <span class="text-negative text-xs font-medium">Alpha Release</span>
        </div>
        <div class="space-y-1.5 text-ivory-muted/70 text-xs leading-relaxed ml-6.5">
          <p>• Not audited — features are still being tested and may contain bugs.</p>
          <p>• DeFi operations involve <strong class="text-ivory-muted">real funds</strong> — losses are possible.</p>
          <p>• You are using this software <strong class="text-ivory-muted">at your own risk</strong>.</p>
        </div>
      </div>

      <!-- Acknowledgement + CTA -->
      <label class="flex items-start gap-3 cursor-pointer mb-8 group">
        <input
          type="checkbox"
          bind:checked={disclaimerAccepted}
          class="mt-0.5 accent-accent w-4 h-4"
        />
        <span class="text-ivory-muted text-xs leading-relaxed group-hover:text-ivory transition-colors">
          I understand this is alpha software and I am using it at my own risk.
        </span>
      </label>

      <div class="flex justify-center">
        <button
          onclick={nextStep}
          disabled={!disclaimerAccepted}
          class="px-10 py-3 border text-sm tracking-widest uppercase rounded transition-colors duration-300 {disclaimerAccepted ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted cursor-not-allowed'}"
        >
          Get Started
        </button>
      </div>

    <!-- ═══════════════ STEP 1: Prerequisites ═══════════════ -->
    {:else if step === 1}
      <div class="text-center mb-8">
        <h2 class="font-display text-2xl font-light tracking-wider text-ivory mb-2">Checking Prerequisites</h2>
        <p class="text-ivory-muted text-sm">Making sure your system is ready.</p>
      </div>

      <div class="space-y-4 mb-8">
        <PrerequisiteCheck
          label="Docker Desktop"
          status={dockerStatus}
          version={dockerVersion}
          downloadUrl={dockerDownloadUrl}
          onInstall={installDocker}
          installError={dockerInstallError}
          optional={false}
        />
        <PrerequisiteCheck
          label="Ollama"
          status={ollamaStatus}
          version={ollamaVersion}
          downloadUrl="https://ollama.com/download"
          onInstall={installOllama}
          optional={true}
          installError={ollamaInstallError}
        />
        {#if dockerStartupHint}
          <div class="flex items-start gap-2 px-4 -mt-2">
            {#if dockerStatus === 'not_running'}
              <div class="w-3 h-3 mt-0.5 flex-shrink-0">
                <div class="w-3 h-3 border-2 border-gold border-t-transparent rounded-full animate-spin"></div>
              </div>
            {/if}
            <p class="text-ivory-muted/60 text-xs leading-relaxed">{dockerStartupHint}</p>
          </div>
        {/if}
      </div>

      {#if dockerStatus === 'running'}
        <div class="text-center">
          <div class="flex items-center justify-center gap-2 text-positive text-sm">
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
              <path d="M4.5 12.75l6 6 9-13.5" />
            </svg>
            <span>Docker is running — continuing...</span>
          </div>
        </div>
      {:else}
        <div class="flex items-center justify-center gap-2 mb-6">
          <button
            onclick={checkPrerequisites}
            class="px-4 py-2 text-ivory-muted text-xs tracking-wider uppercase hover:text-ivory transition-colors duration-200"
          >
            Re-check
          </button>
        </div>

        <div class="flex justify-between">
          <button onclick={prevStep} class="px-6 py-2.5 text-ivory-muted text-sm hover:text-ivory transition-colors">Back</button>
          <button
            onclick={nextStep}
            disabled={!canAdvance(1)}
            class="px-8 py-2.5 border text-sm tracking-widest uppercase rounded transition-colors duration-300 {canAdvance(1) ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted cursor-not-allowed'}"
          >
            Continue
          </button>
        </div>
      {/if}

    <!-- ═══════════════ STEP 2: Essentials ═══════════════ -->
    {:else if step === 2}
      <div class="text-center mb-8">
        <h2 class="font-display text-2xl font-light tracking-wider text-ivory mb-2">Configure Your Agent</h2>
        <p class="text-ivory-muted text-sm">Add a provider key or use a local model to get started.</p>
        <p class="text-ivory-muted/40 text-xs mt-1">Paste a key anywhere — it will auto-fill the right field.</p>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div onpaste={handleApiKeyPaste} class="space-y-8">

        <!-- ─── Agent Name ─── -->
        <div>
          <p class="text-ivory-muted text-xs tracking-wider uppercase mb-3 flex items-center gap-2">
            <svg class="w-4 h-4 text-gold/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" />
            </svg>
            Agent Name
          </p>
          <p class="text-ivory-muted/40 text-xs mb-3">Your agent's identity. This is how it will introduce itself in conversations and messages.</p>
          <input
            type="text"
            bind:value={agentName}
            placeholder="Nyx"
            maxlength="32"
            class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border border-border focus:border-gold-dim focus:outline-none transition-colors duration-300 selectable"
          />
        </div>

        <!-- ─── Section 1: LLM Provider Keys ─── -->
        <div>
          <p class="text-ivory-muted text-xs tracking-wider uppercase mb-4 flex items-center gap-2">
            <span class="w-5 h-5 rounded-full bg-gold/20 text-gold text-[10px] flex items-center justify-center font-bold">1</span>
            LLM Provider Keys
          </p>

          <div class="space-y-4">
            <!-- Anthropic API Key -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <label class="text-ivory-muted text-xs">
                  Anthropic Claude <span class="text-gold text-[10px] uppercase tracking-wider ml-1">Recommended</span>
                </label>
                <button
                  onclick={() => openExternal('https://console.anthropic.com/settings/keys')}
                  class="flex items-center gap-1 text-gold-dim hover:text-gold text-xs transition-colors"
                >
                  <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                  </svg>
                  Get Key
                </button>
              </div>
              <div class="relative">
                <input
                  type="password"
                  bind:value={anthropicKey}
                  placeholder="sk-ant-..."
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {anthropicValidated ? 'border-positive' : anthropicValidationError ? 'border-negative' : 'border-border focus:border-gold-dim'}"
                />
                {#if anthropicValidating}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <div class="w-4 h-4 border-2 border-gold border-t-transparent rounded-full animate-spin"></div>
                  </div>
                {:else if anthropicValidated}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <svg class="w-4 h-4 text-positive" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                      <path d="M4.5 12.75l6 6 9-13.5" />
                    </svg>
                  </div>
                {:else if anthropicValidationError}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <svg class="w-4 h-4 text-negative" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                      <path d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </div>
                {/if}
              </div>
              {#if anthropicValidationError}
                <p class="text-negative text-xs mt-1">{anthropicValidationError}</p>
              {:else}
                <p class="text-ivory-muted/40 text-[10px] mt-1">Best reasoning, planning, and prompt injection resistance.</p>
              {/if}
            </div>

            <!-- OpenAI -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <label class="text-ivory-muted/70 text-xs">OpenAI <span class="text-ivory-muted/30">(voice, GPT-4o)</span></label>
                <button onclick={() => openExternal('https://platform.openai.com/api-keys')} class="text-gold-dim hover:text-gold text-xs transition-colors">Get Key</button>
              </div>
              <div class="relative">
                <input type="password" bind:value={openaiKey} placeholder="sk-proj-..."
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {openaiValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}" />
                {#if openaiValid}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <svg class="w-4 h-4 text-positive/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                  </div>
                {/if}
              </div>
            </div>

            <!-- Venice AI -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <label class="text-ivory-muted/70 text-xs">Venice AI <span class="text-ivory-muted/30">(privacy-first)</span></label>
                <button onclick={() => openExternal('https://venice.ai/settings/api')} class="text-gold-dim hover:text-gold text-xs transition-colors">Get Key</button>
              </div>
              <div class="relative">
                <input type="password" bind:value={veniceKey} placeholder="venice-..."
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {veniceValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}" />
                {#if veniceValid}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <svg class="w-4 h-4 text-positive/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                  </div>
                {/if}
              </div>
            </div>

            <!-- NEAR.ai -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <label class="text-ivory-muted/70 text-xs">NEAR.ai <span class="text-ivory-muted/30">(TEE confidential)</span></label>
                <button onclick={() => openExternal('https://cloud.near.ai')} class="text-gold-dim hover:text-gold text-xs transition-colors">Get Key</button>
              </div>
              <div class="relative">
                <input type="password" bind:value={nearaiKey} placeholder="NEAR.ai API key"
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {nearaiValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}" />
                {#if nearaiValid}
                  <div class="absolute right-3 top-1/2 -translate-y-1/2">
                    <svg class="w-4 h-4 text-positive/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                  </div>
                {/if}
              </div>
            </div>
          </div>

          {#if !hasAnyLlmKey}
            <p class="text-ivory-muted/40 text-xs mt-3 flex items-center gap-1.5">
              <svg class="w-3 h-3 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" /></svg>
              Enter a provider key or download a local model below to continue.
            </p>
          {/if}
        </div>

        <!-- ─── Section 2: Default LLM Provider ─── -->
        <div>
          <p class="text-ivory-muted text-xs tracking-wider uppercase mb-3 flex items-center gap-2">
            <span class="w-5 h-5 rounded-full bg-gold/20 text-gold text-[10px] flex items-center justify-center font-bold">2</span>
            Default LLM
          </p>
          <p class="text-ivory-muted/40 text-xs mb-3">Which provider should be used for reasoning and conversation? Anthropic is recommended for the strongest agentic capabilities, but you can choose any provider with a valid key.</p>
          <div class="space-y-2">
            <button
              onclick={() => { defaultLlmProvider = 'anthropic'; }}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'anthropic' ? 'border-gold bg-gold/5' : 'border-border hover:border-ivory-muted/30'}"
            >
              <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold {defaultLlmProvider === 'anthropic' ? 'bg-gold/20 text-gold' : 'bg-ivory-muted/10 text-ivory-muted'}">An</div>
              <div class="flex-1">
                <span class="text-ivory text-sm">Anthropic Claude</span>
                {#if anthropicValid}
                  <span class="text-gold text-[10px] uppercase tracking-wider ml-1">Recommended</span>
                {:else}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Enter key above</span>
                {/if}
              </div>
              <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'anthropic' ? 'border-gold' : 'border-border'}">
                {#if defaultLlmProvider === 'anthropic'}<div class="w-1.5 h-1.5 rounded-full bg-gold"></div>{/if}
              </div>
            </button>
            <button
              onclick={() => { defaultLlmProvider = 'openai'; }}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'openai' ? 'border-gold bg-gold/5' : 'border-border hover:border-ivory-muted/30'}"
            >
              <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold {defaultLlmProvider === 'openai' ? 'bg-gold/20 text-gold' : 'bg-ivory-muted/10 text-ivory-muted'}">Oa</div>
              <div class="flex-1">
                <span class="text-ivory text-sm">OpenAI GPT-4o</span>
                {#if !openaiValid}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Enter key above</span>
                {/if}
              </div>
              <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'openai' ? 'border-gold' : 'border-border'}">
                {#if defaultLlmProvider === 'openai'}<div class="w-1.5 h-1.5 rounded-full bg-gold"></div>{/if}
              </div>
            </button>
            <button
              onclick={() => { defaultLlmProvider = 'venice'; }}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'venice' ? 'border-gold bg-gold/5' : 'border-border hover:border-ivory-muted/30'}"
            >
              <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold {defaultLlmProvider === 'venice' ? 'bg-gold/20 text-gold' : 'bg-ivory-muted/10 text-ivory-muted'}">Ve</div>
              <div class="flex-1">
                <span class="text-ivory text-sm">Venice AI</span>
                {#if !veniceValid}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Enter key above</span>
                {/if}
              </div>
              <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'venice' ? 'border-gold' : 'border-border'}">
                {#if defaultLlmProvider === 'venice'}<div class="w-1.5 h-1.5 rounded-full bg-gold"></div>{/if}
              </div>
            </button>
            <button
              onclick={() => { defaultLlmProvider = 'nearai'; }}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'nearai' ? 'border-gold bg-gold/5' : 'border-border hover:border-ivory-muted/30'}"
            >
              <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold {defaultLlmProvider === 'nearai' ? 'bg-gold/20 text-gold' : 'bg-ivory-muted/10 text-ivory-muted'}">Na</div>
              <div class="flex-1">
                <span class="text-ivory text-sm">NEAR.ai</span>
                {#if nearaiValid}
                  <span class="text-positive text-[10px] uppercase tracking-wider ml-1">TEE Confidential</span>
                {:else}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Enter key above</span>
                {/if}
              </div>
              <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'nearai' ? 'border-gold' : 'border-border'}">
                {#if defaultLlmProvider === 'nearai'}<div class="w-1.5 h-1.5 rounded-full bg-gold"></div>{/if}
              </div>
            </button>
            <!-- Local (Ollama) option — always selectable -->
            <button
              onclick={() => { defaultLlmProvider = 'ollama'; }}
              class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-200 text-left {defaultLlmProvider === 'ollama' ? 'border-accent bg-accent/5' : 'border-border hover:border-ivory-muted/30'}"
            >
              <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold {defaultLlmProvider === 'ollama' ? 'bg-accent/20 text-accent' : 'bg-ivory-muted/10 text-ivory-muted'}">
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path d="M8.25 3v1.5M4.5 8.25H3m18 0h-1.5M4.5 12H3m18 0h-1.5m-15 3.75H3m18 0h-1.5M8.25 19.5V21M12 3v1.5m0 15V21m3.75-18v1.5m0 15V21m-9-1.5h9a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0015.75 4.5h-9A2.25 2.25 0 004.5 6.75v10.5A2.25 2.25 0 006.75 19.5z" />
                </svg>
              </div>
              <div class="flex-1">
                <span class="text-ivory text-sm">Local (Ollama)</span>
                {#if hasLocalModel}
                  <span class="text-accent text-[10px] uppercase tracking-wider ml-1">Private</span>
                {:else if ollamaStatus === 'running'}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Download a model below</span>
                {:else}
                  <span class="text-ivory-muted/40 text-[10px] ml-1">Will be installed below</span>
                {/if}
              </div>
              <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center flex-shrink-0 {defaultLlmProvider === 'ollama' ? 'border-accent' : 'border-border'}">
                {#if defaultLlmProvider === 'ollama'}<div class="w-1.5 h-1.5 rounded-full bg-accent"></div>{/if}
              </div>
            </button>
          </div>
          {#if defaultLlmProvider === 'ollama' && systemRam > 0}
            <div class="mt-2 px-3 py-2 rounded-lg bg-surface border border-accent/20">
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

          <!-- Inline Ollama install + model selection when Local is selected -->
          {#if defaultLlmProvider === 'ollama'}
            <div class="mt-3 pl-3 border-l-2 border-accent/20">
              {#if ollamaStatus !== 'running' && ollamaStatus !== 'installing'}
                <div class="p-3 rounded-lg border border-accent/20 bg-surface">
                  <p class="text-ivory-muted text-xs mb-2">Ollama is not installed. Install it to use local AI models on your Mac.</p>
                  <button
                    onclick={installOllama}
                    class="px-3 py-1.5 text-[10px] tracking-wider uppercase rounded border border-accent text-accent hover:bg-accent/10 transition-colors"
                  >
                    Install Ollama
                  </button>
                  {#if ollamaInstallError}
                    <p class="text-negative text-[10px] mt-1.5">{ollamaInstallError}</p>
                  {/if}
                </div>
              {:else if ollamaStatus === 'installing'}
                <div class="p-3 rounded-lg border border-accent/20 bg-surface">
                  <div class="flex items-center gap-2">
                    <div class="relative w-4 h-4">
                      <div class="absolute inset-0 border-2 border-border rounded-full"></div>
                      <div class="absolute inset-0 border-2 border-accent border-t-transparent rounded-full animate-spin"></div>
                    </div>
                    <p class="text-ivory-muted text-xs">Installing Ollama...</p>
                  </div>
                </div>
              {:else}
                <p class="text-ivory-muted/40 text-[10px] mb-2">Choose a model to download. {systemRam > 0 ? `${systemRam}GB RAM available.` : ''} Downloads are 2-5 GB each.</p>
                <div class="space-y-1.5">
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
              {/if}
            </div>
          {/if}
        </div>

        <!-- ─── Section 3: Messaging Channels ─── -->
        <div>
          <p class="text-ivory-muted text-xs tracking-wider uppercase mb-3 flex items-center gap-2">
            <span class="w-5 h-5 rounded-full bg-gold/20 text-gold text-[10px] flex items-center justify-center font-bold">3</span>
            Messaging Channels
          </p>
          <p class="text-ivory-muted/40 text-xs mb-3">Enable channels for the agent to send messages on your behalf. All default to Draft Only (safest).</p>
          <div class="space-y-3">
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
              <div class="ml-11 -mt-1 mb-1">
                <label class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">
                  Your Phone Number <span class="text-gold">*</span>
                </label>
                <input
                  type="tel"
                  bind:value={whatsappPhone}
                  placeholder="+44..."
                  class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {whatsappPhoneValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}"
                />
                <p class="text-ivory-muted/50 text-xs mt-1">
                  E.164 format with country code. Only this number can message the agent.
                </p>
              </div>
            {/if}
            <ChannelCard
              name="Telegram"
              enabled={messaging.telegram.enabled}
              autonomy={messaging.telegram.autonomy}
              onToggle={(v) => messaging.telegram.enabled = v}
              onAutonomyChange={(v) => messaging.telegram.autonomy = v}
            />
            <ChannelCard
              name="Slack"
              enabled={messaging.slack.enabled}
              autonomy={messaging.slack.autonomy}
              onToggle={(v) => messaging.slack.enabled = v}
              onAutonomyChange={(v) => messaging.slack.autonomy = v}
            />
            <ChannelCard
              name="Signal"
              enabled={messaging.signal.enabled}
              autonomy={messaging.signal.autonomy}
              onToggle={(v) => messaging.signal.enabled = v}
              onAutonomyChange={(v) => messaging.signal.autonomy = v}
            />
          </div>

          <!-- Signal phone number -->
          {#if messaging.signal.enabled}
            <div class="ml-11 mt-2 mb-1">
              <label class="text-ivory-muted text-xs tracking-wider uppercase mb-1.5 block">
                Signal Phone Number <span class="text-gold">*</span>
              </label>
              <input
                type="tel"
                bind:value={signalPhone}
                placeholder="+44..."
                class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {signalPhoneValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}"
              />
              <p class="text-ivory-muted/50 text-xs mt-1">
                E.164 format. Signal will be registered with this number via signal-cli.
              </p>
            </div>
          {/if}

          <!-- Bot Tokens (shown when Telegram or Slack enabled) -->
          {#if messaging.telegram.enabled || messaging.slack.enabled}
            <div class="mt-4 space-y-3">
              {#if messaging.telegram.enabled}
                <div>
                  <div class="flex items-center justify-between mb-1.5">
                    <label class="text-ivory-muted/70 text-xs">Telegram Bot Token</label>
                    <button onclick={() => openExternal('https://t.me/BotFather')} class="text-gold-dim hover:text-gold text-xs transition-colors">@BotFather</button>
                  </div>
                  <div class="relative">
                    <input type="password" bind:value={telegramToken} placeholder="123456:ABC-..."
                      class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {telegramValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}" />
                    {#if telegramValid}
                      <div class="absolute right-3 top-1/2 -translate-y-1/2">
                        <svg class="w-4 h-4 text-positive/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}
              {#if messaging.slack.enabled}
                <div>
                  <div class="flex items-center justify-between mb-1.5">
                    <label class="text-ivory-muted/70 text-xs">Slack Bot Token</label>
                    <button onclick={() => openExternal('https://api.slack.com/apps')} class="text-gold-dim hover:text-gold text-xs transition-colors">Create App</button>
                  </div>
                  <div class="relative">
                    <input type="password" bind:value={slackToken} placeholder="xoxb-..."
                      class="w-full bg-surface text-ivory text-sm px-4 py-2.5 rounded border focus:outline-none transition-colors duration-300 selectable {slackValid ? 'border-positive/50' : 'border-border focus:border-gold-dim'}" />
                    {#if slackValid}
                      <div class="absolute right-3 top-1/2 -translate-y-1/2">
                        <svg class="w-4 h-4 text-positive/70" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- ─── Section 4: DeFi Security Preset ─── -->
        <div>
          <p class="text-ivory-muted text-xs tracking-wider uppercase mb-3 flex items-center gap-2">
            <span class="w-5 h-5 rounded-full bg-gold/20 text-gold text-[10px] flex items-center justify-center font-bold">4</span>
            DeFi Security
          </p>
          <p class="text-ivory-muted/40 text-xs mb-3">Transaction limits and guardrails for crypto operations. Can be changed later.</p>
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

          <!-- Custom guardrails editor -->
          {#if securityPreset === 'custom'}
            <div class="mt-3 p-4 rounded-lg border border-gold/20 bg-surface-raised space-y-3">
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
                <input type="checkbox" id="require-confirm" bind:checked={guardrails.requireConfirmation}
                  class="accent-gold w-3.5 h-3.5" />
                <label for="require-confirm" class="text-ivory-muted text-xs">Require confirmation before transactions</label>
              </div>
            </div>
          {/if}
        </div>

      </div>

      <div class="flex justify-between items-end mt-8">
        <button onclick={prevStep} class="px-6 py-2.5 text-ivory-muted text-sm hover:text-ivory transition-colors">Back</button>
        <div class="text-right">
          <button
            onclick={nextStep}
            disabled={!canAdvance(2)}
            class="px-8 py-2.5 border text-sm tracking-widest uppercase rounded transition-colors duration-300 {canAdvance(2) ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted cursor-not-allowed'}"
          >
            Launch
          </button>
          {#if defaultLlmProvider === 'ollama' && !hasLocalModel}
            <p class="text-ivory-muted/40 text-[10px] mt-1.5">Download at least one local model to continue</p>
          {/if}
        </div>
      </div>

    <!-- ═══════════════ STEP 3: Launch (Provisioning) ═══════════════ -->
    {:else if step === 3}
      <div class="text-center py-16">
        {#if provisionError}
          <div class="w-16 h-16 rounded-full bg-negative/20 flex items-center justify-center mx-auto mb-6">
            <svg class="w-8 h-8 text-negative" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
              <path d="M6 18L18 6M6 6l12 12" />
            </svg>
          </div>
          <h2 class="font-display text-2xl font-light tracking-wider text-ivory mb-3">Setup Failed</h2>
          <p class="text-negative text-xs mb-6">{provisionError}</p>
          <button
            onclick={() => { provisionError = ''; step = 2; }}
            class="px-6 py-2.5 border border-border text-ivory-muted text-sm rounded hover:border-ivory-muted/50 transition-colors"
          >
            Go Back
          </button>
        {:else}
          <div class="w-16 h-16 mx-auto mb-6 relative">
            <div class="absolute inset-0 border-2 border-border rounded-full"></div>
            <div class="absolute inset-0 border-2 border-gold border-t-transparent rounded-full animate-spin"></div>
          </div>
          <h2 class="font-display text-2xl font-light tracking-wider text-ivory mb-3">Setting Up</h2>
          <p class="text-ivory-muted text-sm">{provisionStatus}</p>
        {/if}
      </div>

    <!-- ═══════════════ STEP 4: Complete ═══════════════ -->
    {:else if step === 4}
      <div class="text-center py-6">
        <div class="w-16 h-16 rounded-full bg-positive/20 flex items-center justify-center mx-auto mb-6">
          <svg class="w-8 h-8 text-positive" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M4.5 12.75l6 6 9-13.5" />
          </svg>
        </div>
        <h2 class="font-display text-3xl font-light tracking-wider text-ivory mb-3">Setup Complete</h2>
        {#if messaging.whatsapp.enabled || messaging.telegram.enabled || messaging.slack.enabled || messaging.signal.enabled}
          <p class="text-ivory-muted text-sm mb-8">{agentName} is running. Complete these steps to finish connecting your channels.</p>
        {:else}
          <p class="text-ivory-muted text-sm mb-8">{agentName} is running. Your AI chief of staff is ready.</p>
        {/if}
      </div>

      <!-- Capabilities overview -->
      <div class="mb-8">
        <p class="text-ivory-muted text-[10px] tracking-wider uppercase mb-3 text-center">What {agentName} can do</p>
        <CapabilitySummary />
      </div>

      <!-- Channel-specific next steps -->
      <div class="space-y-3 mb-8">
        {#if messaging.whatsapp.enabled}
          <div class="bg-surface rounded-lg p-4 border border-green-500/20">
            <div class="flex items-center gap-3 mb-2">
              <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-green-500/20 text-green-400">Wa</div>
              <h3 class="text-ivory text-sm">Connect WhatsApp</h3>
            </div>
            <ol class="text-ivory-muted text-xs space-y-1.5 ml-11 list-decimal">
              <li>Open the <strong class="text-ivory">Chat</strong> page from the sidebar</li>
              <li>Ask the agent: <span class="text-gold italic">"Connect WhatsApp"</span></li>
              <li>A QR code will appear — scan it with your phone (WhatsApp → Linked Devices)</li>
              <li>Once paired, the agent will respond to messages from your allowlisted number</li>
            </ol>
          </div>
        {/if}

        {#if messaging.telegram.enabled && telegramToken}
          <div class="bg-surface rounded-lg p-4 border border-blue-400/20">
            <div class="flex items-center gap-3 mb-2">
              <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-blue-400/20 text-blue-300">Tg</div>
              <h3 class="text-ivory text-sm">Pair Telegram</h3>
            </div>
            <ol class="text-ivory-muted text-xs space-y-1.5 ml-11 list-decimal">
              <li>Open Telegram and find your bot (the one you created with @BotFather)</li>
              <li>Send any message — e.g. <span class="text-gold italic">"Hello"</span></li>
              <li>The agent auto-pairs with your Telegram user ID on the first message</li>
            </ol>
          </div>
        {/if}

        {#if messaging.slack.enabled && slackToken}
          <div class="bg-surface rounded-lg p-4 border border-purple-500/20">
            <div class="flex items-center gap-3 mb-2">
              <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-purple-500/20 text-purple-400">Sl</div>
              <h3 class="text-ivory text-sm">Activate Slack</h3>
            </div>
            <ol class="text-ivory-muted text-xs space-y-1.5 ml-11 list-decimal">
              <li>Open your Slack workspace</li>
              <li>Invite the bot to channels where you want it active (<span class="font-mono text-ivory">/invite @YourBot</span>)</li>
              <li>Send a direct message to the bot to start a conversation</li>
            </ol>
          </div>
        {/if}

        {#if messaging.signal.enabled}
          <div class="bg-surface rounded-lg p-4 border border-blue-600/20">
            <div class="flex items-center gap-3 mb-2">
              <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-blue-600/20 text-blue-400">Si</div>
              <h3 class="text-ivory text-sm">Connect Signal</h3>
            </div>
            <ol class="text-ivory-muted text-xs space-y-1.5 ml-11 list-decimal">
              <li>Signal CLI is running in Docker alongside {agentName}</li>
              <li>Register your number: open <strong class="text-ivory">Chat</strong> and ask {agentName} to <span class="text-gold italic">"Register Signal with {signalPhone || 'my number'}"</span></li>
              <li>You will receive a verification code via SMS — share it with {agentName} to complete registration</li>
              <li>Once verified, send encrypted messages directly to your Signal number</li>
            </ol>
          </div>
        {/if}

        <!-- Google Workspace reminder -->
        <div class="bg-surface rounded-lg p-4 border border-border">
          <div class="flex items-center gap-3 mb-2">
            <div class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-ivory-muted/10 text-ivory-muted">Gw</div>
            <h3 class="text-ivory text-sm">Google Workspace</h3>
          </div>
          <p class="text-ivory-muted text-xs ml-11">
            Connect your Google account from the <strong class="text-ivory">Chat</strong> page to enable Gmail, Calendar, Drive, and Docs integration.
          </p>
        </div>
      </div>

      <button
        onclick={() => goto('/')}
        class="w-full px-8 py-3 border border-gold text-gold text-sm tracking-widest uppercase hover:bg-gold/10 transition-colors duration-300 rounded"
      >
        Open Dashboard
      </button>
    {/if}

    <!-- Secret Key Backup Modal -->
    {#if showSecretKeyModal && secretKeyModalData}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50 px-6"
        onkeydown={(e) => { if (e.key === 'Escape' && secretKeyBackedUp) { showSecretKeyModal = false; } }}
      >
        <div class="w-full max-w-md bg-surface border border-negative/40 rounded-xl p-6">
          <div class="flex items-center gap-3 mb-4">
            <div class="w-10 h-10 rounded-full bg-negative/20 flex items-center justify-center flex-shrink-0">
              <svg class="w-5 h-5 text-negative" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126z" />
                <path d="M12 15.75h.008v.008H12v-.008z" />
              </svg>
            </div>
            <div>
              <h3 class="text-ivory text-sm font-medium">Back Up Your Secret Key</h3>
              <p class="text-negative text-xs">This is the only time it will be shown.</p>
            </div>
          </div>

          <p class="text-ivory-muted text-xs leading-relaxed mb-4">
            Copy this secret key and store it somewhere safe. Without it, you cannot recover funds from this wallet. {agentName} stores an encrypted copy locally, but you should always keep your own backup.
          </p>

          <div class="relative mb-4">
            <input
              id="secret-key-display"
              type="text"
              readonly
              value={secretKeyModalData.secretKey}
              class="w-full bg-black text-ivory text-xs font-mono px-4 py-3 rounded border border-border selectable pr-20"
            />
            <button
              onclick={copySecretKey}
              class="absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-xs rounded transition-colors duration-200 {secretKeyCopied ? 'bg-positive/20 text-positive' : 'bg-surface-raised text-ivory-muted hover:text-ivory'}"
            >
              {secretKeyCopied ? 'Copied!' : 'Copy'}
            </button>
          </div>

          <p class="text-ivory-muted/50 text-xs font-mono mb-4">
            Wallet: {secretKeyModalData.address.slice(0, 8)}...{secretKeyModalData.address.slice(-4)}
          </p>

          <label class="flex items-start gap-3 cursor-pointer mb-5 group">
            <input
              type="checkbox"
              bind:checked={secretKeyBackedUp}
              class="mt-0.5 accent-accent w-4 h-4"
            />
            <span class="text-ivory-muted text-xs leading-relaxed group-hover:text-ivory transition-colors">
              I have saved my secret key somewhere safe.
            </span>
          </label>

          <button
            onclick={() => { showSecretKeyModal = false; }}
            disabled={!secretKeyBackedUp}
            class="w-full px-6 py-2.5 border text-sm tracking-widest uppercase rounded transition-colors duration-300 {secretKeyBackedUp ? 'border-gold text-gold hover:bg-gold/10' : 'border-border text-ivory-muted cursor-not-allowed'}"
          >
            Done
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
