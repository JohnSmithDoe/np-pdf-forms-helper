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
//
// A `farbe` is drawn as a stripe down the card's left edge in that colour's
// Ionic role (`ion-color-…` sets `--ion-color-base`), not as a filled card: a
// red card would swallow the red and yellow badges inside it. The stripe is
// invisible to a screen reader, so the label names the mark.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
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
import type { Farbe } from '../../../model/farbe.types';
import { FARBE_COLOR, FARBE_LABEL } from '../../../util/farbe.utility';

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
  readonly farbe = input<Farbe>();

  protected readonly stripe = computed(() => {
    const farbe = this.farbe();
    return farbe ? `ion-color-${FARBE_COLOR[farbe]}` : undefined;
  });
  protected readonly ariaLabel = computed(() => {
    const farbe = this.farbe();
    return farbe
      ? `${this.title()}, markiert ${FARBE_LABEL[farbe]}`
      : this.title();
  });

  readonly open = output<void>();
}
