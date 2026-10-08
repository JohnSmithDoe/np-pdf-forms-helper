// ─── why ────────────────────────────────────────────────────────
// One entity as a card in a list: title, a muted note, and whatever the list
// projects as the body — the most important facts, never all of them. The whole
// card is the button and opens the entity's detail page, so it carries no
// second action: a delete inside a clickable card is a nested control, and it
// lives on the detail page instead.
//
// It keeps the list row's test ids (`list-row`, `list-row-title`), because a
// card IS the list's row and the specs that search a list should not care how
// it is drawn.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import {
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonIcon,
} from '@ionic/angular/standalone';

@Component({
  selector: 'app-entity-card',
  templateUrl: 'entity-card.component.html',
  styleUrls: ['entity-card.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonIcon,
  ],
})
export class EntityCardComponent {
  readonly title = input.required<string>();
  readonly note = input<string>();
  readonly icon = input<string>();

  readonly open = output<void>();
}
