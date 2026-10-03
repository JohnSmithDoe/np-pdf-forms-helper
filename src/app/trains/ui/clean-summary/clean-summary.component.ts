// ─── why ────────────────────────────────────────────────────────
// The bar on top of the review: how much is still open, in the three tiers.
// It is the answer to "why is Weiter dead", so the two blocking counts come
// first and turn to `success` at zero, while the format count never blocks and
// stays `medium` whatever it says.
//
// `role="status"` so a screen reader hears the counts change after a reclean
// without the focus moving off the cell that was just corrected.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { IonChip, IonIcon, IonLabel } from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  alertCircleOutline,
  checkmarkCircleOutline,
  helpCircleOutline,
  sparklesOutline,
} from 'ionicons/icons';
import type { CleanSummary } from '../../model/trains.types';

@Component({
  selector: 'app-clean-summary',
  templateUrl: 'clean-summary.component.html',
  styleUrls: ['clean-summary.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonChip, IonIcon, IonLabel],
})
export class CleanSummaryComponent {
  readonly summary = input.required<CleanSummary>();

  constructor() {
    addIcons({
      alertCircleOutline,
      checkmarkCircleOutline,
      helpCircleOutline,
      sparklesOutline,
    });
  }
}
