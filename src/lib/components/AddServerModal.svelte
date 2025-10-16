<script lang="ts">
  import { serverStore, type ServerConfig, type TransportConfig, type ServerStoreState } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { createModalEscapeHandler, createModalOutsideClickHandler, globalModalManager } from '$lib/utils/modalHelpers';
  import { createLogger } from '$lib/utils/logger';
  import { platform } from '@tauri-apps/plugin-os';

  // Initialize scoped logger
  const logger = createLogger('AddServerModal');

  import {
    X,
    Database,
    Globe,
    Wifi,
    Network,
    HardDrive,
    Play,
    TestTube,
    FileText,
    Check,
    AlertCircle,
    Upload,
    Download,
    FolderOpen,
    Code
  } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { readText } from '@tauri-apps/plugin-clipboard-manager';


  // ✅ NEW: Use uiStore for modal state (single source of truth)
  const uiState = $derived($uiStore);
  const modalState = $derived(uiState.modals.addServer);
  const showModal = $derived(modalState.open);
  const modalLoading = $derived(modalState.loading);

  // ✅ FIXED: Track currentStep changes for debugging
  let currentStep = $state(1);

  // ✅ NEW: Inline error display (always visible to user)
  let modalError = $state<string | null>(null);

  // ✅ NEW: Log all currentStep changes for debugging
  $effect(() => {
    logger.debug(`📍 currentStep changed to: ${currentStep}`);
  });

  // ✅ FIXED: Force reset JSON import state when modal opens + Auto-detect clipboard
  // 🎯 UX FLOW: JSON import appears as overlay on top of regular modal
  //   - If clipboard has MCP config → show JSON import overlay (can cancel to reveal regular form)
  //   - If user cancels JSON import → return to regular 3-step form underneath
  //   - If user completes JSON import → close both modals
  $effect(() => {
    if (showModal) {
      logger.debug('🔓 Modal opened - resetting JSON import state');
      showJsonImport = false;  // Start with regular form (may be overridden by clipboard detection)
      clipboardDetected = false;
      jsonConfig = '';
      jsonError = '';

      // ✅ Auto-detect MCP config in clipboard using Tauri clipboard API
      autoDetectClipboard();
    } else {
      // ✅ FIXED: Clean up JSON import state when modal closes
      // (Form data is already reset synchronously in closeModal())
      logger.debug('🔒 Modal closed - cleaning up JSON import state');
      showJsonImport = false;
      clipboardDetected = false;
      jsonConfig = '';
      jsonError = '';
      modalHiddenForPicker = false;
    }
  });

  let testResult = $state<{ success: boolean; message?: string } | null>(null);
  let templates: ServerConfig[] = $state([]);
  let showJsonImport = $state(false);
  let jsonConfig = $state('');
  let jsonError = $state('');

  // ✅ NEW: Modal ref for escape handlers
  let modalRef: HTMLDivElement | null = $state(null);

  // Clipboard detection state
  let clipboardDetected = $state(false);
  let detectedServerName = $state('');
  let detectedServerCount = $state(0);

  // Form data
  // ✅ FIXED: Internal UI state uses lowercase 'websocket', maps to camelCase 'webSocket' when creating config
  type UITransportType = 'stdio' | 'http' | 'websocket' | 'tcp' | 'unix';

  let formData = $state({
    name: '',
    description: '',
    transportType: 'stdio' as UITransportType,
    stdio: {
      command: '',
      args: [] as string[],
      workingDirectory: '',
    },
    http: {
      url: '',
      headers: {} as Record<string, string>,
    },
    websocket: {
      url: '',
      headers: {} as Record<string, string>,
    },
    tcp: {
      host: '',
      port: 8080,
    },
    unix: {
      path: '',
    },
    environmentVariables: {} as Record<string, string>,
  });

  let argsInput = $state('');
  let headerPairs = $state([{ key: '', value: '' }]);
  let envPairs = $state([{ key: '', value: '' }]);

  // ✅ NEW: Real-time duplicate detection
  const serverStoreState = $derived($serverStore);
  const existingServers = $derived(
    serverStoreState.servers instanceof Map
      ? Array.from(serverStoreState.servers.values())
      : []
  );

  // ✅ FIXED: Step 2: Check for duplicate server name (IIFE for immediate execution)
  const duplicateNameServer = $derived(
    (() => {
      if (!formData.name.trim()) return null;
      return existingServers.find(
        s => s.config.name.toLowerCase() === formData.name.toLowerCase()
      );
    })()  // <-- IIFE: Execute immediately and return result
  );

  // ✅ FIXED: Step 3: Check for duplicate transport configuration (IIFE)
  const duplicateTransportServer = $derived(
    (() => {
      if (formData.transportType === 'stdio') {
        const command = formData.stdio.command.trim();
        if (!command) return null;
        return existingServers.find(s => {
          if (s.config.transport_config.type !== 'stdio') return false;
          return s.config.transport_config.command === command;
        });
      } else if (formData.transportType === 'http' || formData.transportType === 'websocket') {
        const url = formData.transportType === 'http' ? formData.http.url : formData.websocket.url;
        if (!url.trim()) return null;
        return existingServers.find(s => {
          // Type guard for url-based transports
          if (s.config.transport_config.type !== 'http' && s.config.transport_config.type !== 'webSocket') return false;  // ✅ camelCase
          return s.config.transport_config.url === url;
        });
      } else if (formData.transportType === 'tcp') {
        const { host, port } = formData.tcp;
        if (!host.trim()) return null;
        return existingServers.find(s => {
          if (s.config.transport_config.type !== 'tcp') return false;
          return s.config.transport_config.host === host && s.config.transport_config.port === port;
        });
      } else if (formData.transportType === 'unix') {
        const path = formData.unix.path.trim();
        if (!path) return null;
        return existingServers.find(s => {
          if (s.config.transport_config.type !== 'unix') return false;
          return s.config.transport_config.path === path;
        });
      }
      return null;
    })()
  );

  // Computed values for STDIO command handling
  const isAbsolutePath = $derived(
    (() => {
      const command = formData.stdio.command.trim();
      return command.startsWith('/') || /^[A-Za-z]:[/\\]/.test(command); // Unix absolute or Windows absolute
    })()
  );

  const derivedWorkingDirectory = $derived(
    (() => {
      if (!isAbsolutePath) return '';
      const command = formData.stdio.command.trim();
      const lastSlash = Math.max(command.lastIndexOf('/'), command.lastIndexOf('\\'));
      return lastSlash > 0 ? command.substring(0, lastSlash) : '';
    })()
  );

  const allTransportTypes = [
    {
      id: 'stdio',
      name: 'STDIO',
      description: 'Local process communication',
      icon: Database,
      example: 'node server.js'
    },
    {
      id: 'http',
      name: 'HTTP/SSE',
      description: 'HTTP with Server-Sent Events',
      icon: Globe,
      example: 'https://api.example.com/mcp'
    },
    {
      id: 'websocket',
      name: 'WebSocket',
      description: 'Real-time bidirectional communication',
      icon: Wifi,
      example: 'wss://ws.example.com/mcp'
    },
    {
      id: 'tcp',
      name: 'TCP',
      description: 'Raw TCP socket connection',
      icon: Network,
      example: 'localhost:8080'
    },
    {
      id: 'unix',
      name: 'Unix Socket',
      description: 'Local Unix domain socket',
      icon: HardDrive,
      example: '/tmp/mcp.sock'
    },
  ];

  // Filter out Unix socket support on Windows
  let currentPlatform = $state<string | null>(null);
  let transportTypes = $derived.by(() => {
    if (currentPlatform === 'windows') {
      return allTransportTypes.filter(t => t.id !== 'unix');
    }
    return allTransportTypes;
  });

  // Load platform on component mount
  $effect(() => {
    platform().then(p => {
      currentPlatform = p;
      logger.debug(`Platform detected: ${p}`);
    }).catch(err => {
      logger.error(`Failed to detect platform: ${err}`);
      currentPlatform = 'unknown';
    });
  });

  // ✅ NEW: Proper lifecycle management with cleanup
  let templateUnsubscribe: (() => void) | null = null;

  // Effect 1: Load templates on mount
  $effect(() => {
    loadTemplates();

    // Cleanup template subscription on unmount
    return () => {
      if (templateUnsubscribe) {
        templateUnsubscribe();
        templateUnsubscribe = null;
      }
    };
  });

  // Effect 2: Register modal with global manager when shown
  $effect(() => {
    if (showModal && modalRef) {
      globalModalManager.register('addServer', modalRef, {
        lockScroll: true,
        trapFocus: true
      });

      // Cleanup: Unregister when modal closes
      return () => {
        globalModalManager.unregister('addServer');
      };
    }
  });

  // ✅ NEW: Escape key handler
  $effect(() => {
    if (showModal && !modalLoading) {
      const escapeHandler = createModalEscapeHandler(closeModal);
      document.addEventListener('keydown', escapeHandler);
      return () => document.removeEventListener('keydown', escapeHandler);
    }
  });

  // ✅ NEW: Auto-detect MCP config in clipboard (Tauri API - no permission prompt!)
  async function autoDetectClipboard() {
    try {
      logger.info('📋 Starting clipboard auto-detection...');

      // Use Tauri clipboard API (no permission prompt required)
      const clipboardText = await readText();

      if (!clipboardText || !clipboardText.trim()) {
        logger.info('📋 Clipboard is empty, skipping auto-detection');
        return;
      }

      logger.info(`📋 Clipboard contains ${clipboardText.length} characters, checking if valid MCP config...`);
      const detected = detectMCPConfig(clipboardText);

      if (detected.valid) {
        clipboardDetected = true;
        detectedServerCount = detected.servers.length;
        jsonConfig = clipboardText; // Pre-populate JSON editor

        detectedServerName = detected.servers.length === 1
          ? detected.servers[0].name
          : `${detected.servers.length} servers`;

        logger.info(`📋 ✅ Auto-detected ${detectedServerName} in clipboard - showing notification`);

        // ✅ NEW UX: Show notification instead of auto-opening
        // User can click the banner to import, or continue with manual form
        // showJsonImport = true;  // REMOVED: Don't auto-open
        logger.info(`📋 Clipboard detection complete - waiting for user action`);
      } else {
        logger.info('📋 Clipboard content is not valid MCP configuration (no mcpServers key found)');
      }
    } catch (error) {
      // Better error logging to understand what's failing
      logger.error('📋 Failed to read clipboard:', error);
    }
  }

  // ✅ FIXED: Properly manage subscription lifecycle
  async function loadTemplates() {
    try {
      await serverStore.loadTemplates();

      // Clean up old subscription if exists
      if (templateUnsubscribe) {
        templateUnsubscribe();
      }

      // Create new subscription with cleanup
      templateUnsubscribe = serverStore.subscribe((state: ServerStoreState) => {
        templates = state.templates;
      });
    } catch (error) {
      logger.error('❌ Failed to load templates:', error);
    }
  }

  // ✅ FIXED: Comprehensive form data reset (prevents duplicate detection on just-added servers)
  function resetFormData() {
    logger.debug('🔄 Resetting all form data');

    // Reset basic info
    formData.name = '';
    formData.description = '';

    // Reset transport type (back to default)
    formData.transportType = 'stdio';

    // Reset all transport-specific configs
    formData.stdio.command = '';
    formData.stdio.args = [];
    formData.stdio.workingDirectory = '';

    formData.http.url = '';
    formData.http.headers = {};

    formData.websocket.url = '';
    formData.websocket.headers = {};

    formData.tcp.host = '';
    formData.tcp.port = 8080;

    formData.unix.path = '';

    formData.environmentVariables = {};

    // Reset UI helpers
    argsInput = '';
    headerPairs = [{ key: '', value: '' }];
    envPairs = [{ key: '', value: '' }];

    // Reset state
    currentStep = 1;
    testResult = null;
    modalError = null;
    connectAfterAdding = false;

    // ✅ NEW: Reset JSON import state too (complete cleanup)
    showJsonImport = false;
    jsonConfig = '';
    jsonError = '';
    clipboardDetected = false;
    detectedServerName = '';
    detectedServerCount = 0;
  }

  // ✅ FIXED: Close modal with comprehensive state reset
  function closeModal() {
    logger.info(`🚪 Closing modal - currentStep was: ${currentStep}`);

    // ✅ CRITICAL FIX: Reset ALL form data SYNCHRONOUSLY before closing modal
    // This prevents duplicate detection from firing on the just-added server
    resetFormData();

    uiStore.closeModal('addServer');
    // Additional cleanup happens in $effect when showModal becomes false
  }

  function nextStep() {
    if (currentStep < 3) {
      currentStep++;
    }
  }

  function prevStep() {
    if (currentStep > 1) {
      currentStep--;
    }
  }

  function selectTransport(type: UITransportType) {
    formData.transportType = type;
    nextStep();
  }

  function useTemplate(template: ServerConfig) {
    formData.name = template.name;
    formData.description = template.description || '';
    // ✅ FIXED: Map config type (webSocket) to UI type (websocket)
    formData.transportType = template.transport_config.type === 'webSocket' ? 'websocket' : template.transport_config.type;
    
    switch (template.transport_config.type) {
      case 'stdio':
        formData.stdio.command = template.transport_config.command || '';
        formData.stdio.args = template.transport_config.args || [];
        formData.stdio.workingDirectory = template.transport_config.working_directory || '';
        argsInput = (template.transport_config.args || []).join(' ');
        break;
      case 'http':
        formData.http.url = template.transport_config.url || '';
        formData.http.headers = template.transport_config.headers || {};
        headerPairs = Object.entries(formData.http.headers).map(([key, value]) => ({ key, value }));
        break;
      case 'webSocket':  // ✅ FIXED: camelCase to match type
        formData.websocket.url = template.transport_config.url || '';
        formData.websocket.headers = template.transport_config.headers || {};
        headerPairs = Object.entries(formData.websocket.headers).map(([key, value]) => ({ key, value }));
        break;
      case 'tcp':
        formData.tcp.host = template.transport_config.host || '';
        formData.tcp.port = template.transport_config.port || 8080;
        break;
      case 'unix':
        formData.unix.path = template.transport_config.path || '';
        break;
    }
    
    formData.environmentVariables = template.environment_variables;
    envPairs = Object.entries(formData.environmentVariables).map(([key, value]) => ({ key, value }));
    
    currentStep = 3;
  }

  function updateArgs() {
    formData.stdio.args = argsInput.trim() ? argsInput.trim().split(/\s+/) : [];
  }

  function updateHeaders() {
    const headers: Record<string, string> = {};
    headerPairs.forEach(pair => {
      if (pair.key && pair.value) {
        headers[pair.key] = pair.value;
      }
    });
    
    if (formData.transportType === 'http') {
      formData.http.headers = headers;
    } else if (formData.transportType === 'websocket') {
      formData.websocket.headers = headers;
    }
  }

  function addHeaderPair() {
    headerPairs = [...headerPairs, { key: '', value: '' }];
  }

  function removeHeaderPair(index: number) {
    headerPairs = headerPairs.filter((_, i) => i !== index);
    updateHeaders();
  }

  function updateEnvironmentVariables() {
    const env: Record<string, string> = {};
    envPairs.forEach(pair => {
      if (pair.key && pair.value) {
        env[pair.key] = pair.value;
      }
    });
    formData.environmentVariables = env;
  }

  function addEnvPair() {
    envPairs = [...envPairs, { key: '', value: '' }];
  }

  function removeEnvPair(index: number) {
    envPairs = envPairs.filter((_, i) => i !== index);
    updateEnvironmentVariables();
  }

  // ✅ FIXED: File picker state preservation
  // Hide modal visually without destroying component (preserves state)
  let modalHiddenForPicker = $state(false);

  // Directory picker for STDIO transport
  async function selectDirectory() {
    try {
      logger.debug('📁 Opening directory picker');

      // ✅ FIXED: Hide modal visually but keep component alive (preserves ALL state)
      modalHiddenForPicker = true;

      // Small delay to ensure modal is hidden before file picker shows
      await new Promise(resolve => setTimeout(resolve, 50));

      const result = await open({
        directory: true,
        multiple: false,
        title: 'Select Working Directory'
      });

      logger.debug('📁 Directory picker result:', result);

      if (result && typeof result === 'string') {
        formData.stdio.workingDirectory = result;
        logger.info(`✅ Selected working directory: ${result}`);
      }
    } catch (error) {
      logger.error('❌ Failed to select directory:', error);
    } finally {
      // ✅ FIXED: Show modal again (currentStep, formData, everything preserved!)
      modalHiddenForPicker = false;
    }
  }

  // File picker for executable selection
  async function selectExecutable() {
    try {
      logger.debug('📄 Opening executable picker');

      // ✅ FIXED: Hide modal visually but keep component alive (preserves ALL state)
      modalHiddenForPicker = true;

      // Small delay to ensure modal is hidden before file picker shows
      await new Promise(resolve => setTimeout(resolve, 50));

      const result = await open({
        directory: false,
        multiple: false,
        title: 'Select Server Executable'
        // No filters - allow all files including extensionless executables
      });

      logger.debug('📄 Executable picker result:', result);

      if (result && typeof result === 'string') {
        formData.stdio.command = result;
        logger.info(`✅ Selected executable: ${result}`);
      }
    } catch (error) {
      logger.error('❌ Failed to select executable:', error);
    } finally {
      // ✅ FIXED: Show modal again (currentStep, formData, everything preserved!)
      modalHiddenForPicker = false;
    }
  }

  // JSON config import/export functions
  function toggleJsonImport() {
    showJsonImport = !showJsonImport;
    jsonError = '';
    if (showJsonImport && !jsonConfig) {
      // Pre-populate with current config when opening
      exportToJson();
    }
  }

  // ✅ NEW: Cancel JSON import (keep main modal open)
  function cancelJsonImport() {
    logger.debug('❌ User canceled JSON import - returning to main modal');
    showJsonImport = false;
    clipboardDetected = false;
    jsonConfig = '';
    jsonError = '';
  }

  // ✅ NEW: User-initiated JSON import from clipboard detection banner
  function openClipboardImport() {
    logger.info('📋 User clicked to import from clipboard');
    showJsonImport = true;
  }

  // ✅ NEW: Dismiss clipboard notification without importing
  function dismissClipboardNotification() {
    logger.info('📋 User dismissed clipboard notification');
    clipboardDetected = false;
    detectedServerName = '';
    detectedServerCount = 0;
    jsonConfig = '';
  }

  function exportToJson() {
    try {
      updateArgs();
      updateHeaders();
      updateEnvironmentVariables();

      // Create standard MCP configuration format
      const serverName = formData.name || 'mcp-server';
      const mcpConfig = {
        mcpServers: {
          [serverName]: formData.transportType === 'stdio' ? {
            command: formData.stdio.command,
            args: formData.stdio.args,
            ...(formData.stdio.workingDirectory && { cwd: formData.stdio.workingDirectory }),
            ...(Object.keys(formData.environmentVariables).length > 0 && { env: formData.environmentVariables })
          } : formData.transportType === 'http' ? {
            url: formData.http.url,
            ...(Object.keys(formData.http.headers).length > 0 && { headers: formData.http.headers })
          } : formData.transportType === 'websocket' ? {
            url: formData.websocket.url,
            ...(Object.keys(formData.websocket.headers).length > 0 && { headers: formData.websocket.headers })
          } : formData.transportType === 'tcp' ? {
            host: formData.tcp.host,
            port: formData.tcp.port
          } : formData.transportType === 'unix' ? {
            path: formData.unix.path
          } : {}
        }
      };

      jsonConfig = JSON.stringify(mcpConfig, null, 2);
      jsonError = '';
    } catch (error) {
      jsonError = `Failed to export config: ${error}`;
    }
  }

  // ✅ FIXED: Manual clipboard import (user-triggered)
  async function importFromClipboard() {
    try {
      // Use Tauri clipboard API (consistent with auto-detection)
      const clipboardText = await readText();
      if (!clipboardText || !clipboardText.trim()) {
        uiStore.showError('Clipboard is empty');
        return;
      }

      const detected = detectMCPConfig(clipboardText);

      if (detected.valid) {
        clipboardDetected = true;
        detectedServerCount = detected.servers.length;
        jsonConfig = clipboardText; // Pre-populate JSON editor

        detectedServerName = detected.servers.length === 1
          ? detected.servers[0].name
          : `${detected.servers.length} servers`;

        logger.info(`📋 Importing ${detectedServerName} from clipboard`);

        // Open JSON import view with clipboard content
        showJsonImport = true;
      } else {
        uiStore.showError('Clipboard does not contain valid MCP configuration JSON');
      }
    } catch (error) {
      logger.error('Failed to read clipboard:', error);
      uiStore.showError('Failed to read clipboard');
    }
  }

  function detectMCPConfig(text: string): { valid: boolean; servers: Array<{name: string, config: Record<string, unknown>}> } {
    try {
      const parsed = JSON.parse(text);
      if (parsed.mcpServers && typeof parsed.mcpServers === 'object') {
        const servers = Object.entries(parsed.mcpServers).map(([name, config]) => ({
          name,
          config: config as Record<string, unknown>
        }));
        return { valid: true, servers };
      }
    } catch {
      // ✅ Empty catch is intentional - invalid JSON is a normal failure case
    }
    return { valid: false, servers: [] };
  }


  async function importFromJson() {
    try {
      jsonError = '';
      const config = JSON.parse(jsonConfig);

      // Validate standard MCP configuration format
      if (!config.mcpServers) {
        throw new Error('Invalid config: missing "mcpServers" section');
      }

      const serverNames = Object.keys(config.mcpServers);
      if (serverNames.length === 0) {
        throw new Error('Invalid config: no servers defined in "mcpServers"');
      }

      // ✅ FIXED: Import servers without auto-connecting
      // User can connect manually from the sidebar when ready
      await importServers(config.mcpServers);
    } catch (error) {
      jsonError = `Failed to import config: ${error}`;
    }
  }

  // ✅ FIXED: Add servers without auto-connecting (respects user's workflow choice)
  async function importServers(serversConfig: Record<string, any>) {
    // ✅ Set loading state for the import button
    uiStore.setModalLoading('addServer', true);

    try {
      const serverNames = Object.keys(serversConfig);
      logger.info(`📥 Importing ${serverNames.length} server(s) (add only, no auto-connect)...`);

      const importPromises = serverNames.map(async (name) => {
        const serverConfig = serversConfig[name];

        // ✅ FIXED: Build transport config only (no DB creation yet)
        const transport = buildTransportConfigFromMCP(serverConfig);

        // ✅ Create server config once (not twice!)
        return serverStore.createServerConfig(
          name,
          serverConfig.description || undefined,
          transport,
          serverConfig.env || {}
        );
      });

      const results = await Promise.allSettled(importPromises);
      const successful = results.filter(r => r.status === 'fulfilled').length;
      const failed = results.filter(r => r.status === 'rejected').length;

      if (successful > 0) {
        const message = serverNames.length === 1
          ? `Added ${serverNames[0]} (not connected)`
          : `Added ${successful}/${serverNames.length} server(s) (not connected)`;
        uiStore.showSuccess(message);
      }
      if (failed > 0) {
        const message = serverNames.length === 1
          ? `Failed to add ${serverNames[0]}`
          : `Failed to add ${failed} server(s)`;
        uiStore.showError(message);
      }

      // Close modals on success (even partial success)
      if (successful > 0) {
        // ✅ FIXED: Close JSON import overlay first, then close main modal after brief delay
        showJsonImport = false;

        // Small delay to ensure JSON modal closes before main modal
        // This prevents UI glitches and ensures clean state reset
        setTimeout(() => {
          closeModal();
        }, 100);
      }
    } catch (error) {
      jsonError = `Failed to import servers: ${error}`;
      logger.error('❌ Import failed:', error);
    } finally {
      uiStore.setModalLoading('addServer', false);
    }
  }

  // ✅ FIXED: Only build the transport config object, don't create in DB yet
  function buildTransportConfigFromMCP(mcpConfig: any): TransportConfig {
    if (mcpConfig.command) {
      return {
        type: 'stdio',
        command: mcpConfig.command,
        args: mcpConfig.args || [],
        working_directory: mcpConfig.cwd || undefined,
      };
    } else if (mcpConfig.url) {
      const url = mcpConfig.url.toLowerCase();
      if (url.startsWith('ws://') || url.startsWith('wss://')) {
        return {
          type: 'webSocket',  // ✅ camelCase to match Rust serialization
          url: mcpConfig.url,
          headers: mcpConfig.headers || {},
        };
      } else {
        return {
          type: 'http',
          url: mcpConfig.url,
          headers: mcpConfig.headers || {},
        };
      }
    } else if (mcpConfig.host && mcpConfig.port) {
      return {
        type: 'tcp',
        host: mcpConfig.host,
        port: mcpConfig.port,
      };
    } else if (mcpConfig.path) {
      return {
        type: 'unix',
        path: mcpConfig.path,
      };
    } else {
      throw new Error(`Unable to determine transport type for config`);
    }
  }

  // ✅ FIXED: Test configuration with proper loading state
  async function testConfiguration() {
    if (modalLoading) return;

    uiStore.setModalLoading('addServer', true);
    testResult = null;

    try {
      const config = buildConfig();
      const result = await serverStore.testServerConfig(config);
      testResult = { success: result, message: result ? 'Configuration is valid' : 'Configuration test failed' };
    } catch (error) {
      testResult = { success: false, message: `Test failed: ${error}` };
    } finally {
      uiStore.setModalLoading('addServer', false);
    }
  }

  // ✅ FIXED: Build config WITHOUT saving to database (prevents duplicate creation)
  function buildConfig(): ServerConfig {
    updateArgs();
    updateHeaders();
    updateEnvironmentVariables();

    let transport: TransportConfig;

    switch (formData.transportType) {
      case 'stdio':
        // Use explicit working directory if provided, otherwise derive from absolute path
        let workingDir = formData.stdio.workingDirectory;
        if (!workingDir && isAbsolutePath) {
          workingDir = derivedWorkingDirectory;
        }

        transport = {
          type: 'stdio',
          command: formData.stdio.command,
          args: formData.stdio.args,
          working_directory: workingDir || undefined,
        };
        break;
      case 'http':
        transport = {
          type: 'http',
          url: formData.http.url,
          headers: formData.http.headers,
        };
        break;
      case 'websocket':
        transport = {
          type: 'webSocket',  // ✅ FIXED: camelCase to match Rust serialization
          url: formData.websocket.url,
          headers: formData.websocket.headers,
        };
        break;
      case 'tcp':
        transport = {
          type: 'tcp',
          host: formData.tcp.host,
          port: formData.tcp.port,
        };
        break;
      case 'unix':
        transport = {
          type: 'unix',
          path: formData.unix.path,
        };
        break;
    }

    // ✅ NEW: Just build the config object, don't save yet
    // connectServer() will handle both creation and connection
    return {
      id: crypto.randomUUID(), // Temporary client-side ID
      name: formData.name,
      description: formData.description || undefined,
      transport_config: transport,
      environment_variables: formData.environmentVariables,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }

  // ✅ NEW: Separate add and connect functionality
  let connectAfterAdding = $state(false);

  // ✅ FIXED: Add server configuration (without auto-connecting)
  async function addServer() {
    // Clear any previous errors
    modalError = null;

    // ✅ FIXED: Prevent concurrent calls
    if (modalLoading) {
      logger.warn('⚠️ Add operation already in progress, ignoring duplicate click');
      return;
    }

    const requestId = crypto.randomUUID();
    uiStore.setModalLoading('addServer', true, requestId);

    try {
      // ✅ Build config - validation already done by reactive $derived checks
      const config = buildConfig();

      logger.info(`➕ Adding server: ${config.name} (connect: ${connectAfterAdding})`);
      logger.debug('Server config:', config);

      if (connectAfterAdding) {
        // ✅ User wants to connect immediately - use connectServer() (creates + connects)
        await serverStore.connectServer(config);
        logger.info(`✅ Successfully added and connected to: ${config.name}`);
        uiStore.showSuccess(`Added and connected to ${config.name}`);
      } else {
        // ✅ Just add the configuration without connecting
        await serverStore.createServerConfig(
          config.name,
          config.description,
          config.transport_config,
          config.environment_variables
        );
        logger.info(`✅ Successfully added server: ${config.name}`);
        uiStore.showSuccess(`Added ${config.name} (not connected)`);
      }

      closeModal();
    } catch (error) {
      logger.error('❌ Failed to add server:', error);
      logger.error('Error details:', {
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });

      // ✅ FIXED: Parse error and show user-friendly message INLINE
      const errorStr = error instanceof Error ? error.message : String(error);

      // Handle common error scenarios with clear, actionable messages
      if (errorStr.toLowerCase().includes('already exists') ||
          errorStr.toLowerCase().includes('duplicate')) {
        modalError = `⚠️ Server "${formData.name}" already exists. Please use a different name.`;
      } else if (errorStr.toLowerCase().includes('timeout')) {
        modalError = `⏱️ Operation timed out. Please try again.`;
      } else if (errorStr.toLowerCase().includes('econnrefused') ||
                 errorStr.toLowerCase().includes('connection refused')) {
        modalError = `🔌 Could not connect to server. Is it running?`;
      } else if (errorStr.toLowerCase().includes('command not found') ||
                 errorStr.toLowerCase().includes('no such file')) {
        modalError = `📁 Could not find the server executable. Please check the command path.`;
      } else if (errorStr.toLowerCase().includes('permission')) {
        modalError = `🔐 Permission denied. The command may not be executable.`;
      } else {
        modalError = `❌ Failed to add server: ${errorStr}`;
      }

      // ✅ FIXED: Don't close modal on error - user can see error and fix it
    } finally {
      uiStore.setModalLoading('addServer', false);
    }
  }
</script>

{#if showModal}
  <div
    bind:this={modalRef}
    role="dialog"
    aria-modal="true"
    aria-labelledby="add-server-modal-title"
    tabindex="-1"
    onclick={createModalOutsideClickHandler(modalRef, () => {
      if (!modalLoading) closeModal();
    })}
    onkeydown={(e) => {
      if (e.key === 'Escape' && !modalLoading) closeModal();
    }}
    class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4"
    class:hidden={modalHiddenForPicker || showJsonImport}
  >
    <!--
      ✅ modalHiddenForPicker: Hides modal visually during file picker dialogs
      - Preserves component state (currentStep, formData) while hidden
      - Prevents modal from overlapping native file picker on macOS
      - See selectDirectory() and selectExecutable() for usage
    -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      onclick={(e) => e.stopPropagation()}
      class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden"
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <div>
          <h2 id="add-server-modal-title" class="text-xl font-semibold text-gray-900">Add MCP Server</h2>
          <p class="text-sm text-gray-600 mt-1">Step {currentStep} of 3</p>
        </div>
        <button onclick={closeModal} class="text-gray-400 hover:text-gray-600" aria-label="Close modal">
          <X size={24} />
        </button>
      </div>

      <!-- Progress Bar -->
      <div class="px-6 py-2 bg-gray-50">
        <div class="flex items-center space-x-2">
          {#each [1, 2, 3] as step}
            <div class="flex-1 h-2 rounded-full {step <= currentStep ? 'bg-mcp-primary-500' : 'bg-gray-200'}"></div>
          {/each}
        </div>
      </div>

      <!-- ✅ NEW: Clipboard Detection Banner (Opt-in for JSON import) -->
      {#if clipboardDetected && !showJsonImport}
        <div class="mx-6 mt-4 p-4 bg-blue-50 dark:bg-blue-900/30 border-l-4 border-blue-500 dark:border-blue-400 rounded-r-md">
          <div class="flex items-start">
            <div class="p-2 rounded-lg bg-blue-500 dark:bg-blue-600 text-white mr-3 flex-shrink-0">
              <Check size={16} />
            </div>
            <div class="flex-1 min-w-0">
              <h4 class="font-semibold text-blue-900 dark:text-blue-100 text-sm">MCP Configuration Detected in Clipboard</h4>
              <p class="text-sm text-blue-800 dark:text-blue-200 mt-1">
                Found <strong>{detectedServerName}</strong> ready to import.
                {#if detectedServerCount > 1}
                  <span class="text-xs block mt-1 opacity-90">All {detectedServerCount} servers will be added (connect manually when ready).</span>
                {/if}
              </p>
              <div class="flex gap-2 mt-3">
                <button
                  onclick={openClipboardImport}
                  class="px-4 py-2 bg-blue-600 dark:bg-blue-500 text-white text-sm font-medium rounded-lg hover:bg-blue-700 dark:hover:bg-blue-600 transition-colors shadow-sm"
                >
                  <Upload size={14} class="inline mr-1.5" />
                  Import from Clipboard
                </button>
                <button
                  onclick={dismissClipboardNotification}
                  class="px-4 py-2 bg-white dark:bg-gray-700 text-blue-700 dark:text-blue-300 text-sm font-medium rounded-lg border border-blue-300 dark:border-blue-600 hover:bg-blue-50 dark:hover:bg-gray-600 transition-colors"
                >
                  Continue Manually
                </button>
              </div>
            </div>
            <button
              onclick={dismissClipboardNotification}
              class="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 ml-2 flex-shrink-0"
              aria-label="Dismiss notification"
            >
              <X size={18} />
            </button>
          </div>
        </div>
      {/if}

      <!-- ✅ NEW: Inline Error Banner (Always Visible) -->
      {#if modalError}
        <div class="mx-6 mt-4 p-4 bg-red-50 border-l-4 border-red-500 rounded-r-md">
          <div class="flex items-start">
            <AlertCircle size={20} class="text-red-600 mr-3 flex-shrink-0 mt-0.5" />
            <div class="flex-1">
              <p class="text-sm font-medium text-red-800">{modalError}</p>
            </div>
            <button
              onclick={() => { modalError = null; }}
              class="text-red-600 hover:text-red-800 ml-2 flex-shrink-0"
              aria-label="Dismiss error"
            >
              <X size={16} />
            </button>
          </div>
        </div>
      {/if}

      <!-- Content -->
      <div class="p-6 overflow-y-auto max-h-[60vh]">
        {#if currentStep === 1}
          <!-- Step 1: Choose Transport Type -->
          <div class="space-y-4">
            <div class="text-center mb-6">
              <h3 class="text-lg font-medium mb-2 text-primary">Choose Transport Type</h3>
              <p class="text-secondary">Select how you want to connect to your MCP server</p>
            </div>

            <div class="grid grid-cols-1 gap-3">
              {#each transportTypes as transport}
                <button
                  onclick={() => selectTransport(transport.id as UITransportType)}
                  class="transport-option-button"
                >
                  <div class="flex items-start">
                    <div class="transport-option-icon">
                      {#if transport.icon}
                        {@const IconComponent = transport.icon}
                        <IconComponent size={20} class="transport-option-icon-svg" />
                      {/if}
                    </div>
                    <div class="flex-1">
                      <h4 class="transport-option-title">{transport.name}</h4>
                      <p class="transport-option-description">{transport.description}</p>
                      <p class="transport-option-example">{transport.example}</p>
                    </div>
                  </div>
                </button>
              {/each}
            </div>

            <!-- JSON Import Options -->
            <div class="mt-8 pt-6 border-t border-gray-200">
              <button
                onclick={toggleJsonImport}
                class="json-import-button w-full p-4 text-left border-2 border-dashed rounded-lg transition-colors group"
              >
                <div class="flex items-center justify-center">
                  <div class="json-import-icon p-2 rounded-lg mr-3">
                    <Code size={20} class="json-import-icon-svg" />
                  </div>
                  <div class="text-center">
                    <h4 class="json-import-title font-medium">Import JSON Configuration</h4>
                    <p class="json-import-subtitle text-sm">Paste or type MCP server configuration</p>
                  </div>
                </div>
              </button>
            </div>
          </div>

        {:else if currentStep === 2}
          <!-- Step 2: Basic Information -->
          <div class="space-y-4">
            <div class="text-center mb-6">
              <h3 class="text-lg font-medium mb-2 text-primary">Server Information</h3>
              <p class="text-secondary">Provide basic details about your MCP server</p>
            </div>

            <div>
              <label for="add-server-name" class="form-label">Server Name *</label>
              <input
                id="add-server-name"
                type="text"
                bind:value={formData.name}
                placeholder="My MCP Server"
                class="form-input"
                class:border-red-500={duplicateNameServer}
                required
              />
              <!-- ✅ NEW: Duplicate name warning -->
              {#if duplicateNameServer}
                <div class="mt-2 p-3 bg-yellow-50 dark:bg-yellow-900/20 border-l-4 border-yellow-400 dark:border-yellow-500 rounded-r-md">
                  <div class="flex items-start">
                    <AlertCircle size={16} class="text-yellow-600 dark:text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                    <div class="flex-1">
                      <p class="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                        ⚠️ A server named "{formData.name}" already exists
                      </p>
                      <p class="text-xs text-yellow-700 dark:text-yellow-300 mt-1">
                        Please use a different name to continue
                      </p>
                    </div>
                  </div>
                </div>
              {/if}
            </div>

            <div>
              <label for="add-server-description" class="form-label">Description</label>
              <textarea
                id="add-server-description"
                bind:value={formData.description}
                placeholder="Brief description of what this server does..."
                class="form-input h-20 resize-none"
              ></textarea>
            </div>

            <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <div class="flex items-center mb-2">
                {#if transportTypes.find(t => t.id === formData.transportType)?.icon}
                  {@const IconComponent = transportTypes.find(t => t.id === formData.transportType)?.icon || Database}
                  <IconComponent size={16} class="text-blue-600 mr-2" />
                {:else}
                  <Database size={16} class="text-blue-600 mr-2" />
                {/if}
                <span class="text-sm font-medium text-blue-900">
                  {transportTypes.find(t => t.id === formData.transportType)?.name} Transport
                </span>
              </div>
              <p class="text-xs text-blue-700">
                {transportTypes.find(t => t.id === formData.transportType)?.description}
              </p>
            </div>
          </div>

        {:else if currentStep === 3}
          <!-- Step 3: Transport Configuration -->
          <div class="space-y-4">
            <div class="text-center mb-6">
              <h3 class="text-lg font-medium mb-2 text-primary">Transport Configuration</h3>
              <p class="text-secondary">Configure the connection details for your {formData.transportType.toUpperCase()} server</p>
            </div>

            {#if formData.transportType === 'stdio'}
              <div>
                <label for="add-server-stdio-command" class="form-label">Command *</label>
                <div class="flex gap-2">
                  <input
                    id="add-server-stdio-command"
                    type="text"
                    bind:value={formData.stdio.command}
                    placeholder="node server.js"
                    class="form-input flex-1"
                    required
                  />
                  <button
                    type="button"
                    onclick={selectExecutable}
                    class="btn-secondary px-3 py-2 text-sm whitespace-nowrap"
                  >
                    Browse
                  </button>
                </div>
                {#if isAbsolutePath && derivedWorkingDirectory}
                  <p class="text-xs text-gray-500 mt-1">
                    <span class="font-medium">Working directory:</span> {derivedWorkingDirectory}
                  </p>
                {/if}
              </div>

              <div>
                <label for="add-server-stdio-args" class="form-label">Arguments</label>
                <input
                  id="add-server-stdio-args"
                  type="text"
                  bind:value={argsInput}
                  oninput={updateArgs}
                  placeholder="--port 3000 --verbose"
                  class="form-input"
                />
                <p class="text-xs text-gray-500 mt-1">Space-separated command arguments</p>
              </div>

              {#if !isAbsolutePath}
                <div>
                  <label for="add-server-stdio-workdir" class="form-label">Working Directory</label>
                  <div class="flex gap-2">
                    <input
                      id="add-server-stdio-workdir"
                      type="text"
                      bind:value={formData.stdio.workingDirectory}
                      placeholder="/path/to/server"
                      class="form-input flex-1"
                    />
                    <button
                      type="button"
                      onclick={selectDirectory}
                      class="btn-secondary px-3 py-2 text-sm whitespace-nowrap"
                    >
                      Browse
                    </button>
                  </div>
                  <p class="text-xs text-gray-500 mt-1">Optional working directory for the command</p>
                </div>
              {:else}
                <div>
                  <label for="add-server-stdio-workdir-override" class="form-label">Working Directory Override</label>
                  <div class="flex gap-2">
                    <input
                      id="add-server-stdio-workdir-override"
                      type="text"
                      bind:value={formData.stdio.workingDirectory}
                      placeholder="Leave empty to use directory from command path"
                      class="form-input flex-1"
                    />
                    <button
                      type="button"
                      onclick={selectDirectory}
                      class="btn-secondary px-3 py-2 text-sm whitespace-nowrap"
                    >
                      Browse
                    </button>
                  </div>
                  <p class="text-xs text-gray-500 mt-1">Override the directory derived from the command path</p>
                </div>
              {/if}

            {:else if formData.transportType === 'http' || formData.transportType === 'websocket'}
              <div>
                <label for="add-server-http-url" class="form-label">URL *</label>
                <input
                  id="add-server-http-url"
                  type="url"
                  value={formData.transportType === 'http' ? formData.http.url : formData.websocket.url}
                  oninput={(e) => {
                    const value = e.currentTarget.value;
                    if (formData.transportType === 'http') {
                      formData.http.url = value;
                    } else {
                      formData.websocket.url = value;
                    }
                  }}
                  placeholder={formData.transportType === 'http' ? 'https://api.example.com/mcp' : 'wss://ws.example.com/mcp'}
                  class="form-input"
                  required
                />
              </div>

              <div>
                <label class="form-label">Headers</label>
                {#each headerPairs as pair, index}
                  <div class="flex items-center space-x-2 mb-2">
                    <label for="add-server-header-key-{index}" class="sr-only">Header name</label>
                    <input
                      id="add-server-header-key-{index}"
                      type="text"
                      bind:value={pair.key}
                      oninput={updateHeaders}
                      placeholder="Header name"
                      class="form-input flex-1"
                    />
                    <label for="add-server-header-value-{index}" class="sr-only">Header value</label>
                    <input
                      id="add-server-header-value-{index}"
                      type="text"
                      bind:value={pair.value}
                      oninput={updateHeaders}
                      placeholder="Header value"
                      class="form-input flex-1"
                    />
                    <button
                      onclick={() => removeHeaderPair(index)}
                      class="p-2 text-red-600 hover:bg-red-50 rounded"
                      aria-label="Remove header"
                    >
                      <X size={16} />
                    </button>
                  </div>
                {/each}
                <button
                  onclick={addHeaderPair}
                  class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700"
                >
                  + Add Header
                </button>
              </div>

            {:else if formData.transportType === 'tcp'}
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label for="add-server-tcp-host" class="form-label">Host *</label>
                  <input
                    id="add-server-tcp-host"
                    type="text"
                    bind:value={formData.tcp.host}
                    placeholder="localhost"
                    class="form-input"
                    required
                  />
                </div>
                <div>
                  <label for="add-server-tcp-port" class="form-label">Port *</label>
                  <input
                    id="add-server-tcp-port"
                    type="number"
                    bind:value={formData.tcp.port}
                    placeholder="8080"
                    class="form-input"
                    required
                  />
                </div>
              </div>

            {:else if formData.transportType === 'unix'}
              <div>
                <label for="add-server-unix-path" class="form-label">Socket Path *</label>
                <input
                  id="add-server-unix-path"
                  type="text"
                  bind:value={formData.unix.path}
                  placeholder="/tmp/mcp.sock"
                  class="form-input"
                  required
                />
              </div>
            {/if}

            <!-- ✅ NEW: Duplicate transport config warning -->
            {#if duplicateTransportServer}
              <div class="p-3 bg-yellow-50 dark:bg-yellow-900/20 border-l-4 border-yellow-400 dark:border-yellow-500 rounded-r-md">
                <div class="flex items-start">
                  <AlertCircle size={16} class="text-yellow-600 dark:text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                  <div class="flex-1">
                    <p class="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                      ⚠️ Similar configuration detected
                    </p>
                    <p class="text-xs text-yellow-700 dark:text-yellow-300 mt-1">
                      Server <strong>"{duplicateTransportServer?.config.name}"</strong> already uses the same
                      {formData.transportType === 'stdio' ? 'command' :
                       formData.transportType === 'http' || formData.transportType === 'websocket' ? 'URL' :
                       formData.transportType === 'tcp' ? 'host and port' : 'socket path'}.
                      This may connect to the same server instance.
                    </p>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Environment Variables -->
            <div>
              <label class="form-label">Environment Variables</label>
              {#each envPairs as pair, index}
                <div class="flex items-center space-x-2 mb-2">
                  <input
                    type="text"
                    bind:value={pair.key}
                    oninput={updateEnvironmentVariables}
                    placeholder="Variable name"
                    class="form-input flex-1"
                  />
                  <input
                    type="text"
                    bind:value={pair.value}
                    oninput={updateEnvironmentVariables}
                    placeholder="Variable value"
                    class="form-input flex-1"
                  />
                  <button
                    onclick={() => removeEnvPair(index)}
                    class="p-2 text-red-600 hover:bg-red-50 rounded"
                  >
                    <X size={16} />
                  </button>
                </div>
              {/each}
              <button
                onclick={addEnvPair}
                class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700"
              >
                + Add Variable
              </button>
            </div>

            <!-- Test Configuration -->
            <div class="border-t border-gray-200 pt-4 space-y-4">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-gray-900">Test Configuration</h4>
                <button
                  onclick={testConfiguration}
                  disabled={modalLoading}
                  class="btn-secondary {modalLoading ? 'opacity-50' : ''}"
                >
                  <TestTube size={16} class="mr-2" />
                  {modalLoading ? 'Testing...' : 'Test'}
                </button>
              </div>

              {#if testResult}
                <div class="p-3 rounded-lg {testResult.success ? 'bg-green-50 border border-green-200' : 'bg-red-50 border border-red-200'}">
                  <div class="flex items-center">
                    {#if testResult.success}
                      <Check size={16} class="text-green-600 mr-2" />
                      <span class="text-green-700 text-sm">{testResult.message}</span>
                    {:else}
                      <AlertCircle size={16} class="text-red-600 mr-2" />
                      <span class="text-red-700 text-sm">{testResult.message}</span>
                    {/if}
                  </div>
                </div>
              {/if}

              <!-- ✅ NEW: Connection option -->
              <div class="p-4 bg-gray-50 rounded-lg border border-gray-200">
                <label class="flex items-center cursor-pointer group">
                  <input
                    type="checkbox"
                    bind:checked={connectAfterAdding}
                    class="w-4 h-4 text-mcp-primary-600 border-gray-300 rounded focus:ring-mcp-primary-500 cursor-pointer"
                  />
                  <div class="ml-3 flex-1">
                    <span class="text-sm font-medium text-gray-900 group-hover:text-mcp-primary-700">
                      Connect immediately after adding
                    </span>
                    <p class="text-xs text-gray-600 mt-0.5">
                      {#if connectAfterAdding}
                        Will attempt to connect to the server after saving configuration
                      {:else}
                        Server will be added but not connected (you can connect later)
                      {/if}
                    </p>
                  </div>
                </label>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between p-6 border-t border-gray-200 bg-gray-50">
        <div class="flex space-x-3">
          {#if currentStep > 1}
            <button onclick={prevStep} class="btn-secondary">
              Back
            </button>
          {/if}
        </div>

        <div class="flex space-x-3">
          <button onclick={closeModal} class="btn-secondary">
            Cancel
          </button>
          
          {#if currentStep < 3}
            <button
              onclick={nextStep}
              class="btn-primary"
              disabled={currentStep === 2 && (!formData.name || !!duplicateNameServer)}
            >
              Next
            </button>
          {:else}
            <button
              onclick={addServer}
              disabled={modalLoading || !!duplicateNameServer}
              class="btn-primary {(modalLoading || duplicateNameServer) ? 'opacity-50 cursor-not-allowed' : ''}"
            >
              {#if modalLoading}
                <div class="flex items-center">
                  <div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full mr-2"></div>
                  {connectAfterAdding ? 'Adding & Connecting...' : 'Adding...'}
                </div>
              {:else}
                {#if connectAfterAdding}
                  <Play size={16} class="mr-2" />
                  Add & Connect
                {:else}
                  <Check size={16} class="mr-2" />
                  Add Server
                {/if}
              {/if}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- JSON Import Modal - Layered on top of main modal -->
{#if showJsonImport}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="fixed inset-0 bg-black bg-opacity-60 flex items-center justify-center z-60 p-4"
    onclick={(e) => {
      // Click outside closes JSON import, reveals main modal
      if (e.target === e.currentTarget) cancelJsonImport();
    }}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] flex flex-col overflow-hidden"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 sm:p-6 border-b border-gray-200 flex-shrink-0">
        <h2 class="text-lg sm:text-xl font-semibold text-gray-900">Import JSON Configuration</h2>
        <button onclick={cancelJsonImport} class="text-gray-400 hover:text-gray-600" aria-label="Cancel JSON import">
          <X size={24} />
        </button>
      </div>

      <!-- Content - Flexible with scroll -->
      <div class="flex-1 flex flex-col p-4 sm:p-6 overflow-hidden min-h-0">
        <!-- Clipboard Detection Notice -->
        {#if clipboardDetected}
          <div class="mb-3 p-3 rounded-lg bg-blue-50 border border-blue-200 flex-shrink-0">
            <div class="flex items-start">
              <div class="p-2 rounded-lg bg-blue-500 text-white mr-3 flex-shrink-0">
                <Check size={20} />
              </div>
              <div class="flex-1 min-w-0">
                <h4 class="font-semibold text-blue-900 mb-1 text-sm sm:text-base">Configuration Auto-Detected</h4>
                <p class="text-xs sm:text-sm text-blue-800">
                  Found <strong>{detectedServerName}</strong> in your clipboard. You can edit the configuration below before adding.
                </p>
              </div>
            </div>
          </div>
        {/if}

        <!-- Textarea Container - Takes remaining space -->
        <div class="flex-1 flex flex-col min-h-0 mb-3">
          <label class="form-label mb-2 flex-shrink-0">JSON Configuration</label>
          <textarea
            bind:value={jsonConfig}
            placeholder='Paste your MCP server configuration JSON here...'
            class="form-input font-mono text-xs sm:text-sm flex-1 min-h-[200px] resize-none"
          ></textarea>
          <p class="text-xs text-gray-500 mt-2 flex-shrink-0">
            {#if detectedServerCount > 1}
              Multiple servers detected. All {detectedServerCount} servers will be added (connect manually when ready).
            {:else if detectedServerCount === 1}
              Single server configuration detected. You can edit before adding.
            {:else}
              Paste a valid MCP server configuration in JSON format using the <code>mcpServers</code> schema.
            {/if}
          </p>
        </div>

        {#if jsonError}
          <div class="mb-3 p-3 rounded-lg bg-red-50 border border-red-200 flex-shrink-0">
            <div class="flex items-center">
              <AlertCircle size={16} class="text-red-600 mr-2" />
              <span class="text-red-700 text-xs sm:text-sm">{jsonError}</span>
            </div>
          </div>
        {/if}

        <!-- Footer Buttons - Fixed at bottom -->
        <div class="flex flex-col sm:flex-row justify-between gap-3 flex-shrink-0">
          {#if !clipboardDetected}
            <button onclick={exportToJson} class="btn-secondary text-sm">
              <Download size={16} class="mr-2" />
              <span class="hidden sm:inline">Export Current Config</span>
              <span class="sm:hidden">Export</span>
            </button>
          {:else}
            <div></div>
          {/if}

          <div class="flex flex-col sm:flex-row gap-2 sm:gap-3">
            <button
              onclick={cancelJsonImport}
              class="btn-secondary text-sm order-2 sm:order-1"
            >
              Cancel
            </button>
            <button
              onclick={importFromJson}
              disabled={!jsonConfig.trim() || modalLoading}
              class="btn-primary text-sm order-1 sm:order-2 {modalLoading ? 'opacity-50 cursor-not-allowed' : ''}"
            >
              {#if modalLoading}
                <div class="flex items-center justify-center">
                  <div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full mr-2"></div>
                  <span class="hidden sm:inline">Adding...</span>
                  <span class="sm:hidden">Adding...</span>
                </div>
              {:else}
                <Check size={16} class="mr-2" />
                <span class="hidden sm:inline">{detectedServerCount > 1 ? 'Add All Servers' : 'Add Server'}</span>
                <span class="sm:hidden">Add</span>
              {/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Transport Option Button Styling - Dark Mode Compatible */
  .transport-option-button {
    padding: 1rem;
    text-align: left;
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    transition: all var(--mcp-transition-fast);
    background-color: var(--mcp-surface-primary);
    color: var(--mcp-text-primary);
    cursor: pointer;
    width: 100%;
  }

  .transport-option-button:hover {
    border-color: var(--mcp-primary-500);
    background-color: var(--mcp-surface-tertiary);
  }

  .transport-option-icon {
    padding: 0.5rem;
    background-color: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-lg);
    margin-right: 1rem;
    transition: all var(--mcp-transition-fast);
  }

  .transport-option-button:hover .transport-option-icon {
    background-color: var(--mcp-surface-elevated);
  }

  .transport-option-icon-svg {
    color: var(--mcp-text-secondary);
    transition: color var(--mcp-transition-fast);
  }

  .transport-option-button:hover .transport-option-icon-svg {
    color: var(--mcp-primary-600);
  }

  .transport-option-title {
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    margin-bottom: 0.25rem;
  }

  .transport-option-description {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
    margin-bottom: 0.25rem;
  }

  .transport-option-example {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    font-family: var(--mcp-font-mono);
  }

  /* JSON Import Button Styles - Same pattern as transport buttons */
  .json-import-button {
    padding: 1rem;
    text-align: left;
    border: 2px solid var(--mcp-border-primary);
    border-style: dashed;
    border-radius: var(--mcp-radius-lg);
    transition: all var(--mcp-transition-fast);
    background-color: var(--mcp-surface-primary);
    color: var(--mcp-text-primary);
    cursor: pointer;
    width: 100%;
  }

  .json-import-button:hover {
    border-color: var(--mcp-primary-500);
    background-color: var(--mcp-surface-tertiary);
  }

  .json-import-icon {
    padding: 0.5rem;
    background-color: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-lg);
    margin-right: 0.75rem;
    transition: all var(--mcp-transition-fast);
  }

  .json-import-button:hover .json-import-icon {
    background-color: var(--mcp-surface-elevated);
  }

  .json-import-icon-svg {
    color: var(--mcp-text-secondary);
    transition: color var(--mcp-transition-fast);
  }

  .json-import-button:hover .json-import-icon-svg {
    color: var(--mcp-primary-600);
  }

  .json-import-title {
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    margin-bottom: 0.25rem;
  }

  .json-import-subtitle {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  /* ✅ NEW: Custom z-index for layered modals */
  .z-60 {
    z-index: 60;
  }
</style>
