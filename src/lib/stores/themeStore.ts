/**
 * MCP Studio Theme Management Store
 * Enterprise-grade theme system with dark/light mode support
 */

import { writable } from 'svelte/store';
import { browser } from '$app/environment';

// Theme types
export type Theme = 'light' | 'dark' | 'system';

interface ThemeState {
  theme: Theme;
  resolvedTheme: 'light' | 'dark';
  isSystemTheme: boolean;
}

// Default theme state
const defaultThemeState: ThemeState = {
  theme: 'system',
  resolvedTheme: 'light',
  isSystemTheme: true
};

// Create the store
function createThemeStore() {
  const { subscribe, set, update } = writable<ThemeState>(defaultThemeState);
  let initialized = false;

  // Get system preference
  const getSystemTheme = (): 'light' | 'dark' => {
    if (!browser) return 'light';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  };

  // Resolve theme based on current setting
  const resolveTheme = (theme: Theme): 'light' | 'dark' => {
    if (theme === 'system') {
      return getSystemTheme();
    }
    return theme;
  };

  // Apply theme to DOM
  const applyTheme = (resolvedTheme: 'light' | 'dark') => {
    if (!browser) return;

    document.documentElement.setAttribute('data-theme', resolvedTheme);
    document.documentElement.classList.remove('light', 'dark');
    document.documentElement.classList.add(resolvedTheme);
  };

  // Load theme from localStorage
  const loadTheme = () => {
    if (!browser) return;
    
    const stored = localStorage.getItem('mcp-studio-theme') as Theme;
    const theme = stored && ['light', 'dark', 'system'].includes(stored) ? stored : 'system';
    const resolvedTheme = resolveTheme(theme);
    
    set({
      theme,
      resolvedTheme,
      isSystemTheme: theme === 'system'
    });
    
    applyTheme(resolvedTheme);
  };

  // Save theme to localStorage
  const saveTheme = (theme: Theme) => {
    if (!browser) return;
    localStorage.setItem('mcp-studio-theme', theme);
  };

  // Set theme
  const setTheme = (newTheme: Theme) => {
    const resolvedTheme = resolveTheme(newTheme);
    
    update(() => ({
      theme: newTheme,
      resolvedTheme,
      isSystemTheme: newTheme === 'system'
    }));
    
    applyTheme(resolvedTheme);
    saveTheme(newTheme);
  };

  // Toggle between light and dark
  const toggleTheme = () => {
    update((state) => {
      const newTheme = state.resolvedTheme === 'light' ? 'dark' : 'light';
      const resolvedTheme = newTheme;
      
      applyTheme(resolvedTheme);
      saveTheme(newTheme);
      
      return {
        theme: newTheme,
        resolvedTheme,
        isSystemTheme: false
      };
    });
  };

  // Listen for system theme changes
  const initSystemListener = () => {
    if (!browser) return;
    
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      update((state) => {
        if (state.isSystemTheme) {
          const resolvedTheme = getSystemTheme();
          applyTheme(resolvedTheme);
          return { ...state, resolvedTheme };
        }
        return state;
      });
    };
    
    // Modern browsers
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', handleChange);
    } else {
      // Fallback for older browsers
      mediaQuery.addListener(handleChange);
    }
  };

  // Initialize theme system
  const init = () => {
    if (initialized) {
      return;
    }
    initialized = true;
    loadTheme();
    initSystemListener();
  };

  return {
    subscribe,
    setTheme,
    toggleTheme,
    init
  };
}

// Export the store
export const themeStore = createThemeStore();

// Theme utilities for components
export const getThemeClass = (theme: 'light' | 'dark', lightClass: string, darkClass: string) => {
  return theme === 'dark' ? darkClass : lightClass;
};

export const getThemeIcon = (theme: Theme) => {
  switch (theme) {
    case 'light':
      return 'sun';
    case 'dark':
      return 'moon';
    case 'system':
      return 'monitor';
    default:
      return 'monitor';
  }
};

// CSS-in-JS theme values (for dynamic styling)
export const getThemeColors = (theme: 'light' | 'dark') => {
  if (theme === 'dark') {
    return {
      primary: '#0ea5e9',
      surface: '#1f2937',
      surfaceSecondary: '#374151',
      text: '#f9fafb',
      textSecondary: '#d1d5db',
      border: '#4b5563'
    };
  }
  
  return {
    primary: '#0284c7',
    surface: '#ffffff',
    surfaceSecondary: '#f9fafb',
    text: '#111827',
    textSecondary: '#6b7280',
    border: '#e5e7eb'
  };
};

// Theme detection hook for SSR-compatible initialization
export const useTheme = () => {
  if (browser) {
    themeStore.init();
  }
  
  return themeStore;
};