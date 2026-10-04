// ─── why ────────────────────────────────────────────────────────
// A list page is a facade, a heading and an `ng-template` row. That is the whole
// return on cloning np-commlink's pattern: five routes, one shell.
//
// It is the routed page, so it owns the overlays — the confirm before a delete
// and the error presentation — exactly as `filler.page.ts` does. Both come from
// the shared services: `OverlayService.confirm` owns the dismiss-role contract,
// and the consequence rides in the QUESTION because an `ion-alert` announces its
// header as the dialog's accessible name and its message after focus.
//
// The fitted radsaetze open on tap, like the radsatz list's history: the count
// is always on the row, the lines only when asked for — eight lines on each of
// 400 wagen would bury the list.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import { IonNote } from '@ionic/angular/standalone';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { TrainsFacade, WagenListFacade, type WagenRow } from '../../data';

@Component({
  selector: 'app-page-wagen-list',
  templateUrl: 'wagen-list.page.html',
  styleUrls: ['wagen-list.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonNote, ListItemComponent, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: WagenListFacade }],
})
export class WagenListPage {
  readonly #trains = inject(TrainsFacade);
  readonly #overlays = inject(OverlayService);
  readonly #reports = inject(ReportPresenterService);

  protected readonly expanded = signal<string[]>([]);

  protected isExpanded(id: string): boolean {
    return this.expanded().includes(id);
  }

  protected toggle(id: string): void {
    this.expanded.update((open) =>
      open.includes(id) ? open.filter((entry) => entry !== id) : [...open, id]
    );
  }

  protected async onRemove(row: WagenRow): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      `Wagen ${row.nummer} wirklich entfernen? Alle Wartungen dieses Wagens werden mit entfernt.`
    );
    if (!confirmed) return;
    await this.#reports.run(() => this.#trains.removeWagen(row.id));
  }
}
