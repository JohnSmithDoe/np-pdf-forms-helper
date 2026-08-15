import * as child_process from 'child_process';

/**
 * cmd.exe's internal start command by default interprets a "..."-enclosed 1st argument as the window
 * title for the new console window to create (which doesn't apply here).
 * By supplying a (dummy) window title * - "" - explicitly,
 * the 2nd argument is reliably interpreted as the target executable / document path.
 */
export function startWithExpolorer(filename: string): false {
  child_process.exec(`start "" "${filename}"`);
  return false;
}
