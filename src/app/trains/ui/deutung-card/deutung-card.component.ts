// ─── why ────────────────────────────────────────────────────────
// One column the backend could read two ways — or one whose cells it read with
// a caveat — and refuses to settle alone. The card exists to make the user's
// decision CHEAP: the reason in one sentence, then the same cells side by side
// as they come out under each reading, so "1.234 → 1234 or 1,234?" is answered
// by looking rather than by knowing what a decimal style is.
//
// The proposed reading is preselected but NOT confirmed. Confirming is a click,
// always, because the rule the import is built on is that nothing with a second
// possible reading is resolved without the user.
//
// `selected` is a `linkedSignal` over the backend's `chosen`: the user may flip
// the segment back and forth while comparing, and only „Bestätigen“ sends it.
// After a confirmation the backend answers with `chosen` set to what was
// confirmed (it re-reads the column), and the link snaps `selected` to that —
// so a confirmed card shows the reading it was confirmed with, and flipping it
// again is a visible, unconfirmed change.
//
// The column for the selected reading is in `<strong>`, not in a colour: the
// stylesheet stays layout-only, and weight survives both palettes.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  linkedSignal,
  output,
} from '@angular/core';
import {
  IonButton,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonChip,
  IonIcon,
  IonLabel,
  IonNote,
  IonSegment,
  IonSegmentButton,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { checkmarkCircleOutline } from 'ionicons/icons';
import { labelOf } from '../../model/field-catalogue';
import {
  DATE_ORDER_LABELS,
  DATE_ORDER_SHORT,
  DECIMAL_LABELS,
  DECIMAL_SHORT,
} from '../../model/reading-labels';
import type {
  Confirmation,
  DateOrder,
  DecimalStyle,
  DeutungCard,
} from '../../model/trains.types';

interface ReadingOption {
  value: string;
  label: string;
  short: string;
}

@Component({
  selector: 'app-deutung-card',
  templateUrl: 'deutung-card.component.html',
  styleUrls: ['deutung-card.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonChip,
    IonIcon,
    IonLabel,
    IonNote,
    IonSegment,
    IonSegmentButton,
  ],
})
export class DeutungCardComponent {
  readonly card = input.required<DeutungCard>();

  readonly confirm = output<Confirmation>();

  protected readonly fieldLabel = computed(() => labelOf(this.card().field));

  protected readonly options = computed<ReadingOption[]>(() => {
    const reading = this.card().reading;
    switch (reading.kind) {
      case 'decimal':
        return [reading.chosen, reading.alternative].map((style) => ({
          value: style,
          label: DECIMAL_LABELS[style],
          short: DECIMAL_SHORT[style],
        }));
      case 'dateOrder':
        return [reading.chosen, reading.alternative].map((order) => ({
          value: order,
          label: DATE_ORDER_LABELS[order],
          short: DATE_ORDER_SHORT[order],
        }));
      case 'hinweis':
        return [];
    }
  });

  protected readonly selected = linkedSignal<string>(() => {
    const reading = this.card().reading;
    return reading.kind === 'hinweis' ? '' : reading.chosen;
  });

  protected readonly settled = computed(() => {
    const reading = this.card().reading;
    return (
      this.card().confirmed &&
      (reading.kind === 'hinweis' || reading.chosen === this.selected())
    );
  });

  protected readonly alternativeSelected = computed(() => {
    const reading = this.card().reading;
    return (
      reading.kind !== 'hinweis' && reading.alternative === this.selected()
    );
  });

  protected readonly selectedLabel = computed(
    () =>
      this.options().find((option) => option.value === this.selected())
        ?.short ?? ''
  );

  constructor() {
    addIcons({ checkmarkCircleOutline });
  }

  protected onSelect(event: Event): void {
    const value = (event as CustomEvent<{ value?: string | number }>).detail
      .value;
    if (value !== undefined) this.selected.set(String(value));
  }

  protected onConfirm(): void {
    const { column, reading } = this.card();
    switch (reading.kind) {
      case 'decimal':
        this.confirm.emit({
          kind: 'decimal',
          column,
          style: this.selected() as DecimalStyle,
        });
        return;
      case 'dateOrder':
        this.confirm.emit({
          kind: 'dateOrder',
          column,
          order: this.selected() as DateOrder,
        });
        return;
      case 'hinweis':
        this.confirm.emit({ kind: 'hinweis', column });
        return;
    }
  }
}
