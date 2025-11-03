<!--
  AICopilot.svelte

  Contextual AI assistant sidebar showing:
  - Contextual suggestions (based on selected message/test/insight)
  - Inline chat (ask questions about the workspace)
  - Action buttons for common tasks

  Stays in sync with workspace state:
  - Selecting a message → suggestions change
  - Test fails → fix suggestion appears
  - Untested tools → generate suggestion appears
-->

<script lang="ts">
  import { Send, Loader2, Lightbulb, MessageSquare, Zap, AlertCircle } from 'lucide-svelte';
  import type { ProtocolMessage, TestData, DetectedPattern, AISuggestion } from '$lib/stores/workspaceStore';
  import { createLogger } from '$lib/utils/logger';

  interface Props {
    selectedMessage: ProtocolMessage | null;
    selectedTest: TestData | null;
    suggestions: AISuggestion[];
    insights: DetectedPattern[];
    onSuggestion: (suggestion: AISuggestion) => void;
    onInsight: (insight: DetectedPattern) => void;
  }

  const { selectedMessage, selectedTest, suggestions, insights, onSuggestion, onInsight } = $props();

  const logger = createLogger('AICopilot');

  // Chat state
  let messages = $state<any[]>([]);
  let input = $state('');
  let isLoading = $state(false);
  let chatContainer: HTMLDivElement;

  // Tabs
  let activeTab = $state<'suggestions' | 'insights' | 'chat'>('suggestions');

  // Effects
  $effect(() => {
    // Auto-scroll chat to bottom
    if (chatContainer) {
      setTimeout(() => {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }, 0);
    }
  });

  async function sendMessage() {
    if (!input.trim() || isLoading) return;

    const userMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: input.trim(),
      timestamp: new Date(),
    };

    messages = [...messages, userMessage];
    const question = input;
    input = '';
    isLoading = true;

    try {
      // TODO: Send to AI with workspace context
      logger.info('Sending chat message:', question);

      // Simulate response for now
      const assistantMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: 'This is a placeholder response. Full AI integration coming soon.',
        timestamp: new Date(),
      };

      messages = [...messages, assistantMessage];
    } catch (e) {
      logger.error('Failed to send message:', e);
    } finally {
      isLoading = false;
    }
  }

  function handleSuggestionClick(suggestion: AISuggestion) {
    logger.info('Executing suggestion:', suggestion.id);
    onSuggestion(suggestion);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  // Build context string for AI
  function buildContext(): string {
    const parts: string[] = [];

    if (selectedMessage) {
      parts.push(`Selected message: ${selectedMessage.direction}, size: ${selectedMessage.size}B`);
      if (selectedMessage.latencyMs) {
        parts.push(`Latency: ${selectedMessage.latencyMs}ms`);
      }
    }

    if (selectedTest) {
      parts.push(`Selected test: ${selectedTest.name}`);
      if (selectedTest.lastResult) {
        parts.push(`Status: ${selectedTest.lastResult.passed ? 'PASSED' : 'FAILED'}`);
        if (selectedTest.lastResult.errorMessage) {
          parts.push(`Error: ${selectedTest.lastResult.errorMessage}`);
        }
      }
    }

    if (insights.length > 0) {
      parts.push(`${insights.length} active insights`);
    }

    return parts.join(' | ');
  }
</script>

<div class="ai-copilot">
  <!-- Tab selector -->
  <div class="tab-selector">
    <button
      class="tab-button"
      class:active={activeTab === 'suggestions'}
      onclick={() => (activeTab = 'suggestions')}
    >
      <Lightbulb size={16} />
      Suggestions
      {#if suggestions.length > 0}
        <span class="badge">{suggestions.length}</span>
      {/if}
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'insights'}
      onclick={() => (activeTab = 'insights')}
    >
      <AlertCircle size={16} />
      Insights
      {#if insights.length > 0}
        <span class="badge">{insights.length}</span>
      {/if}
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'chat'}
      onclick={() => (activeTab = 'chat')}
    >
      <MessageSquare size={16} />
      Chat
    </button>
  </div>

  <!-- Tab content -->
  <div class="tab-content">
    <!-- Suggestions tab -->
    {#if activeTab === 'suggestions'}
      <div class="suggestions-view">
        {#if suggestions.length === 0}
          <div class="empty-state">
            <Lightbulb size={40} />
            <p>Select a test or message to see suggestions</p>
          </div>
        {:else}
          <div class="suggestions-list">
            {#each suggestions as suggestion (suggestion.id)}
              <button
                class="suggestion-card"
                onclick={() => handleSuggestionClick(suggestion)}
              >
                <div class="suggestion-header">
                  <Zap size={18} />
                  <span class="suggestion-title">{suggestion.title}</span>
                </div>
                <div class="suggestion-description">
                  {suggestion.description}
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Insights tab -->
    {#if activeTab === 'insights'}
      <div class="insights-view">
        {#if insights.length === 0}
          <div class="empty-state">
            <AlertCircle size={40} />
            <p>No insights yet. Run some tests!</p>
          </div>
        {:else}
          <div class="insights-list">
            {#each insights as insight (insight.type + insight.message)}
              <button
                class="insight-summary"
                onclick={() => onInsight(insight)}
              >
                <div class="insight-header">
                  <span class={`severity-dot ${insight.severity}`}></span>
                  <span class="insight-msg">{insight.message}</span>
                </div>
                {#if insight.suggestedAction}
                  <div class="insight-action">
                    {insight.suggestedAction}
                  </div>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Chat tab -->
    {#if activeTab === 'chat'}
      <div class="chat-view">
        <div class="chat-context">
          {#if selectedMessage || selectedTest || insights.length > 0}
            <div class="context-note">
              <span class="context-label">Context:</span>
              <span class="context-text">{buildContext()}</span>
            </div>
          {/if}
        </div>

        <div class="chat-messages" bind:this={chatContainer}>
          {#each messages as message (message.id)}
            <div class="message" class:user={message.role === 'user'}>
              <div class="message-badge">
                {message.role === 'user' ? '👤' : '🤖'}
              </div>
              <div class="message-content">
                {message.content}
              </div>
            </div>
          {/each}

          {#if isLoading}
            <div class="message assistant">
              <div class="message-badge">🤖</div>
              <div class="message-content loading">
                <Loader2 size={16} class="animate-spin" />
              </div>
            </div>
          {/if}
        </div>

        <div class="chat-input">
          <textarea
            placeholder="Ask about this message, test, or workspace..."
            bind:value={input}
            onkeydown={handleKeydown}
            disabled={isLoading}
            rows={2}
          />
          <button
            class="send-button"
            onclick={sendMessage}
            disabled={!input.trim() || isLoading}
          >
            <Send size={16} />
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .ai-copilot {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  /* Tab selector */
  .tab-selector {
    display: flex;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .tab-button {
    flex: 1;
    padding: 0.75rem 0.5rem;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    border-bottom: 2px solid transparent;
    transition: all 0.15s ease;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tab-button:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .tab-button.active {
    color: var(--text-primary);
    border-bottom-color: var(--info-text);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--info-bg);
    color: var(--info-text);
    font-size: 0.75rem;
    font-weight: 700;
  }

  /* Tab content */
  .tab-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: auto;
    min-height: 0;
  }

  .suggestions-view,
  .insights-view,
  .chat-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    color: var(--text-secondary);
    gap: 0.75rem;
    flex: 1;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
    text-align: center;
  }

  /* Suggestions list */
  .suggestions-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.75rem;
    overflow: auto;
  }

  .suggestion-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
  }

  .suggestion-card:hover {
    background: var(--bg-tertiary);
    border-color: var(--text-secondary);
    transform: translateY(-1px);
  }

  .suggestion-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--info-text);
    font-weight: 600;
  }

  .suggestion-title {
    flex: 1;
    font-size: 0.9375rem;
  }

  .suggestion-description {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  /* Insights list */
  .insights-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    overflow: auto;
  }

  .insight-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    font-size: 0.875rem;
  }

  .insight-summary:hover {
    background: var(--bg-tertiary);
  }

  .insight-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
  }

  .severity-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .severity-dot.high {
    background: rgb(220, 38, 38);
  }

  .severity-dot.medium {
    background: rgb(180, 83, 9);
  }

  .severity-dot.low {
    background: rgb(59, 130, 246);
  }

  .insight-msg {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .insight-action {
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding-left: 1rem;
  }

  /* Chat view */
  .chat-view {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .chat-context {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .context-note {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
  }

  .context-label {
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-text {
    color: var(--text-primary);
    font-size: 0.8125rem;
    line-height: 1.4;
    word-break: break-word;
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
  }

  .message {
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
  }

  .message.user {
    flex-direction: row-reverse;
  }

  .message-badge {
    font-size: 1.25rem;
    flex-shrink: 0;
  }

  .message-content {
    padding: 0.75rem;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    line-height: 1.5;
    max-width: 80%;
    word-wrap: break-word;
  }

  .message.user .message-content {
    background: var(--info-bg);
    color: var(--info-text);
  }

  .message.assistant .message-content {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
  }

  .message-content.loading {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 1.5rem;
  }

  .chat-input {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem;
    border-top: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.875rem;
    font-family: inherit;
    resize: none;
    transition: border-color 0.15s ease;
  }

  textarea:focus {
    outline: none;
    border-color: var(--info-text);
  }

  textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .send-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .send-button:hover:not(:disabled) {
    background: var(--info-bg);
    color: var(--info-text);
    border-color: var(--info-text);
  }

  .send-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .send-button:active:not(:disabled) {
    transform: scale(0.95);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  :global(.animate-spin) {
    animation: spin 1s linear infinite;
  }
</style>
