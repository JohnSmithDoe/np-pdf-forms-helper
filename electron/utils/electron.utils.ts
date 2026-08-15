import { BrowserWindow, dialog, OpenDialogOptions } from 'electron';

export function showFilePickerSync(
  mainWindow: BrowserWindow,
  options: Omit<OpenDialogOptions, 'properties' | 'securityScopedBookmarks'>
): string | null {
  let result = null;
  if (mainWindow) {
    const filename = dialog.showOpenDialogSync(mainWindow, {
      ...options,
      properties: ['openFile'],
    });
    result = filename?.pop() || null;
  }
  return result;
}
export function showFolderPickerSync(
  mainWindow: BrowserWindow,
  options: Omit<OpenDialogOptions, 'properties' | 'securityScopedBookmarks'>
): string | null {
  let result = null;
  if (mainWindow) {
    const filename = dialog.showOpenDialogSync(mainWindow, {
      ...options,
      properties: ['openDirectory'],
    });
    result = filename?.pop() || null;
  }
  return result;
}
