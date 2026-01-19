<!--
  Notification Panel Component
  Displays recent notifications with mark as read functionality
-->
<script lang="ts">
  import { notificationStore, unreadCount, type Notification, type NotificationType } from '$lib/stores/notificationStore';
  import {
    Bell,
    X,
    CheckCircle,
    AlertCircle,
    AlertTriangle,
    Info,
    Check,
    Trash2
  } from 'lucide-svelte';

  // Reactive state
  const notifications = $derived($notificationStore.notifications);
  const panelOpen = $derived($notificationStore.panelOpen);
  const unread = $derived($unreadCount);

  function togglePanel() {
    notificationStore.togglePanel();
  }

  function closePanel() {
    notificationStore.closePanel();
  }

  function markAsRead(id: string) {
    notificationStore.markAsRead(id);
  }

  function markAllAsRead() {
    notificationStore.markAllAsRead();
  }

  function removeNotification(id: string) {
    notificationStore.remove(id);
  }

  function clearAll() {
    notificationStore.clear();
  }

  function getIcon(type: NotificationType) {
    switch (type) {
      case 'success': return CheckCircle;
      case 'error': return AlertCircle;
      case 'warning': return AlertTriangle;
      case 'info': return Info;
    }
  }

  function getIconColor(type: NotificationType) {
    switch (type) {
      case 'success': return 'text-green-500';
      case 'error': return 'text-red-500';
      case 'warning': return 'text-yellow-500';
      case 'info': return 'text-blue-500';
    }
  }

  function formatTime(date: Date): string {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (seconds < 60) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    return date.toLocaleDateString();
  }

  // Close panel when clicking outside
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.notification-panel-container')) {
      closePanel();
    }
  }
</script>

<svelte:window onclick={panelOpen ? handleClickOutside : undefined} />

