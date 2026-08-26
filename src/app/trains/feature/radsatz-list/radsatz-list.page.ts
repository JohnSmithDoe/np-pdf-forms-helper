// ─── why ────────────────────────────────────────────────────────
// The radsaetze, each with its fitting history underneath. The history is the
// point of the page — a radsatz's value is knowing where it has been and what
// was done to it — so it is on the row rather than behind a navigation.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import { AlertController, IonNote } from '@ionic/angular/standalone';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { TrainsFacade, RadsatzListFacade, type RadsatzRow } from '../../data';

@Component({
  selector: 'app-page-radsatz-list',
  templateUrl: 'radsatz-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonNote, ListItemComponent, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: RadsatzListFacade }],
})
export class RadsatzListPage {
  readonly #trains = inject(TrainsFacade);
  readonly #alerts = inject(AlertController);

  protected readonly expanded = signal<string[]>([]);

  constructor() {
    void this.#trains.load();
  }

  protected isExpanded(id: string): boolean {
    return this.expanded().includes(id);
  }

  protected toggle(id: string): void {
    this.expanded.update((open) =>
      open.includes(id) ? open.filter((entry) => entry !== id) : [...open, id]
    );
  }

  protected async onRemove(row: RadsatzRow): Promise<void> {
    const alert = await this.#alerts.create({
      header: `Radsatz ${row.number} wirklich entfernen?`,
      message: 'Die Ein- und Ausbau-Historie wird mit entfernt.',
      buttons: [
        { text: 'Abbrechen', role: 'cancel' },
        { text: 'Bestätigen', role: 'confirm' },
      ],
    });
    await alert.present();
    const { role } = await alert.onWillDismiss();
    if (role !== 'confirm') return;
    try {
      await this.#trains.removeRadsatz(row.id);
    } catch (error) {
      if (!(error instanceof BackendError)) throw error;
    }
  }
}
