/**
 * MCP Studio - Application Constants
 *
 * Centralized timing, polling, and configuration constants.
 * Organized by category for easy discovery and maintenance.
 */

// ========================================
// POLLING INTERVALS (milliseconds)
// ========================================

/**
 * How often to poll for various data updates
 * All values in milliseconds
 */
export const POLLING = {
	/** How often to check for new sampling requests (5 seconds) */
	SAMPLING_REQUESTS: 5_000,

	/** How often to refresh LLM provider list (30 seconds) */
	LLM_PROVIDERS: 30_000,

	/** How often to refresh protocol inspector messages (2 seconds) */
	PROTOCOL_MESSAGES: 2_000
} as const;

// ========================================
// UI TIMEOUTS & DELAYS (milliseconds)
// ========================================

/**
 * Timeouts and delays for UI operations
 * All values in milliseconds
 */
export const TIMEOUTS = {
	/** Fallback timeout for app initialization (1 second) */
	APP_INIT_FALLBACK: 1_000,

	/** Delay before forcing UI repaint (100ms) */
	REPAINT_DELAY: 100,

	/** Delay before hiding modal for file picker (100ms) */
	MODAL_HIDE: 100,

	/** Delay before restoring modal after file picker (50ms) */
	MODAL_RESTORE: 50,

	/** Delay before marking notifications as read (500ms) */
	NOTIFICATION_MARK_READ: 500,

	/** Short delay for UI updates (10ms) */
	UI_UPDATE_SHORT: 10,

	/** Toast notification duration (2 seconds) */
	TOAST_DURATION: 2_000
} as const;

// ========================================
// RETRY CONFIGURATION
// ========================================

/**
 * Configuration for retry logic with exponential backoff
 */
export const RETRY = {
	/** Maximum retry attempts for failed operations */
	MAX_ATTEMPTS: 3,

	/** Initial delay for exponential backoff (1 second) */
	INITIAL_DELAY: 1_000,

	/** Jitter percentage (0.0-1.0) to prevent thundering herd */
	JITTER_PERCENT: 0.1
} as const;

// ========================================
// TYPE EXPORTS
// ========================================

/** Type-safe access to polling interval keys */
export type PollingKey = keyof typeof POLLING;

/** Type-safe access to timeout keys */
export type TimeoutKey = keyof typeof TIMEOUTS;

/** Type-safe access to retry config keys */
export type RetryKey = keyof typeof RETRY;
