// ─── why ────────────────────────────────────────────────────────
// The empty state, and it has THREE cases rather than one, because "nothing
// matched your search" and "there is nothing here yet" want different words and
// different buttons.
//
// It renders nothing while the list is UNKNOWN. `items()` is `undefined` until
// the first response and `[]` once known-empty, and collapsing the two flashes
// "noch keine" over data that is on its way.
// ────────────────────────────────────────────────────────────────

import {
  booleanAttribute,
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import {
  IonButton,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardTitle,
} from '@ionic/angular/standalone';

@Component({
  selector: 'app-item-list-empty',
  templateUrl: 'item-list-empty.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonButton, IonCard, IonCardContent, IonCardHeader, IonCardTitle],
})
export class ItemListEmptyComponent {
  readonly isKnownEmpty = input(false, { transform: booleanAttribute });
  readonly isSearching = input(false, { transform: booleanAttribute });
  readonly searchTerm = input<string>();
  readonly emptyText = input('Noch keine Einträge vorhanden.');
  readonly createLabel = input<string>();

  readonly createRequested = output<void>();
}
