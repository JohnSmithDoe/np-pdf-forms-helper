// ─── why ────────────────────────────────────────────────────────
// The saved mappings. Their whole value is invisible until the second file from
// a sender arrives, so the row shows which fields a template covers — that is
// what tells the user whether it is the one they want to keep.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { TemplateListFacade, type TemplateRow, TrainsFacade } from '../../data';

@Component({
  selector: 'app-page-template-list',
  templateUrl: 'template-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ListItemComponent, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: TemplateListFacade }],
})
export class TemplateListPage {
  readonly #trains = inject(TrainsFacade);
  readonly #overlays = inject(OverlayService);
  readonly #reports = inject(ReportPresenterService);

  protected async onRemove(row: TemplateRow): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      `Vorlage ${row.name} wirklich entfernen?`
    );
    if (!confirmed) return;
    await this.#reports.run(() => this.#trains.removeTemplate(row.id));
  }
}
