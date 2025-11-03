/**
 * Unified Workspace Store
 *
 * Single source of truth coordinating:
 * - Protocol messages (what's happening)
 * - Test data (what's being tested)
 * - Chat history (what we discussed)
 * - Derived insights (patterns, failures, suggestions)
 *
 * Philosophy:
 * - No isolated component state
 * - Cross-pane awareness (selecting message → affects suggestions)
 * - Automatic pattern detection
 * - One store, one truth, full transparency
 */

import { writable, derived, get } from 'svelte/store';
import { contextStore, type ServerContext } from './contextStore';
import { createLogger } from '$lib/utils/logger';

const logger = createLogger('WorkspaceStore');

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

export interface ProtocolMessage {
  id: string;
  serverId: string;
  timestamp: number;
  direction: 'outgoing' | 'incoming';
  messageId: string;
  payload: string;
  size: number;
  latencyMs?: number;
}

export interface TestSuiteData {
  id: string;
  serverId: string;
  name: string;
  description: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
  tests: TestData[];
}

export interface TestData {
  id: string;
  suiteId: string;
  name: string;
  description: string | null;
  toolName: string;
  kind: string;
  category: 'happy_path' | 'edge_case' | 'error' | 'security' | 'load';
  complexity: 'low' | 'medium' | 'high';
  parameters: Record<string, any>;
  assertions: AssertionData[];
  lastResult?: TestResultData;
}

export interface AssertionData {
  type: string;
  expected: any;
  message?: string;
}

export interface TestResultData {
  id: string;
  testId: string;
  passed: boolean;
  errorMessage: string | null;
  actualResult?: any;
  durationMs: number;
  timestamp: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  provider?: string;
  usage?: {
    inputTokens: number;
    outputTokens: number;
  };
  cost?: number;
}

export interface DetectedPattern {
  type: 'repeated_failure' | 'slow_response' | 'untested_tool' | 'missing_edge_case';
  severity: 'low' | 'medium' | 'high';
  message: string;
  relatedTestIds?: string[];
  relatedMessageIds?: string[];
  suggestedAction?: string;
}

export interface AISuggestion {
  id: string;
  type: 'generate_test' | 'fix_test' | 'investigate' | 'report_bug' | 'optimize';
  title: string;
  description: string;
  context: {
    toolName?: string;
    testId?: string;
    messageId?: string;
    failureReason?: string;
  };
  action: () => Promise<void>;
}

export interface CoverageMatrix {
  byTool: Map<string, ToolCoverage>;
  overallPassRate: number;
  totalTests: number;
  passedTests: number;
  failedTests: number;
  testedTools: number;
  totalTools: number;
}

export interface ToolCoverage {
  toolName: string;
  happyPath: CoverageCell;
  edgeCase: CoverageCell;
  error: CoverageCell;
  security: CoverageCell;
  load: CoverageCell;
}

export interface CoverageCell {
  status: 'untested' | 'passed' | 'failed' | 'running';
  testIds: string[];
}

export interface WorkspaceState {
  // Current server context
  serverId: string | null;

  // Raw data
  protocolMessages: ProtocolMessage[];
  testSuites: TestSuiteData[];
  chatHistory: ChatMessage[];

  // UI state
  selectedMessageId: string | null;
  selectedTestId: string | null;
  selectedSuiteId: string | null;

  // Flags
  isGeneratingTests: boolean;
  isRunningTests: boolean;
  isLoadingMessages: boolean;
}

// ============================================================================
// INTERNAL STATE (writable)
// ============================================================================

