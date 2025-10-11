import { writable } from 'svelte/store';

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  source: string;
  message: string;
  details?: any;
}

interface LogStoreState {
  logs: LogEntry[];
  maxLogs: number;
  filters: {
    levels: Set<LogLevel>;
    sources: Set<string>;
    searchQuery: string;
  };
}

const initialState: LogStoreState = {
  logs: [],
  maxLogs: 500, // Keep last 500 logs
  filters: {
    levels: new Set(['debug', 'info', 'warn', 'error']),
    sources: new Set(),
    searchQuery: '',
  },
};

function createLogStore() {
  const { subscribe, set, update } = writable<LogStoreState>(initialState);

  return {
    subscribe,

    // Add a log entry
    log(level: LogLevel, source: string, message: string, details?: any) {
      const entry: LogEntry = {
        id: crypto.randomUUID(),
        timestamp: new Date().toISOString(),
        level,
        source,
        message,
        details,
      };

      update((state) => {
        const logs = [entry, ...state.logs].slice(0, state.maxLogs);
        return { ...state, logs };
      });
    },

    // Convenience methods
    debug(source: string, message: string, details?: any) {
      this.log('debug', source, message, details);
    },

    info(source: string, message: string, details?: any) {
      this.log('info', source, message, details);
    },

    warn(source: string, message: string, details?: any) {
      this.log('warn', source, message, details);
    },

    error(source: string, message: string, details?: any) {
      this.log('error', source, message, details);
    },

    // Clear all logs
    clear() {
      update((state) => ({ ...state, logs: [] }));
    },

    // Set log level filter
    setLevelFilter(levels: Set<LogLevel>) {
      update((state) => ({
        ...state,
        filters: { ...state.filters, levels },
      }));
    },

    // Set source filter
    setSourceFilter(sources: Set<string>) {
      update((state) => ({
        ...state,
        filters: { ...state.filters, sources },
      }));
    },

    // Set search query
    setSearchQuery(searchQuery: string) {
      update((state) => ({
        ...state,
        filters: { ...state.filters, searchQuery },
      }));
    },

    // Get filtered logs
    getFilteredLogs(state: LogStoreState): LogEntry[] {
      return state.logs.filter((log) => {
        // Level filter
        if (!state.filters.levels.has(log.level)) {
          return false;
        }

        // Source filter (if any sources selected)
        if (state.filters.sources.size > 0 && !state.filters.sources.has(log.source)) {
          return false;
        }

        // Search query
        if (state.filters.searchQuery) {
          const query = state.filters.searchQuery.toLowerCase();
          return (
            log.message.toLowerCase().includes(query) ||
            log.source.toLowerCase().includes(query) ||
            JSON.stringify(log.details).toLowerCase().includes(query)
          );
        }

        return true;
      });
    },

    // Export logs as JSON
    exportLogs(state: LogStoreState): string {
      return JSON.stringify(state.logs, null, 2);
    },
  };
}

export const logStore = createLogStore();

// Hook into console methods to capture logs
if (typeof window !== 'undefined') {
  const originalConsole = {
    log: console.log,
    info: console.info,
    warn: console.warn,
    error: console.error,
    debug: console.debug,
  };

  console.log = (...args: any[]) => {
    originalConsole.log(...args);
    logStore.info('console', args.map(String).join(' '));
  };

  console.info = (...args: any[]) => {
    originalConsole.info(...args);
    logStore.info('console', args.map(String).join(' '));
  };

  console.warn = (...args: any[]) => {
    originalConsole.warn(...args);
    logStore.warn('console', args.map(String).join(' '));
  };

  console.error = (...args: any[]) => {
    originalConsole.error(...args);
    logStore.error('console', args.map(String).join(' '));
  };

  console.debug = (...args: any[]) => {
    originalConsole.debug(...args);
    logStore.debug('console', args.map(String).join(' '));
  };
}
