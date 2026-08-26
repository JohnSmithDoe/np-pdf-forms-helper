// ─── why ────────────────────────────────────────────────────────
// The searchbar, cloned from np-commlink. Debounced, because every keystroke
// otherwise re-filters and re-sorts the whole list.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import { IonSearchbar, SearchbarCustomEvent } from '@ionic/angular/standalone';

@Component({
  selector: 'app-item-list-searchbar',
  templateUrl: 'item-list-searchbar.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonSearchbar],
})
export class ItemListSearchbarComponent {
  readonly query = input<string>();
  readonly queryChange = output<string | undefined>();

  protected onInput(event: SearchbarCustomEvent): void {
    this.queryChange.emit(event.detail.value ?? undefined);
  }
}
