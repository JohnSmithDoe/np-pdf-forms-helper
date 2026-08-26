// ─── why ────────────────────────────────────────────────────────
// The routed page, and the only place in the domain that opens an overlay.
//
// That is not a style choice: Sheriff makes `smart-ui` a strict leaf, so the
// document list cannot import the field dialog and the profile bar cannot
// import the confirm alert. Each smart component emits "the user asked for X"
// and the composition point decides what that means. The rule buys something
// real — every modal flow in the app is readable in this one file, and a smart
// component stays inspectable through its outputs alone.
//
// It no longer loads. `clientDataResolver` on the parent `documents` route does
// that, because this is one of three pages that can be the first one reached —
// and with it went the welcome report, which rode that first response onto
// `report$` and would now be emitted before any page is subscribed.
//
// It still subscribes to `report$`, for every OTHER command's acknowledgement.
// The two commands whose report a wizard renders as a page are sent `silent`, so
// nothing arrives here twice.
//
// A confirmation is an `ion-alert` rather than a component, because that is all
// it is in substance: a question, two buttons, and a boolean. The answer arrives
// as the dismiss ROLE, so `'confirm'` is the contract and not the button's label
// — `OverlayService` owns that rule now, because four pages depend on it.
//
// A report is a TOAST unless it needs the user, and `ReportPresenterService`
// applies that rule for every page rather than this one keeping a private copy.
//
// Every facade call goes through `#run`, which is `ReportPresenterService.run`:
// a rejection from a template handler would otherwise become an unhandled
// rejection reaching only the global error listener, and the user would see
// nothing at all. Only `#present` acts on the folder a report can carry —
// `presentError` does not, because a FAILURE never has a `messageFolder` and the
// branch was unreachable. The host keeps `class: 'ion-page'` because Ionic sizes
// and scrolls that box — a routed page without it renders its header and content
// on top of one another.
//
// The mode toggle is in the toolbar, and its twin is on the wizard's start page:
// a mode you cannot leave from where you are is a mode you are stuck in.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { Router } from '@angular/router';
import {
  IonButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonIcon,
  IonMenuButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { documentTextOutline, sparklesOutline } from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { SettingsService } from '../../../@shared/data/settings/settings.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { APP_WORDMARK } from '../../../@shared/model/app.consts';
import type { ClientReport } from '../../../@shared/model/client.types';
import type { MappedField } from '../../../@shared/model/document.types';
import type { Profile } from '../../../@shared/model/profile.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import type { FillerDocument, FillerField } from '../../model/filler.types';
import { DocumentListComponent } from '../../smart-ui/document-list/document-list.component';
import { ExportPanelComponent } from '../../smart-ui/export-panel/export-panel.component';
import { FieldDialog } from '../../smart-ui/field-dialog/field.dialog';
import { ProfileBarComponent } from '../../smart-ui/profile-bar/profile-bar.component';
import { ProfileDialog } from '../../smart-ui/profile-dialog/profile.dialog';

@Component({
  selector: 'app-page-filler',
  templateUrl: 'filler.page.html',
  styleUrls: ['filler.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    DocumentListComponent,
    ExportPanelComponent,
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonTitle,
    IonToolbar,
    ProfileBarComponent,
  ],
})
export class FillerPage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #settings = inject(SettingsService);
  readonly #router = inject(Router);
  readonly #destroyRef = inject(DestroyRef);

  protected readonly wordmark = APP_WORDMARK;

  constructor() {
    addIcons({ documentTextOutline, sparklesOutline });

    this.facade.report$
      .pipe(takeUntilDestroyed(this.#destroyRef))
      .subscribe((report) => void this.#present(report));
  }

  protected onWizardMode(): void {
    this.#settings.setViewMode('wizard');
    void this.#router.navigate(['/documents/start']);
  }

  protected async onAddField(document: FillerDocument): Promise<void> {
    const field = await this.#overlays.openModal<MappedField>(FieldDialog, {
      document,
    });
    if (!field) return;

    await this.#run(async () => {
      await this.facade.addField(document.id, field);
      this.facade.setFieldSelected(field.origId, true);
    });
  }

  protected async onRemoveDocument(document: FillerDocument): Promise<void> {
    const question = `Dokument ${document.name} wirklich entfernen?`;
    if (!(await this.#confirm(question))) return;
    await this.#run(() => this.facade.removeDocument(document.id));
  }

  protected async onRemoveField(field: FillerField): Promise<void> {
    const question = `Feld ${field.mappedName} wirklich entfernen?`;
    if (!(await this.#confirm(question))) return;
    await this.#run(() => this.facade.removeField(field.origId));
  }

  protected async onReset(): Promise<void> {
    const question =
      'Dies löscht alle Daten aus dem Programm. Sind Sie sich wirklich sicher?';
    if (!(await this.#confirm(question))) return;
    await this.#run(() => this.facade.resetApp());
  }

  protected async onAddProfile(): Promise<void> {
    const name = await this.#overlays.openModal<string>(ProfileDialog);
    if (!name) return;
    await this.#run(() => this.facade.addProfile(name));
  }

  protected async onRemoveProfile(profile: Profile): Promise<void> {
    const question = `Export Profil ${profile.name} wirklich entfernen?`;
    if (!(await this.#confirm(question))) return;
    await this.#run(() => this.facade.removeProfile());
  }

  protected presentError(error: BackendError): void {
    void this.#reports.showError(error);
  }

  async #present(report: ClientReport): Promise<void> {
    const folder = await this.#reports.show(report);
    if (!folder) return;
    await this.#run(() => this.facade.openOutputFolder(folder));
  }

  #confirm(question: string): Promise<boolean> {
    return this.#overlays.confirm(question);
  }

  async #run(action: () => Promise<void>): Promise<void> {
    await this.#reports.run(action);
  }
}
