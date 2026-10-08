// ─── why ────────────────────────────────────────────────────────
// Owners and werkstaetten are the same page. The role comes from route `data`
// rather than a param, because it does not vary — there are exactly two — and a
// param would say otherwise.
//
// Each partner is a card; the alias count is on it because it is the number
// that predicts how quiet the next import will be. Everything it did — the
// Wagen it keeps or owns, its Aufträge and work — is on its detail page
// (`/trains/partner/:id`), where removing it lives too.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
} from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { toSignal } from '@angular/core/rxjs-interop';
import { IonNote } from '@ionic/angular/standalone';
import { ListPageComponent } from '../../../@shared/feature/item-lists/list-page/list-page.component';
import { EntityCardComponent } from '../../../@shared/ui/base-item/entity-card/entity-card.component';
import { LIST_FACADE } from '../../../@shared/util/item-lists/list-page.facade';
import { PartnerListFacade } from '../../data';
import type { PartnerRolle } from '../../model/trains.types';

@Component({
  selector: 'app-page-partner-list',
  templateUrl: 'partner-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [EntityCardComponent, IonNote, ListPageComponent],
  providers: [{ provide: LIST_FACADE, useExisting: PartnerListFacade }],
})
export class PartnerListPage {
  readonly #list = inject(PartnerListFacade);
  readonly #router = inject(Router);
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

  protected onOpen(id: string): void {
    void this.#router.navigate(['/trains/partner', id]);
  }
}
