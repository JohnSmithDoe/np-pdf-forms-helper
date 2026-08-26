// ─── why ────────────────────────────────────────────────────────
// The IPC seam, and the only file in the renderer that knows Tauri exists.
// It carries the TRANSPORT and nothing domain-specific: which commands exist is
// each domain's own business, and lives in its `data/` layer beside the types
// those commands speak.
//
// The contract is plain request/response: a command RETURNS its data and
// REJECTS on failure. A rejected promise already carries the failure to the
// exact caller that caused it, so there is no `error$`. Success REPORTS are the
// one thing that still goes out on `report$`, because they are cross-cutting —
// any command of any domain may attach one, and only the page presents them.
//
// A command is a NAMED operation with a NAMED payload, and both names are the
// Rust ones, so nothing translates on the way out.
//
// There is ONE of this service, deliberately. Two would mean two `busy`
// counters and two report streams, and the busy overlay would flicker as
// commands from different domains overlapped.
//
// Three things stay centralised in `call()` because every command needs them:
//   • busy tracking — a COUNTER, so concurrent calls cannot switch the
//     indicator off early; exposed as a boolean signal.
//   • report forwarding — read off the response, not off the command.
//   • error normalisation — Rust serialises `AppError` as `{ messages: [...] }`,
//     a plain object rather than an `Error`, and that is unwrapped once here.
//
// `silent` suppresses the BROADCAST, not the report: the response still carries
// it, and the caller that asked for silence is the one rendering it. That is the
// wizards' result steps, where the report IS the page — left on `report$` it
// would also raise a dialog covering the page that already says the same thing.
// It is opt-in per call rather than a rule about which commands are quiet,
// because the same command is loud on the expert page and quiet in a wizard.
//
// The `isTauri` guard is for `pnpm start`: a plain browser has no Tauri, and a
// German sentence beats whatever the API throws when its globals are missing.
// Anything that is not the serialised `AppError` is a renderer-side fault, not a
// backend answer, so it keeps whatever text it has.
// ────────────────────────────────────────────────────────────────

import { computed, Injectable, Signal, signal } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { Subject } from 'rxjs';
import { ClientReport } from '../../model/client.types';

const UNKNOWN_ERROR = 'Es ist ein unbekannter Fehler aufgetreten.';
const NO_DESKTOP =
  'Die Anwendung läuft nicht in der Desktop-Umgebung. Es können keine Daten geladen oder gespeichert werden.';

export interface BackendRequest {
  command: string;
  payload: Record<string, unknown>;
}

export interface CallOptions {
  silent?: boolean;
}

export interface BackendResponse {
  message?: ClientReport;
}

export class BackendError extends Error {
  readonly messages: string[];

  constructor(messages: string[]) {
    super(messages.join(' '));
    this.name = 'BackendError';
    this.messages = messages;
  }
}

function toBackendError(cause: unknown): BackendError {
  if (cause instanceof BackendError) return cause;
  if (
    typeof cause === 'object' &&
    cause !== null &&
    Array.isArray((cause as { messages?: unknown }).messages)
  ) {
    return new BackendError((cause as { messages: string[] }).messages);
  }
  const message = cause instanceof Error ? cause.message : String(cause);
  return new BackendError([message.trim() || UNKNOWN_ERROR]);
}

function reportOf(result: unknown): ClientReport | undefined {
  if (typeof result !== 'object' || result === null) return undefined;
  return (result as BackendResponse).message ?? undefined;
}

@Injectable({ providedIn: 'root' })
export class BackendService {
  readonly #pending = signal(0);

  readonly busy: Signal<boolean> = computed(() => this.#pending() > 0);

  readonly report$ = new Subject<ClientReport>();

  async #send<T>({ command, payload }: BackendRequest): Promise<T> {
    if (!isTauri()) throw new BackendError([NO_DESKTOP]);
    return await invoke<T>(command, payload);
  }

  async call<T extends BackendResponse>(
    request: BackendRequest,
    { silent = false }: CallOptions = {}
  ): Promise<T> {
    this.#pending.update((count) => count + 1);
    try {
      const result = await this.#send<T>(request);
      const report = reportOf(result);
      if (report && !silent) this.report$.next(report);
      return result;
    } catch (cause) {
      throw toBackendError(cause);
    } finally {
      this.#pending.update((count) => count - 1);
    }
  }
}
