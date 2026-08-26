// ─── why ────────────────────────────────────────────────────────
// The list page shell, cloned from np-commlink where thirteen pages share it.
// It depends on `LIST_FACADE` and on a row TEMPLATE, so a new list costs a
// facade and an `ng-template` and no new page.
//
// It is `type:feature` and lives in `@shared` because a domain's feature layer
// may only reach another feature through the `featureMayUseSharedFeature`
// predicate already sitting in `sheriff.config.ts` — this is its first user.
//
// THIS component carries `.ion-page`, not the routed page around it — the class
// has to sit on the element that actually holds the header/content pair, which
// is this one. Nothing here may set `:host { display: block }` either: that
// overrides `.ion-page`'s flex column and stops `ion-content` flexing.
//
// `isKnownEmpty` is `items()?.length === 0` and not `!items()?.length`. The
// former is false while the list is still unknown; the latter is true, and shows
// the empty state over data that has not arrived.
//
// `backHref` REPLACES the burger rather than joining it, and that is the whole
// meaning of the input: a list the menu links to directly is a top-level screen
// and needs the menu, a list reached from a domain dashboard needs the way back
// up. Two controls in the same slot would offer both readings of where the page
// sits. `ion-back-button` falls back to the href only when there is no history,
// so a cold deep link still lands on the hub.
// ────────────────────────────────────────────────────────────────

import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  TemplateRef,
  viewChild,
} from '@angular/core';
import {
  IonBackButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonIcon,
  IonInfiniteScroll,
  IonInfiniteScrollContent,
  IonList,
  IonMenuButton,
  IonTitle,
  IonToolbar,
  IonButton,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { addOutline, arrowDownOutline, arrowUpOutline } from 'ionicons/icons';
import type { BaseItem } from '../../../model/item-list.types';
import { ItemListEmptyComponent } from '../../../ui/base-item/item-list-empty/item-list-empty.component';
import { ItemListSearchbarComponent } from '../../../ui/base-item/item-list-searchbar/item-list-searchbar.component';
import { LIST_FACADE } from '../../../util/item-lists/list-page.facade';

export interface ListRowContext {
  $implicit: BaseItem;
  ionList: IonList | undefined;
}

@Component({
  selector: 'app-list-page',
  templateUrl: 'list-page.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonBackButton,
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonIcon,
    IonInfiniteScroll,
    IonInfiniteScrollContent,
    IonList,
    IonMenuButton,
    IonTitle,
    IonToolbar,
    ItemListEmptyComponent,
    ItemListSearchbarComponent,
    NgTemplateOutlet,
  ],
})
export class ListPageComponent {
  protected readonly facade = inject(LIST_FACADE);

  readonly heading = input.required<string>();
  readonly itemTemplate = input.required<TemplateRef<ListRowContext>>();
  readonly emptyText = input('Noch keine Einträge vorhanden.');
  readonly createLabel = input<string>();
  readonly backHref = input<string>();

  protected readonly ionList = viewChild<IonList>('ionList');

  protected readonly canCreate = !!this.facade.create;
  protected readonly canLoadMore = !!this.facade.loadMore;
  protected readonly isKnownEmpty = computed(
    () => this.facade.items()?.length === 0
  );

  constructor() {
    addIcons({ addOutline, arrowDownOutline, arrowUpOutline });
  }

  protected onCreate(): void {
    this.facade.create?.();
  }

  protected onLoadMore(event: Event): void {
    this.facade.loadMore?.();
    void (event.target as HTMLIonInfiniteScrollElement).complete();
  }

  protected sortIcon(key: string): string {
    const sort = this.facade.sort();
    if (sort?.sortBy !== key) return 'arrow-down-outline';
    return sort.sortDirection === 'asc'
      ? 'arrow-up-outline'
      : 'arrow-down-outline';
  }
}
