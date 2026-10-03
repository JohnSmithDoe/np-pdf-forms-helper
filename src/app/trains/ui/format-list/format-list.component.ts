// ─── why ────────────────────────────────────────────────────────
// The lossless rewrites — `31.12.25` written out as `31.12.2025`, a stray space
// gone — grouped by column and rule, collapsed, with three samples each. They
// ask nothing of the user and block nothing; they are shown so that the cleaned
// file holds no change the user was not told about, and collapsed so that a
// thousand of them do not bury the two cards that do need a decision.
//
// Two Ionic behaviours shape this, both from `document-list`:
//   • `ion-accordion` does NOT defer its `slot="content"`, so the sample table
//     is gated on an `expanded` signal or every group's rows are built up front.
//   • `ionChange` BUBBLES. Anything inside an accordion that raises one would
//     reach the group's listener, so the handler checks the event came from the
//     group itself before it trusts `detail.value`.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  input,
  signal,
} from '@angular/core';
import {
  IonAccordion,
  IonAccordionGroup,
  IonBadge,
  IonItem,
  IonLabel,
} from '@ionic/angular/standalone';
import type { FormatGroup } from '../../model/trains.types';

const SAMPLES = 3;

@Component({
  selector: 'app-format-list',
  templateUrl: 'format-list.component.html',
  styleUrls: ['format-list.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonAccordion, IonAccordionGroup, IonBadge, IonItem, IonLabel],
})
export class FormatListComponent {
  readonly formats = input.required<FormatGroup[]>();

  protected readonly samples = SAMPLES;
  readonly #expanded = signal<ReadonlySet<string>>(new Set());

  protected key(group: FormatGroup): string {
    return `${group.column}:${group.rule}`;
  }

  protected isExpanded(group: FormatGroup): boolean {
    return this.#expanded().has(this.key(group));
  }

  protected onExpandedChange(event: Event): void {
    if (event.target !== event.currentTarget) return;
    const value = (event as CustomEvent<{ value?: string | string[] }>).detail
      .value;
    const open = Array.isArray(value) ? value : value ? [value] : [];
    this.#expanded.set(new Set(open));
  }
}
