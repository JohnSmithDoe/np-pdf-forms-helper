// ─── why ────────────────────────────────────────────────────────
// What this program is, for the person who has to approve it.
//
// It exists because the version used to reach the screen only on the welcome
// toast, and that toast is gone — the load moved into a route resolver, which
// runs before any page is listening on `report$`. But a page beat a
// `ReplaySubject` on the merits anyway: "unknown software" on a managed desktop
// has to be signed off, and the questions are always the same — what does it
// write, where, and does it talk to anything. A toast that says "Version 1.1.9"
// answers none of them.
//
// The paths come from the BACKEND rather than being written here, because
// `AppConfig` resolves them against the WORKING directory: `tauri dev` puts
// `data/` under `src-tauri/`, an installed build puts it beside the executable.
// A path documented in the template would be wrong in one of those two.
//
// The claims below are checkable in this repo and are the reason they are worth
// stating: there is no network client in `src-tauri/Cargo.toml`, no external
// binary since pdftk went, and every write lands under the folders named here.
// If that stops being true, this page is what has to change with it.
//
// It loads in the constructor and holds the failure rather than throwing one: an
// info page that cannot reach the backend should still say what it can, and
// `pnpm start` outside Tauri is exactly that case.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import {
  IonButtons,
  IonContent,
  IonHeader,
  IonItem,
  IonLabel,
  IonList,
  IonMenuButton,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { APP_WORDMARK } from '../../../@shared/model/app.consts';
import { InfoBackend } from '../../data/info.backend';
import type { AppInfo } from '../../model/info.types';

@Component({
  selector: 'app-page-info',
  templateUrl: 'info.page.html',
  styleUrls: ['info.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonButtons,
    IonContent,
    IonHeader,
    IonItem,
    IonLabel,
    IonList,
    IonMenuButton,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class InfoPage {
  readonly #backend = inject(InfoBackend);

  protected readonly wordmark = APP_WORDMARK;
  protected readonly info = signal<AppInfo | undefined>(undefined);
  protected readonly unavailable = signal<string | undefined>(undefined);

  constructor() {
    void this.#load();
  }

  async #load(): Promise<void> {
    try {
      this.info.set(await this.#backend.appInfo());
    } catch (error) {
      if (!(error instanceof BackendError)) throw error;
      this.unavailable.set(error.messages[0] ?? undefined);
    }
  }
}
