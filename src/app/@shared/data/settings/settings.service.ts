// ─── why ────────────────────────────────────────────────────────
// UI preferences, in localStorage and nowhere else. The rule this establishes:
// a UI preference lives here, the user's DATA lives in the backend.
//
// Synchronous is the requirement, not a shortcut. `/documents` redirects to the
// expert page or the wizard depending on `viewMode`, and a `RedirectFunction`
// resolves during `checkGuards` — before any resolver has run and before an app
// initializer would have helped. `ionic-storage` (np-commlink's choice) is
// IndexedDB-backed and async, so it cannot answer in that phase; a `settings.db`
// behind a Tauri command would cost a Rust module and an IPC round trip
// blocking the first navigation. Both to hold one enum.
//
// Every read and write is wrapped: a webview with storage disabled THROWS on
// access rather than returning null, and a preference is never worth taking the
// app down for. An unreadable or unknown value falls back to the default, which
// is `wizard` — whoever has not chosen yet is not the expert.
//
// The signal is the source components read; localStorage is written through on
// change. It is not re-read after construction, because this app is one window
// and nothing else writes the key.
//
// There is no `toggle`. Both switch sites name the mode they are going TO,
// because each also navigates to that mode's landing route — a toggle would
// return the new mode and leave the caller to map it back to a URL.
//
// `fullEnabled` (`npdh.full`) is a feature toggle, not a preference: the
// dashboard's Vorlagen and Einstellungen tiles and „Export erstellen“ wait
// behind it. It has no setter on purpose — the e2e and the developer set the
// key by hand. The import and the master import are no longer switched.
// ────────────────────────────────────────────────────────────────

import { Injectable, signal } from '@angular/core';
import type { ViewMode } from '../../model/settings.types';

const VIEW_MODE_KEY = 'npdh.viewMode';
const DEFAULT_VIEW_MODE: ViewMode = 'wizard';
const FULL_KEY = 'npdh.full';

function isViewMode(value: unknown): value is ViewMode {
  return value === 'expert' || value === 'wizard';
}

function read(key: string): ReturnType<Storage['getItem']> | undefined {
  try {
    return localStorage.getItem(key);
  } catch {
    return;
  }
}

function write(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

@Injectable({ providedIn: 'root' })
export class SettingsService {
  readonly #viewMode = signal<ViewMode>(readViewMode());

  readonly viewMode = this.#viewMode.asReadonly();

  readonly fullEnabled = read(FULL_KEY) === 'on';

  setViewMode(mode: ViewMode): void {
    this.#viewMode.set(mode);
    write(VIEW_MODE_KEY, mode);
  }
}

function readViewMode(): ViewMode {
  const stored = read(VIEW_MODE_KEY);
  return isViewMode(stored) ? stored : DEFAULT_VIEW_MODE;
}
