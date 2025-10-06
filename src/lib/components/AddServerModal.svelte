<script lang="ts">
  import { serverStore, type ServerConfig, type TransportConfig } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
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

  let showModal = $state(true);
  let currentStep = $state(1);
  let loading = $state(false);
  let testResult = $state<{ success: boolean; message?: string } | null>(null);
  let templates: ServerConfig[] = $state([]);
  let showJsonImport = $state(false);
  let jsonConfig = $state('');
  let jsonError = $state('');

  // Clipboard detection state
  let clipboardDetected = $state(false);
  let detectedServerName = $state('');
  let detectedServerCount = $state(0);
  let showClipboardBanner = $state(false);

  // Form data
  let formData = $state({
    name: '',
    description: '',
    transportType: 'stdio' as TransportConfig['type'],
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

  // Computed values for STDIO command handling
  const isAbsolutePath = $derived(() => {
    const command = formData.stdio.command.trim();
    return command.startsWith('/') || /^[A-Za-z]:[/\\]/.test(command); // Unix absolute or Windows absolute
  });

  const derivedWorkingDirectory = $derived(() => {
    if (!isAbsolutePath()) return '';
    const command = formData.stdio.command.trim();
    const lastSlash = Math.max(command.lastIndexOf('/'), command.lastIndexOf('\\'));
    return lastSlash > 0 ? command.substring(0, lastSlash) : '';
  });

  const transportTypes = [
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

  $effect(() => {
    loadTemplates();
  });

  // Check clipboard for MCP config when modal opens
  $effect(() => {
    if (showModal && currentStep === 1) {
      checkClipboardForConfig();
    }
  });

  async function loadTemplates() {
    try {
      await serverStore.loadTemplates();
      const state = serverStore.subscribe((s: any) => {
        templates = s.templates;
      });
      return state;
    } catch (error) {
      console.error('Failed to load templates:', error);
    }
  }

  function closeModal() {
    uiStore.closeModal('addServer');
    showModal = false;
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

  function selectTransport(type: TransportConfig['type']) {
    formData.transportType = type;
    nextStep();
  }

  function useTemplate(template: ServerConfig) {
    formData.name = template.name;
    formData.description = template.description || '';
    formData.transportType = template.transport_config.type;
    
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
      case 'websocket':
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

  // Directory picker for STDIO transport
  async function selectDirectory() {
    try {
      // Temporarily hide modal to prevent z-index interference
      const originalShow = showModal;
      showModal = false;

      // Small delay to ensure modal is hidden
      await new Promise(resolve => setTimeout(resolve, 100));

      const result = await open({
        directory: true,
        multiple: false,
        title: 'Select Working Directory'
      });

      // Restore modal
      showModal = originalShow;

      if (result && typeof result === 'string') {
        formData.stdio.workingDirectory = result;
      }
    } catch (error) {
      console.error('Failed to select directory:', error);
      // Ensure modal is restored even on error
      showModal = true;
    }
  }

  // File picker for executable selection
  async function selectExecutable() {
    try {
      // Temporarily hide modal to prevent z-index interference
      const originalShow = showModal;
      showModal = false;

      // Small delay to ensure modal is hidden
      await new Promise(resolve => setTimeout(resolve, 100));

      const result = await open({
        directory: false,
        multiple: false,
        title: 'Select Server Executable'
        // No filters - allow all files including extensionless executables
      });

      // Restore modal
      showModal = originalShow;

      if (result && typeof result === 'string') {
        formData.stdio.command = result;
      }
    } catch (error) {
      console.error('Failed to select executable:', error);
      // Ensure modal is restored even on error
      showModal = true;
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

  // Clipboard detection for MCP config
  async function checkClipboardForConfig() {
    try {
      // Use browser's native clipboard API (available in Tauri)
      const clipboardText = await navigator.clipboard.readText();
      if (!clipboardText || !clipboardText.trim()) return;

      const detected = detectMCPConfig(clipboardText);

      if (detected.valid) {
        clipboardDetected = true;
        detectedServerCount = detected.servers.length;
        jsonConfig = clipboardText; // Store for both single and multiple

        if (detected.servers.length === 1) {
          // Single server: pre-fill form and show banner
          detectedServerName = detected.servers[0].name;
          showClipboardBanner = true;
        } else if (detected.servers.length > 1) {
          // Multiple servers: show JSON import with all servers
          detectedServerName = `${detected.servers.length} servers`;
          showClipboardBanner = true;
        }
      }
    } catch (error) {
      // Silently fail - clipboard might be empty, inaccessible, or not text
      // This is expected behavior, not an error condition
    }
  }

  function detectMCPConfig(text: string): { valid: boolean; servers: Array<{name: string, config: any}> } {
    try {
      const parsed = JSON.parse(text);
      if (parsed.mcpServers && typeof parsed.mcpServers === 'object') {
        const servers = Object.entries(parsed.mcpServers).map(([name, config]) => ({
          name,
          config: config as any
        }));
        return { valid: true, servers };
      }
    } catch {}
    return { valid: false, servers: [] };
  }

  function useDetectedConfig() {
    if (detectedServerCount === 1) {
      // Single server: import and jump to step 3
      importFromJson();
      showClipboardBanner = false;
    } else {
      // Multiple servers: open JSON editor
      showJsonImport = true;
      showClipboardBanner = false;
    }
  }

  function dismissDetection() {
    showClipboardBanner = false;
    clipboardDetected = false;
  }

  function importFromJson() {
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

      // Handle single vs multiple servers
      if (serverNames.length === 1) {
        // Single server: use existing logic (import to form)
        importSingleServer(serverNames[0], config.mcpServers[serverNames[0]]);
      } else {
        // Multiple servers: import all and connect
        importMultipleServers(config.mcpServers);
      }
    } catch (error) {
      jsonError = `Failed to import config: ${error}`;
    }
  }

  function importSingleServer(serverName: string, serverConfig: any) {
    // Import basic fields
    formData.name = serverName;
    formData.description = serverConfig.description || '';

    // Determine transport type based on config properties
    if (serverConfig.command) {
      // STDIO transport
      formData.transportType = 'stdio';
      formData.stdio.command = serverConfig.command;
      formData.stdio.args = serverConfig.args || [];
      formData.stdio.workingDirectory = serverConfig.cwd || '';
      argsInput = (serverConfig.args || []).join(' ');

      // Import environment variables
      formData.environmentVariables = serverConfig.env || {};
    } else if (serverConfig.url) {
      // Determine if it's HTTP or WebSocket based on URL scheme
      const url = serverConfig.url.toLowerCase();
      if (url.startsWith('ws://') || url.startsWith('wss://')) {
        formData.transportType = 'websocket';
        formData.websocket.url = serverConfig.url;
        formData.websocket.headers = serverConfig.headers || {};
        headerPairs = Object.entries(formData.websocket.headers).map(([key, value]) => ({ key, value }));
      } else {
        formData.transportType = 'http';
        formData.http.url = serverConfig.url;
        formData.http.headers = serverConfig.headers || {};
        headerPairs = Object.entries(formData.http.headers).map(([key, value]) => ({ key, value }));
      }
      formData.environmentVariables = {};
    } else if (serverConfig.host && serverConfig.port) {
      // TCP transport
      formData.transportType = 'tcp';
      formData.tcp.host = serverConfig.host;
      formData.tcp.port = serverConfig.port;
      formData.environmentVariables = {};
    } else if (serverConfig.path) {
      // Unix socket transport
      formData.transportType = 'unix';
      formData.unix.path = serverConfig.path;
      formData.environmentVariables = {};
    } else {
      throw new Error('Unable to determine transport type from configuration');
    }

    // Ensure header and env pairs have at least one empty entry
    if (headerPairs.length === 0) headerPairs = [{ key: '', value: '' }];

    envPairs = Object.entries(formData.environmentVariables).map(([key, value]) => ({ key, value }));
    if (envPairs.length === 0) envPairs = [{ key: '', value: '' }];

    // Jump to final step
    currentStep = 3;
    showJsonImport = false;
  }

  async function importMultipleServers(serversConfig: Record<string, any>) {
    try {
      const serverNames = Object.keys(serversConfig);
      const importPromises = serverNames.map(async (name) => {
        const serverConfig = serversConfig[name];

        // Build ServerConfig for each server
        const config = await convertMCPConfigToServerConfig(name, serverConfig);
        return serverStore.connectServer(config);
      });

      const results = await Promise.allSettled(importPromises);
      const successful = results.filter(r => r.status === 'fulfilled').length;
      const failed = results.filter(r => r.status === 'rejected').length;

      if (successful > 0) {
        uiStore.showSuccess(`Successfully imported and connected ${successful}/${serverNames.length} server(s)`);
      }
      if (failed > 0) {
        uiStore.showError(`Failed to import ${failed} server(s)`);
      }

      // Close modals on success
      showJsonImport = false;
      closeModal();
    } catch (error) {
      jsonError = `Failed to import servers: ${error}`;
    }
  }

  async function convertMCPConfigToServerConfig(name: string, mcpConfig: any): Promise<ServerConfig> {
    let transport: TransportConfig;

    if (mcpConfig.command) {
      transport = {
        type: 'stdio',
        command: mcpConfig.command,
        args: mcpConfig.args || [],
        working_directory: mcpConfig.cwd || undefined,
      };
    } else if (mcpConfig.url) {
      const url = mcpConfig.url.toLowerCase();
      if (url.startsWith('ws://') || url.startsWith('wss://')) {
        transport = {
          type: 'websocket',
          url: mcpConfig.url,
          headers: mcpConfig.headers || {},
        };
      } else {
        transport = {
          type: 'http',
          url: mcpConfig.url,
          headers: mcpConfig.headers || {},
        };
      }
    } else if (mcpConfig.host && mcpConfig.port) {
      transport = {
        type: 'tcp',
        host: mcpConfig.host,
        port: mcpConfig.port,
      };
    } else if (mcpConfig.path) {
      transport = {
        type: 'unix',
        path: mcpConfig.path,
      };
    } else {
      throw new Error(`Unable to determine transport type for server: ${name}`);
    }

    return await serverStore.createServerConfig(
      name,
      mcpConfig.description || undefined,
      transport,
      mcpConfig.env || {}
    );
  }

  async function testConfiguration() {
    loading = true;
    testResult = null;

    try {
      const config = await buildConfig();
      const result = await serverStore.testServerConfig(config);
      testResult = { success: result, message: result ? 'Configuration is valid' : 'Configuration test failed' };
    } catch (error) {
      testResult = { success: false, message: `Test failed: ${error}` };
    } finally {
      loading = false;
    }
  }

  async function buildConfig(): Promise<ServerConfig> {
    updateArgs();
    updateHeaders();
    updateEnvironmentVariables();

    let transport: TransportConfig;
    
    switch (formData.transportType) {
      case 'stdio':
        // Use explicit working directory if provided, otherwise derive from absolute path
        let workingDir = formData.stdio.workingDirectory;
        if (!workingDir && isAbsolutePath()) {
          workingDir = derivedWorkingDirectory();
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
          type: 'websocket',
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

    return await serverStore.createServerConfig(
      formData.name,
      formData.description || undefined,
      transport,
      formData.environmentVariables
    );
  }

  async function saveAndConnect() {
    loading = true;
    
    try {
      const config = await buildConfig();
      await serverStore.connectServer(config);
      uiStore.showSuccess(`Connected to ${config.name}`);
      closeModal();
    } catch (error) {
      uiStore.showError(`Failed to connect: ${error}`);
    } finally {
      loading = false;
    }
  }
</script>

{#if showModal}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
    <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <div>
          <h2 class="text-xl font-semibold text-gray-900">Add MCP Server</h2>
          <p class="text-sm text-gray-600 mt-1">Step {currentStep} of 3</p>
        </div>
        <button onclick={closeModal} class="text-gray-400 hover:text-gray-600">
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

      <!-- Content -->
      <div class="p-6 overflow-y-auto max-h-[60vh]">
        {#if currentStep === 1}
          <!-- Step 1: Choose Transport Type -->
          <div class="space-y-4">
            <!-- Clipboard Detection Banner -->
            {#if showClipboardBanner}
              <div class="clipboard-banner">
                <div class="flex items-start">
                  <div class="clipboard-banner-icon">
                    <Check size={20} />
                  </div>
                  <div class="flex-1">
                    <h4 class="clipboard-banner-title">
                      Configuration Detected
                    </h4>
                    <p class="clipboard-banner-text">
                      Found <strong>{detectedServerName}</strong> in clipboard
                      {#if detectedServerCount > 1}
                        - {detectedServerCount} servers ready to import
                      {/if}
                    </p>
                  </div>
                  <div class="flex items-center space-x-2">
                    <button onclick={useDetectedConfig} class="btn-primary text-sm">
                      Use {detectedServerCount > 1 ? 'These' : 'This'}
                    </button>
                    <button onclick={dismissDetection} class="btn-ghost text-sm">
                      Dismiss
                    </button>
                  </div>
                </div>
              </div>
            {/if}

            <div class="text-center mb-6">
              <h3 class="text-lg font-medium mb-2 text-primary">Choose Transport Type</h3>
              <p class="text-secondary">Select how you want to connect to your MCP server</p>
            </div>

            <div class="grid grid-cols-1 gap-3">
              {#each transportTypes as transport}
                <button
                  onclick={() => selectTransport(transport.id as TransportConfig['type'])}
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

            <!-- Templates -->

            <!-- JSON Import Option -->
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
                    <p class="json-import-subtitle text-sm">Paste or edit a JSON configuration</p>
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
                required
              />
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
                {#if isAbsolutePath() && derivedWorkingDirectory()}
                  <p class="text-xs text-gray-500 mt-1">
                    <span class="font-medium">Working directory:</span> {derivedWorkingDirectory()}
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

              {#if !isAbsolutePath()}
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
            <div class="border-t border-gray-200 pt-4">
              <div class="flex items-center justify-between mb-3">
                <h4 class="text-sm font-medium text-gray-900">Test Configuration</h4>
                <button
                  onclick={testConfiguration}
                  disabled={loading}
                  class="btn-secondary {loading ? 'opacity-50' : ''}"
                >
                  <TestTube size={16} class="mr-2" />
                  {loading ? 'Testing...' : 'Test'}
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
              disabled={currentStep === 2 && !formData.name}
            >
              Next
            </button>
          {:else}
            <button
              onclick={saveAndConnect}
              disabled={loading}
              class="btn-primary {loading ? 'opacity-50' : ''}"
            >
              <Play size={16} class="mr-2" />
              {loading ? 'Connecting...' : 'Connect'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- JSON Import Modal -->
{#if showJsonImport}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <h2 class="text-xl font-semibold text-gray-900">Import JSON Configuration</h2>
        <button onclick={() => showJsonImport = false} class="text-gray-400 hover:text-gray-600">
          <X size={24} />
        </button>
      </div>

      <!-- Content -->
      <div class="p-6">
        <div class="mb-4">
          <label class="form-label">JSON Configuration</label>
          <textarea
            bind:value={jsonConfig}
            placeholder='Paste your MCP server configuration JSON here...'
            class="form-input h-64 font-mono text-sm"
            rows="12"
          ></textarea>
          <p class="text-xs text-gray-500 mt-1">
            Paste a valid MCP server configuration in JSON format. The configuration will be parsed and applied to the form.
          </p>
        </div>

        {#if jsonError}
          <div class="mb-4 p-3 rounded-lg bg-red-50 border border-red-200">
            <div class="flex items-center">
              <AlertCircle size={16} class="text-red-600 mr-2" />
              <span class="text-red-700 text-sm">{jsonError}</span>
            </div>
          </div>
        {/if}

        <div class="flex justify-between">
          <button onclick={exportToJson} class="btn-secondary">
            <Download size={16} class="mr-2" />
            Export Current Config
          </button>

          <div class="flex space-x-3">
            <button onclick={() => showJsonImport = false} class="btn-secondary">
              Cancel
            </button>
            <button onclick={importFromJson} class="btn-primary" disabled={!jsonConfig.trim()}>
              <Upload size={16} class="mr-2" />
              Import Configuration
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

  /* Clipboard Detection Banner */
  .clipboard-banner {
    @apply mb-6 p-4 border rounded-lg;
    background: linear-gradient(135deg, #e0f2fe 0%, #dbeafe 100%);
    border-color: #3b82f6;
  }

  .clipboard-banner-icon {
    @apply p-2 rounded-lg mr-3 flex-shrink-0;
    background-color: #3b82f6;
    color: white;
  }

  .clipboard-banner-title {
    @apply font-semibold text-blue-900 mb-1;
  }

  .clipboard-banner-text {
    @apply text-sm text-blue-800;
  }
</style>