function createWorkspaceStore() {
  const initialState: WorkspaceState = {
    serverId: null,
    protocolMessages: [],
    testSuites: [],
    chatHistory: [],
    selectedMessageId: null,
    selectedTestId: null,
    selectedSuiteId: null,
    isGeneratingTests: false,
    isRunningTests: false,
    isLoadingMessages: false,
  };

  const internalState = writable<WorkspaceState>(initialState);

  // ========================================================================
  // DERIVED STATE (computed)
  // ========================================================================

  // Sync with server context
  const serverContext = derived(contextStore, $ctx => $ctx);

  // Selected message details
  const selectedMessage = derived(
    [internalState, serverContext],
    ([$state, $ctx]) => {
      if (!$state.selectedMessageId) return null;
      return $state.protocolMessages.find(m => m.id === $state.selectedMessageId) || null;
    }
  );

  // Selected test details
  const selectedTest = derived(
    [internalState],
    ([$state]) => {
      if (!$state.selectedTestId) return null;
      for (const suite of $state.testSuites) {
        const test = suite.tests.find((t: TestData) => t.id === $state.selectedTestId);
        if (test) return test;
      }
      return null;
    }
  );

  // All tests flattened
  const allTests = derived(internalState, ($state: WorkspaceState) => {
    const tests: TestData[] = [];
    for (const suite of $state.testSuites) {
      tests.push(...suite.tests);
    }
    return tests;
  });

  // Coverage matrix computation
  const coverage = derived(
    [internalState, serverContext],
    ([$state, $ctx]) => {
      const matrix: CoverageMatrix = {
        byTool: new Map(),
        overallPassRate: 0,
        totalTests: 0,
        passedTests: 0,
        failedTests: 0,
        testedTools: 0,
        totalTools: 0,
      };

      // Get all tools from server capabilities
      const capabilities = $ctx.selectedServer?.capabilities;
      const tools = (capabilities && Array.isArray(capabilities.tools)) ? capabilities.tools : [];
      matrix.totalTools = tools.length;

      // Compute coverage per tool
      for (const tool of tools) {
        const toolTests = $state.testSuites
          .flatMap(s => s.tests)
          .filter(t => t.toolName === tool.name);

        if (toolTests.length === 0) {
          // No tests for this tool
          matrix.byTool.set(tool.name, {
            toolName: tool.name,
            happyPath: { status: 'untested', testIds: [] },
            edgeCase: { status: 'untested', testIds: [] },
            error: { status: 'untested', testIds: [] },
            security: { status: 'untested', testIds: [] },
            load: { status: 'untested', testIds: [] },
          });
          continue;
        }

        // Group by category
        const byCategory: Record<string, TestData[]> = {
          happy_path: [],
          edge_case: [],
          error: [],
          security: [],
          load: [],
        };

        for (const test of toolTests) {
          byCategory[test.category]?.push(test);
        }

        // Determine status per category
        const toolCoverage: ToolCoverage = {
          toolName: tool.name,
          happyPath: computeCategoryStatus(byCategory.happy_path),
          edgeCase: computeCategoryStatus(byCategory.edge_case),
          error: computeCategoryStatus(byCategory.error),
          security: computeCategoryStatus(byCategory.security),
          load: computeCategoryStatus(byCategory.load),
        };

        matrix.byTool.set(tool.name, toolCoverage);
        matrix.testedTools++;
      }

      // Compute aggregate stats
      matrix.totalTests = $state.testSuites.flatMap(s => s.tests).length;
      const results = $state.testSuites
        .flatMap(s => s.tests)
        .map(t => t.lastResult)
        .filter((r): r is TestResultData => !!r);

      matrix.passedTests = results.filter(r => r.passed).length;
      matrix.failedTests = results.filter(r => !r.passed).length;
      matrix.overallPassRate = results.length > 0
        ? (matrix.passedTests / results.length) * 100
        : 0;

      return matrix;
    }
  );

  // Detected patterns and insights
  const insights = derived(
    [internalState, allTests, coverage],
    ([$state, $tests, $coverage]) => {
      const patterns: DetectedPattern[] = [];

      // Pattern 1: Repeated failures
      const failureGroups = new Map<string, TestResultData[]>();
      for (const test of $tests) {
        if (test.lastResult?.passed === false) {
          const key = test.lastResult.errorMessage || 'unknown_error';
          if (!failureGroups.has(key)) {
            failureGroups.set(key, []);
          }
          failureGroups.get(key)!.push(test.lastResult);
        }
      }

      for (const [error, results] of failureGroups) {
        if (results.length >= 2) {
          patterns.push({
            type: 'repeated_failure',
            severity: results.length >= 3 ? 'high' : 'medium',
            message: `"${error}" affecting ${results.length} tests`,
            relatedTestIds: results.map(r => {
              const test = $tests.find(t => t.lastResult?.id === r.id);
              return test?.id || '';
            }).filter(Boolean),
            suggestedAction: `Investigate error: ${error}`,
          });
        }
      }

      // Pattern 2: Slow responses
      const slowMessages = $state.protocolMessages.filter(
        m => m.direction === 'incoming' && (m.latencyMs || 0) > 2000
      );
      if (slowMessages.length >= 2) {
        patterns.push({
          type: 'slow_response',
          severity: 'medium',
          message: `${slowMessages.length} responses > 2s (avg: ${Math.round(
            slowMessages.reduce((s, m) => s + (m.latencyMs || 0), 0) / slowMessages.length
          )}ms)`,
          relatedMessageIds: slowMessages.map(m => m.id),
          suggestedAction: 'Check server performance',
        });
      }

      // Pattern 3: Untested tools
      for (const [toolName, toolCov] of $coverage.byTool) {
        if (
          toolCov.happyPath.status === 'untested' &&
          toolCov.edgeCase.status === 'untested'
        ) {
          patterns.push({
            type: 'untested_tool',
            severity: 'low',
            message: `${toolName} has no tests`,
            suggestedAction: `Generate tests for ${toolName}`,
          });
        }
      }

      return patterns;
    }
  );

  // AI suggestions based on context
  const suggestions = derived(
    [selectedMessage, selectedTest, insights, allTests],
    ([$msg, $test, $insights, $tests]) => {
      const sugg: AISuggestion[] = [];

      // If a test is selected and failed, suggest fix
      if ($test && $test.lastResult && !$test.lastResult.passed) {
        sugg.push({
          id: `fix-${$test.id}`,
          type: 'fix_test',
          title: 'Fix failing test',
          description: `${$test.name}: ${$test.lastResult.errorMessage || 'Unknown error'}`,
          context: {
            testId: $test.id,
            failureReason: $test.lastResult.errorMessage || undefined,
          },
          action: async () => {
            // TODO: Invoke AI to fix test
            logger.info('Fixing test:', $test.id);
          },
        });
      }

      // If insights show untested tools, suggest generation
      for (const insight of $insights) {
        if (insight.type === 'untested_tool') {
          const toolName = insight.message.split(' ')[0];
          sugg.push({
            id: `gen-${toolName}`,
            type: 'generate_test',
            title: `Generate tests for ${toolName}`,
            description: insight.suggestedAction || '',
            context: { toolName },
            action: async () => {
              logger.info('Generating tests for:', toolName);
            },
          });
        }
      }

      // If message selected, suggest relevant actions
      if ($msg && $msg.direction === 'incoming') {
        sugg.push({
          id: `inspect-${$msg.id}`,
          type: 'investigate',
          title: 'Examine this response',
          description: `Size: ${$msg.size}B, Latency: ${$msg.latencyMs}ms`,
          context: { messageId: $msg.id },
          action: async () => {
            logger.info('Investigating message:', $msg.id);
          },
        });
      }

      return sugg;
    }
  );

  // ========================================================================
  // PUBLIC API
  // ========================================================================

  return {
    // Subscriptions
    subscribe: internalState.subscribe,

    // Derived state subscriptions
    serverContext: { subscribe: serverContext.subscribe },
    selectedMessage: { subscribe: selectedMessage.subscribe },
    selectedTest: { subscribe: selectedTest.subscribe },
    allTests: { subscribe: allTests.subscribe },
    coverage: { subscribe: coverage.subscribe },
    insights: { subscribe: insights.subscribe },
    suggestions: { subscribe: suggestions.subscribe },

    // Actions: state mutations
    setServerId(serverId: string | null) {
      internalState.update(s => ({ ...s, serverId }));
      logger.debug('Server selected:', serverId);
    },

    addProtocolMessage(message: ProtocolMessage) {
      internalState.update(s => ({
        ...s,
        protocolMessages: [...s.protocolMessages, message],
      }));
    },

    clearProtocolMessages() {
      internalState.update(s => ({ ...s, protocolMessages: [] }));
    },

    addTestSuite(suite: TestSuiteData) {
      internalState.update(s => ({
        ...s,
        testSuites: [...s.testSuites, suite],
      }));
    },

    updateTestResult(testId: string, result: TestResultData) {
      internalState.update(s => ({
        ...s,
        testSuites: s.testSuites.map(suite => ({
          ...suite,
          tests: suite.tests.map(test => {
            if (test.id === testId) {
              return { ...test, lastResult: result };
            }
            return test;
          }),
        })),
      }));
      logger.debug('Test result updated:', testId, result.passed ? 'PASS' : 'FAIL');
    },

    addChatMessage(message: ChatMessage) {
      internalState.update(s => ({
        ...s,
        chatHistory: [...s.chatHistory, message],
      }));
    },

    selectMessage(messageId: string | null) {
      internalState.update(s => ({ ...s, selectedMessageId: messageId }));
      logger.debug('Message selected:', messageId);
    },

    selectTest(testId: string | null) {
      internalState.update(s => ({ ...s, selectedTestId: testId }));
      logger.debug('Test selected:', testId);
    },

    selectSuite(suiteId: string | null) {
      internalState.update(s => ({ ...s, selectedSuiteId: suiteId }));
      logger.debug('Suite selected:', suiteId);
    },

    setGeneratingTests(generating: boolean) {
      internalState.update(s => ({ ...s, isGeneratingTests: generating }));
    },

    setRunningTests(running: boolean) {
      internalState.update(s => ({ ...s, isRunningTests: running }));
    },

    setLoadingMessages(loading: boolean) {
      internalState.update(s => ({ ...s, isLoadingMessages: loading }));
    },

    // Getters (non-reactive)
    getCurrentState() {
      return get(internalState);
    },

    getSelectedMessage() {
      return get(selectedMessage);
    },

    getSelectedTest() {
      return get(selectedTest);
    },

    getCoverage() {
      return get(coverage);
    },

    getInsights() {
      return get(insights);
    },

    getSuggestions() {
      return get(suggestions);
    },
  };
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

function computeCategoryStatus(tests: TestData[]): CoverageCell {
  if (tests.length === 0) {
    return { status: 'untested', testIds: [] };
  }

  const results = tests
    .map(t => t.lastResult)
    .filter((r): r is TestResultData => !!r);

  if (results.length === 0) {
    return { status: 'untested', testIds: tests.map(t => t.id) };
  }

  const allPassed = results.every(r => r.passed);
  const status = allPassed ? 'passed' : 'failed';

  return {
    status: status as 'passed' | 'failed',
    testIds: tests.map(t => t.id),
  };
}

// ============================================================================
// EXPORT
// ============================================================================

const store = createWorkspaceStore();

// Export main store
export const workspaceStore = store;

// Export derived stores for easier access
export const workspaceServerContext = store.serverContext;
export const workspaceSelectedMessage = store.selectedMessage;
export const workspaceSelectedTest = store.selectedTest;
export const workspaceAllTests = store.allTests;
export const workspaceCoverage = store.coverage;
export const workspaceInsights = store.insights;
export const workspaceSuggestions = store.suggestions;
