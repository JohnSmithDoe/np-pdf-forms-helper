// ─── why ────────────────────────────────────────────────────────
// The Telematik list: the way in for someone looking for a device or a silent
// Wagen, which is why it starts from the devices and not from the Wagen list.
// One card per Wagen with Telematik, longest silent first — the order and every
// line are Rust's (`trains::telematik`). A silent one is `danger`, the
// customer's red. A click opens the Wagen's detail page; the detail page's
// Telematik rows come back with `?wagen=<id>`, and the list opens searched for
// that Wagen's number — visible in the searchbar, so clearing it shows them all.
//
// It reloads whenever the store's Wagen-Zustand changes, so an import that
// brings new readings shows here without a manual refresh.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  effect,
  inject,
  untracked,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router } from '@angular/router';
import { IonNote } from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { radioOutline } from 'ionicons/icons';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { EntityCardComponent } from '../../../@shared/ui/base-item/entity-card/entity-card.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { TelematikListFacade, TrainsFacade } from '../../data';

@Component({
  selector: 'app-page-telematik-list',
  templateUrl: 'telematik-list.page.html',
  styleUrls: ['telematik-list.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [EntityCardComponent, IonNote, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: TelematikListFacade }],
})
export class TelematikListPage {
  readonly #router = inject(Router);
  readonly #reports = inject(ReportPresenterService);
  readonly #trains = inject(TrainsFacade);
  readonly #list = inject(TelematikListFacade);
  readonly #query = toSignal(inject(ActivatedRoute).queryParamMap);

  constructor() {
    addIcons({ radioOutline });
    effect(() => {
      this.#trains.zustand();
      const wagen = this.#query()?.get('wagen');
      untracked(() => void this.#load(wagen));
    });
  }

  async #load(wagen: string | null | undefined): Promise<void> {
    await this.#reports.run(() => this.#list.load());
    if (wagen) this.#list.searchWagen(wagen);
  }

  protected onOpen(id: string): void {
    void this.#router.navigate(['/trains/wagen', id]);
  }
}
