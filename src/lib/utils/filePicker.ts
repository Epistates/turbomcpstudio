/**
 * File Picker Utilities
 *
 * Wrapper around Tauri's file dialog with modal z-index workaround.
 * When opening file dialogs from within modals, we need to temporarily hide
 * the modal to prevent z-index interference.
 */

import { open, type OpenDialogOptions } from '@tauri-apps/plugin-dialog';
import { TIMEOUTS } from '$lib/constants';

/**
 * Pick a file or directory with automatic modal suppression
 *
 * @param options - Tauri open dialog options
 * @param modalState - Optional reactive state to temporarily suppress
 * @returns Selected path(s) or null if cancelled
 */
export async function pickFile(
  options: OpenDialogOptions,
  modalState?: { get: () => boolean; set: (value: boolean) => void }
): Promise<string | string[] | null> {
  let originalModalState: boolean | undefined;

  try {
    // Temporarily hide modal if state provided
    if (modalState) {
      originalModalState = modalState.get();
      modalState.set(false);

      // Small delay to ensure modal is visually hidden
      await new Promise((resolve) => setTimeout(resolve, TIMEOUTS.MODAL_HIDE));
    }

    // Open file dialog
    const result = await open(options);

    return result;
  } catch (error) {
    console.error('File picker error:', error);
    throw error;
  } finally {
    // Restore modal state
    if (modalState && originalModalState !== undefined) {
      // Small delay before reshowing modal
      await new Promise((resolve) => setTimeout(resolve, TIMEOUTS.MODAL_RESTORE));
      modalState.set(originalModalState);
    }
  }
}

/**
 * Pick a directory
 *
 * @param title - Dialog title
 * @param modalState - Optional modal state for suppression
 * @returns Selected directory path or null
 */
export async function pickDirectory(
  title: string = 'Select Directory',
  modalState?: { get: () => boolean; set: (value: boolean) => void }
): Promise<string | null> {
  const result = await pickFile(
    {
      directory: true,
      multiple: false,
      title
    },
    modalState
  );

  return result as string | null;
}

/**
 * Pick an executable file
 *
 * @param title - Dialog title
 * @param modalState - Optional modal state for suppression
 * @returns Selected file path or null
 */
export async function pickExecutable(
  title: string = 'Select Executable',
  modalState?: { get: () => boolean; set: (value: boolean) => void }
): Promise<string | null> {
  const result = await pickFile(
    {
      directory: false,
      multiple: false,
      title
    },
    modalState
  );

  return result as string | null;
}

/**
 * Pick multiple files
 *
 * @param title - Dialog title
 * @param filters - File type filters
 * @param modalState - Optional modal state for suppression
 * @returns Selected file paths or null
 */
export async function pickFiles(
  title: string = 'Select Files',
  filters?: Array<{ name: string; extensions: string[] }>,
  modalState?: { get: () => boolean; set: (value: boolean) => void }
): Promise<string[] | null> {
  const result = await pickFile(
    {
      directory: false,
      multiple: true,
      title,
      filters
    },
    modalState
  );

  return result as string[] | null;
}