<div class="notification-panel-container relative">
  <!-- Bell Button -->
  <button
    class="notification-bell"
    onclick={(e) => { e.stopPropagation(); togglePanel(); }}
    aria-label="Notifications"
    title={unread > 0 ? `${unread} unread notifications` : 'Notifications'}
  >
    <Bell size={18} />
    {#if unread > 0}
      <span class="notification-badge">{unread > 9 ? '9+' : unread}</span>
    {/if}
  </button>

  <!-- Panel Dropdown -->
  {#if panelOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="notification-panel" onclick={(e) => e.stopPropagation()}>
      <!-- Header -->
      <div class="notification-panel-header">
        <h3 class="notification-panel-title">Notifications</h3>
        <div class="notification-panel-actions">
          {#if notifications.length > 0}
            <button
              onclick={markAllAsRead}
              class="notification-action-btn"
              title="Mark all as read"
            >
              <Check size={14} />
            </button>
            <button
              onclick={clearAll}
              class="notification-action-btn"
              title="Clear all"
            >
              <Trash2 size={14} />
            </button>
          {/if}
          <button
            onclick={closePanel}
            class="notification-action-btn"
            title="Close"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      <!-- Notifications List -->
      <div class="notification-list">
        {#if notifications.length === 0}
          <div class="notification-empty">
            <Bell size={32} class="text-gray-300 dark:text-gray-600 mb-2" />
            <p class="text-sm text-secondary">No notifications</p>
            <p class="text-xs text-tertiary">You're all caught up!</p>
          </div>
        {:else}
          {#each notifications as notification (notification.id)}
            {@const IconComponent = getIcon(notification.type)}
            <div
              class="notification-item"
              class:unread={!notification.read}
              onclick={() => markAsRead(notification.id)}
            >
              <div class="notification-icon {getIconColor(notification.type)}">
                <IconComponent size={16} />
              </div>
              <div class="notification-content">
                <div class="notification-header">
                  <span class="notification-title">{notification.title}</span>
                  <span class="notification-time">{formatTime(notification.timestamp)}</span>
                </div>
                <p class="notification-message">{notification.message}</p>
                {#if notification.source}
                  <span class="notification-source">{notification.source}</span>
                {/if}
              </div>
              <button
                onclick={(e) => { e.stopPropagation(); removeNotification(notification.id); }}
                class="notification-remove"
                title="Dismiss"
              >
                <X size={12} />
              </button>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer -->
      {#if notifications.length > 0}
        <div class="notification-panel-footer">
          <span class="text-xs text-secondary">
            {notifications.length} notification{notifications.length !== 1 ? 's' : ''}
            {#if unread > 0}
              • {unread} unread
            {/if}
          </span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .notification-panel-container {
    position: relative;
  }

  .notification-bell {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
    position: relative;
  }

  .notification-bell:hover {
    background: var(--mcp-surface-secondary);
    color: var(--mcp-text-primary);
  }

  .notification-badge {
    position: absolute;
    top: 4px;
    right: 4px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    background: var(--mcp-error-500);
    color: white;
    font-size: 10px;
    font-weight: 600;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .notification-panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 360px;
    max-height: 480px;
    background: var(--mcp-surface-elevated);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-lg);
    z-index: 50;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .notification-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-3) var(--mcp-space-4);
    border-bottom: 1px solid var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
  }

  .notification-panel-title {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0;
  }

  .notification-panel-actions {
    display: flex;
    gap: var(--mcp-space-1);
  }

  .notification-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: var(--mcp-radius-sm);
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .notification-action-btn:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary);
  }

  .notification-list {
    flex: 1;
    overflow-y: auto;
    max-height: 360px;
  }

  .notification-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--mcp-space-8);
    text-align: center;
  }

  .notification-item {
    display: flex;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3) var(--mcp-space-4);
    border-bottom: 1px solid var(--mcp-border-primary);
    cursor: pointer;
    transition: background var(--mcp-transition-fast);
    position: relative;
  }

  .notification-item:hover {
    background: var(--mcp-surface-secondary);
  }

  .notification-item:last-child {
    border-bottom: none;
  }

  .notification-item.unread {
    background: var(--mcp-primary-50);
  }

  [data-theme="dark"] .notification-item.unread {
    background: rgba(59, 130, 246, 0.1);
  }

  .notification-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--mcp-radius-full);
    background: var(--mcp-surface-tertiary);
  }

  .notification-content {
    flex: 1;
    min-width: 0;
  }

  .notification-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--mcp-space-2);
    margin-bottom: 2px;
  }

  .notification-title {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .notification-time {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    white-space: nowrap;
  }

  .notification-message {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary);
    margin: 0;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .notification-source {
    display: inline-block;
    font-size: 10px;
    color: var(--mcp-text-tertiary);
    background: var(--mcp-surface-tertiary);
    padding: 1px 6px;
    border-radius: var(--mcp-radius-sm);
    margin-top: 4px;
  }

  .notification-remove {
    position: absolute;
    top: var(--mcp-space-2);
    right: var(--mcp-space-2);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
    border-radius: var(--mcp-radius-sm);
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    opacity: 0;
    transition: all var(--mcp-transition-fast);
  }

  .notification-item:hover .notification-remove {
    opacity: 1;
  }

  .notification-remove:hover {
    background: var(--mcp-error-100);
    color: var(--mcp-error-600);
  }

  .notification-panel-footer {
    padding: var(--mcp-space-2) var(--mcp-space-4);
    border-top: 1px solid var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
    text-align: center;
  }

  /* Scrollbar styling */
  .notification-list::-webkit-scrollbar {
    width: 4px;
  }

  .notification-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .notification-list::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 2px;
  }

  /* Mobile adjustments */
  @media (max-width: 767px) {
    .notification-panel {
      position: fixed;
      top: 60px;
      right: 8px;
      left: 8px;
      width: auto;
      max-height: calc(100vh - 80px);
    }
  }
</style>
