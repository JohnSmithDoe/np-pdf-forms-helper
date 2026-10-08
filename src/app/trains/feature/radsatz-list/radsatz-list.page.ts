// ─── why ────────────────────────────────────────────────────────
// The radsaetze as cards: where each runs now and since when. Its history —
// every Einbau and the work done to it, each linking to its Wagen — is the
// point of a Radsatz and lives on its detail page (`/trains/radsaetze/:id`),
// which a click opens and where removing it lives too.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonNote } from '@ionic/angular/standalone';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { EntityCardComponent } from '../../../@shared/ui/base-item/entity-card/entity-card.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { RadsatzListFacade, TrainsFacade } from '../../data';

@Component({
  selector: 'app-page-radsatz-list',
  templateUrl: 'radsatz-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [EntityCardComponent, IonNote, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: RadsatzListFacade }],
})
export class RadsatzListPage {
  readonly #trains = inject(TrainsFacade);
  readonly #router = inject(Router);

  constructor() {
    void this.#trains.load();
  }

  protected onOpen(id: string): void {
    void this.#router.navigate(['/trains/radsaetze', id]);
  }
}
