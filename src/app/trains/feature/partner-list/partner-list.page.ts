// ─── why ────────────────────────────────────────────────────────
// Owners and werkstaetten are the same page. The role comes from route `data`
// rather than a param, because it does not vary — there are exactly two — and a
// param would say otherwise.
//
// The alias count is on the row because it is the number that predicts how quiet
// the next import will be.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { toSignal } from '@angular/core/rxjs-interop';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { PartnerListFacade, type PartnerRow, TrainsFacade } from '../../data';
import type { PartnerRolle } from '../../model/trains.types';

@Component({
  selector: 'app-page-partner-list',
  templateUrl: 'partner-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ListItemComponent, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: PartnerListFacade }],
})
export class PartnerListPage {
  readonly #trains = inject(TrainsFacade);
  readonly #list = inject(PartnerListFacade);
  readonly #overlays = inject(OverlayService);
  readonly #reports = inject(ReportPresenterService);
  readonly #data = toSignal(inject(ActivatedRoute).data);

  protected readonly role = computed<PartnerRolle>(
    () => (this.#data()?.['role'] as PartnerRolle) ?? 'werkstatt'
  );
  protected readonly heading = computed(() =>
    this.role() === 'halter' ? 'Eigentümer' : 'Werkstätten'
  );

  constructor() {
    effect(() => this.#list.setRole(this.role()));
  }

  protected async onRemove(row: PartnerRow): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      `${row.name} wirklich entfernen? Wagen und Wartungen bleiben erhalten und verlieren die Zuordnung.`
    );
    if (!confirmed) return;
    await this.#reports.run(() => this.#trains.removePartner(row.id));
  }
}
