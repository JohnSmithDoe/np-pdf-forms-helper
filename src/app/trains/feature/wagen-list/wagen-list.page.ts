// ─── why ────────────────────────────────────────────────────────
// A list page is a facade, a heading and an `ng-template` row. That is the whole
// return on cloning np-commlink's pattern: five routes, one shell.
//
// Each Wagen is a CARD with the facts a dispatcher scans for — where it is and
// how long it has been silent (in `danger`, the dashboard's red, after seven
// days), open Schäden and Aufträge, the next Prüfung, how many Radsätze are
// fitted. Everything else is on its detail page (`/trains/wagen/:id`), which a
// click opens and where removing it lives too.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonBadge, IonNote } from '@ionic/angular/standalone';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { EntityCardComponent } from '../../../@shared/ui/base-item/entity-card/entity-card.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { WagenListFacade } from '../../data';

@Component({
  selector: 'app-page-wagen-list',
  templateUrl: 'wagen-list.page.html',
  styleUrls: ['wagen-list.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [EntityCardComponent, IonBadge, IonNote, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: WagenListFacade }],
})
export class WagenListPage {
  readonly #router = inject(Router);

  protected onOpen(id: string): void {
    void this.#router.navigate(['/trains/wagen', id]);
  }
}
