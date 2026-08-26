// ─── why ────────────────────────────────────────────────────────
// One row, cloned from np-commlink. Swipe semantics are the load-bearing part
// and are deliberately fixed: the DESTRUCTIVE action is always on `end` and a
// constructive one always on `start`, so muscle memory means the same thing on
// every list in the app. Both emitters close the sliding item BEFORE emitting,
// or the row stays open behind whatever the action opened.
//
// `detail` lines are projected rather than configured, because every list wants
// a different second line and an input per variant would be a union that grows
// with the app.
// ────────────────────────────────────────────────────────────────

import {
  booleanAttribute,
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import {
  IonIcon,
  IonItem,
  IonItemOption,
  IonItemOptions,
  IonItemSliding,
  IonLabel,
  IonList,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { trashOutline } from 'ionicons/icons';

@Component({
  selector: 'app-list-item',
  templateUrl: 'list-item.component.html',
  styleUrls: ['list-item.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonIcon,
    IonItem,
    IonItemOption,
    IonItemOptions,
    IonItemSliding,
    IonLabel,
    IonNote,
  ],
})
export class ListItemComponent {
  readonly ionList = input<IonList>();
  readonly title = input.required<string>();
  readonly note = input<string>();
  readonly leadingIcon = input<string>();
  readonly canDelete = input(true, { transform: booleanAttribute });

  readonly selectItem = output<void>();
  readonly deleteItem = output<void>();

  constructor() {
    addIcons({ trashOutline });
  }

  protected async emitDelete(): Promise<void> {
    await this.ionList()?.closeSlidingItems();
    this.deleteItem.emit();
  }
}
