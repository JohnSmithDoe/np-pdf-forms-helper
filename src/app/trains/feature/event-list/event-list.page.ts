// ─── why ────────────────────────────────────────────────────────
// The one list that pages, because the event set does not fit in a message. The
// header shows the loaded count against the total so "50 von 4200" is visible
// rather than implied.
//
// It is called INSTANDHALTUNGEN and not "Wartungen", which is the whole point of
// `docs/fachdomaene.md`: Instandhaltung is the DIN 31051 umbrella and Wartung is
// one of its four sub-activities, so a heading saying Wartungen claims a list of
// one kind while showing all four — a Revision and a Verbesserung included.
//
// Events are not deletable from here. One is a record of something that
// happened, and the way to remove it is to remove its wagen — which the wagen
// list asks about explicitly.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { IonNote } from '@ionic/angular/standalone';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { EventListFacade, TrainsFacade } from '../../data';

@Component({
  selector: 'app-page-event-list',
  templateUrl: 'event-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonNote, ListItemComponent, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: EventListFacade }],
})
export class EventListPage {
  readonly #trains = inject(TrainsFacade);
  protected readonly list = inject(EventListFacade);

  protected readonly heading = computed(() => {
    const loaded = this.list.items()?.length ?? 0;
    const total = this.list.total();
    return total > loaded
      ? `Instandhaltungen (${loaded} von ${total})`
      : 'Instandhaltungen';
  });

  constructor() {
    void this.#trains.loadEvents();
  }
}
