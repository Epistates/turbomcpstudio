/**
 * Global Search Store
 *
 * Manages search state across the application:
 * - Global search (header): searches across servers, tools, resources
 * - Component searches: local filtering within each view
 * - Search results: categorized, scored, and ranked
 * - Navigation: deep links to search results
 */

import { writable, derived } from 'svelte/store';
import { serverStore } from './serverStore';

export interface SearchResult {
  id: string;
  type: 'server' | 'tool' | 'resource' | 'prompt' | 'collection';
  title: string;
  description?: string;
  category: string;
  score: number; // Relevance score 0-100
  metadata?: Record<string, any>;
}

interface SearchState {
  globalQuery: string;
  isGlobalSearchOpen: boolean;
  results: SearchResult[];
  selectedIndex: number;
}

function createSearchStore() {
  const { subscribe, set, update } = writable<SearchState>({
    globalQuery: '',
    isGlobalSearchOpen: false,
    results: [],
    selectedIndex: 0
  });

  // Smart category suggestions based on query
  function getSuggestedCategories(query: string): SearchResult[] {
    const lowerQuery = query.toLowerCase();
    const suggestions: SearchResult[] = [];

    // Suggest navigation to relevant views even if no exact matches
    if (lowerQuery.includes('tool') || lowerQuery.includes('function') || lowerQuery.includes('call')) {
      suggestions.push({
        id: 'nav-tools',
        type: 'tool',
        title: 'Browse Tools',
        description: 'View all available MCP tools',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true }
      });
    }

    if (lowerQuery.includes('resource') || lowerQuery.includes('file') || lowerQuery.includes('data')) {
      suggestions.push({
        id: 'nav-resources',
        type: 'resource',
        title: 'Browse Resources',
        description: 'View all available MCP resources',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true }
      });
    }

    if (lowerQuery.includes('prompt') || lowerQuery.includes('template')) {
      suggestions.push({
        id: 'nav-prompts',
        type: 'prompt',
        title: 'Browse Prompts',
        description: 'View all available MCP prompts',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true }
      });
    }

    if (lowerQuery.includes('server') || lowerQuery.includes('connect')) {
      suggestions.push({
        id: 'nav-servers',
        type: 'server',
        title: 'Browse Servers',
        description: 'View all MCP servers',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true }
      });
    }

    if (lowerQuery.includes('collect') || lowerQuery.includes('workflow')) {
      suggestions.push({
        id: 'nav-collections',
        type: 'collection',
        title: 'Browse Collections',
        description: 'View all saved workflows and collections',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true }
      });
    }

    if (lowerQuery.includes('protocol') || lowerQuery.includes('message') || lowerQuery.includes('debug')) {
      suggestions.push({
        id: 'nav-protocol',
        type: 'server',
        title: 'Protocol Inspector',
        description: 'View MCP protocol messages',
        category: 'Navigation',
        score: 50,
        metadata: { isNavSuggestion: true, navigateTo: 'protocol' }
      });
    }

    return suggestions;
  }

  // Search across all data sources
  async function performGlobalSearch(query: string): Promise<SearchResult[]> {
    if (!query || query.length < 2) return [];

    const results: SearchResult[] = [];
    const lowerQuery = query.toLowerCase();

    // Get current server data from serverStore
    let servers: any[] = [];
    const unsubscribe = serverStore.subscribe(state => {
      // ✅ FIXED: Convert Map to array
      servers = state.servers instanceof Map
        ? Array.from(state.servers.values())
        : [];
    });
    unsubscribe();

    // Search servers
    servers.forEach(server => {
      const nameMatch = server.name?.toLowerCase().includes(lowerQuery);
      const descMatch = server.description?.toLowerCase().includes(lowerQuery);

      if (nameMatch || descMatch) {
        results.push({
          id: server.id,
          type: 'server',
          title: server.name || 'Unnamed Server',
          description: server.description,
          category: 'Servers',
          score: nameMatch ? 90 : 70,
          metadata: { serverId: server.id, status: server.status }
        });
      }

      // Search tools within this server
      if (server.capabilities?.tools) {
        server.capabilities.tools.forEach((tool: any) => {
          const toolNameMatch = tool.name?.toLowerCase().includes(lowerQuery);
          const toolDescMatch = tool.description?.toLowerCase().includes(lowerQuery);

          if (toolNameMatch || toolDescMatch) {
            results.push({
              id: `${server.id}-tool-${tool.name}`,
              type: 'tool',
              title: tool.name,
              description: tool.description || `Tool from ${server.name}`,
              category: 'Tools',
              score: toolNameMatch ? 85 : 65,
              metadata: { serverId: server.id, serverName: server.name, tool }
            });
          }
        });
      }

      // Search resources within this server
      if (server.capabilities?.resources) {
        server.capabilities.resources.forEach((resource: any) => {
          const resNameMatch = resource.name?.toLowerCase().includes(lowerQuery);
          const resUriMatch = resource.uri?.toLowerCase().includes(lowerQuery);

          if (resNameMatch || resUriMatch) {
            results.push({
              id: `${server.id}-resource-${resource.uri}`,
              type: 'resource',
              title: resource.name || resource.uri,
              description: resource.description || `Resource from ${server.name}`,
              category: 'Resources',
              score: resNameMatch ? 85 : 65,
              metadata: { serverId: server.id, serverName: server.name, resource }
            });
          }
        });
      }

      // Search prompts within this server
      if (server.capabilities?.prompts) {
        server.capabilities.prompts.forEach((prompt: any) => {
          const promptNameMatch = prompt.name?.toLowerCase().includes(lowerQuery);
          const promptDescMatch = prompt.description?.toLowerCase().includes(lowerQuery);

          if (promptNameMatch || promptDescMatch) {
            results.push({
              id: `${server.id}-prompt-${prompt.name}`,
              type: 'prompt',
              title: prompt.name,
              description: prompt.description || `Prompt from ${server.name}`,
              category: 'Prompts',
              score: promptNameMatch ? 85 : 65,
              metadata: { serverId: server.id, serverName: server.name, prompt }
            });
          }
        });
      }
    });

    // Add navigation suggestions if no exact results or few results
    const suggestions = getSuggestedCategories(query);
    const allResults = [...results, ...suggestions];

    // Sort by score (highest first), but keep exact matches before suggestions
    return allResults.sort((a, b) => b.score - a.score);
  }

  return {
    subscribe,

    // Open/close global search
    openGlobalSearch: () => update(s => ({ ...s, isGlobalSearchOpen: true })),
    closeGlobalSearch: () => update(s => ({ ...s, isGlobalSearchOpen: false, globalQuery: '', results: [], selectedIndex: 0 })),

    // Update search query and perform search
    setGlobalQuery: async (query: string) => {
      update(s => ({ ...s, globalQuery: query }));
      const results = await performGlobalSearch(query);
      update(s => ({ ...s, results, selectedIndex: 0 }));
    },

    // Navigate results with keyboard
    selectNext: () => update(s => ({
      ...s,
      selectedIndex: Math.min(s.selectedIndex + 1, s.results.length - 1)
    })),

    selectPrevious: () => update(s => ({
      ...s,
      selectedIndex: Math.max(s.selectedIndex - 1, 0)
    })),

    // Get selected result
    getSelectedResult: (): SearchResult | null => {
      let result: SearchResult | null = null;
      subscribe(s => {
        result = s.results[s.selectedIndex] || null;
      })();
      return result;
    },

    // Clear all
    reset: () => set({
      globalQuery: '',
      isGlobalSearchOpen: false,
      results: [],
      selectedIndex: 0
    })
  };
}

export const searchStore = createSearchStore();

// Derived store for categorized results
export const categorizedResults = derived(
  searchStore,
  $search => {
    const categories: Record<string, SearchResult[]> = {};

    $search.results.forEach(result => {
      if (!categories[result.category]) {
        categories[result.category] = [];
      }
      categories[result.category].push(result);
    });

    return categories;
  }
);
