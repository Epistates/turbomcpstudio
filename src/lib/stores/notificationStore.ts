/**
 * Notification Store
 * Persistent notification management for the notification panel
 */

import { writable, derived } from 'svelte/store';

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
  source?: string;
  action?: {
    label: string;
    handler: () => void;
  };
}

interface NotificationStoreState {
  notifications: Notification[];
  maxNotifications: number;
  panelOpen: boolean;
}

const initialState: NotificationStoreState = {
  notifications: [],
  maxNotifications: 50,
  panelOpen: false,
};

function createNotificationStore() {
  const { subscribe, set, update } = writable<NotificationStoreState>(initialState);

  return {
    subscribe,

    // Add a notification
    add(
      type: NotificationType,
      title: string,
      message: string,
      options?: { source?: string; action?: Notification['action'] }
    ) {
      const notification: Notification = {
        id: crypto.randomUUID(),
        type,
        title,
        message,
        timestamp: new Date(),
        read: false,
        source: options?.source,
        action: options?.action,
      };

      update((state) => {
        const notifications = [notification, ...state.notifications].slice(0, state.maxNotifications);
        return { ...state, notifications };
      });

      return notification.id;
    },

    // Convenience methods
    success(title: string, message: string, options?: { source?: string }) {
      return this.add('success', title, message, options);
    },

    error(title: string, message: string, options?: { source?: string }) {
      return this.add('error', title, message, options);
    },

    warning(title: string, message: string, options?: { source?: string }) {
      return this.add('warning', title, message, options);
    },

    info(title: string, message: string, options?: { source?: string }) {
      return this.add('info', title, message, options);
    },

    // Mark notification as read
    markAsRead(id: string) {
      update((state) => ({
        ...state,
        notifications: state.notifications.map((n) =>
          n.id === id ? { ...n, read: true } : n
        ),
      }));
    },

    // Mark all as read
    markAllAsRead() {
      update((state) => ({
        ...state,
        notifications: state.notifications.map((n) => ({ ...n, read: true })),
      }));
    },

    // Remove a notification
    remove(id: string) {
      update((state) => ({
        ...state,
        notifications: state.notifications.filter((n) => n.id !== id),
      }));
    },

    // Clear all notifications
    clear() {
      update((state) => ({ ...state, notifications: [] }));
    },

    // Toggle panel
    togglePanel() {
      update((state) => ({ ...state, panelOpen: !state.panelOpen }));
    },

    // Open panel
    openPanel() {
      update((state) => ({ ...state, panelOpen: true }));
    },

    // Close panel
    closePanel() {
      update((state) => ({ ...state, panelOpen: false }));
    },
  };
}

export const notificationStore = createNotificationStore();

// Derived store for unread count
export const unreadCount = derived(
  notificationStore,
  ($store) => $store.notifications.filter((n) => !n.read).length
);

// Derived store for recent notifications (last 5)
export const recentNotifications = derived(
  notificationStore,
  ($store) => $store.notifications.slice(0, 5)
);
