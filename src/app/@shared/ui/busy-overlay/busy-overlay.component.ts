// ─── why ────────────────────────────────────────────────────────
// Full-window busy scrim. Dumb by construction: `ui` may never reach `data`, so
// the busy state is passed in — `<app-busy-overlay [busy]="backend.busy()" />`.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { IonSpinner } from '@ionic/angular/standalone';

@Component({
  selector: 'app-busy-overlay',
  templateUrl: 'busy-overlay.component.html',
  styleUrls: ['busy-overlay.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonSpinner],
})
export class BusyOverlayComponent {
  readonly busy = input.required<boolean>();
  readonly label = input('Bitte warten …');
}
