// ─── why ────────────────────────────────────────────────────────
// A `ClientReport`'s BODY, with no chrome around it, because it now has three
// hosts: `ReportDialog` wraps it in a modal, and each wizard's result step wraps
// it in a page whose footer carries that wizard's next actions. Extracting it is
// what stops the result pages from being a second rendering of the same thing,
// drifting from the dialog one bugfix at a time.
//
// The headline is NOT here. Each host already has the right slot for it —
// `ion-title` in the dialog's toolbar, the page title on a result step — and a
// heading rendered twice is a heading a screen reader announces twice.
//
// "Ordner öffnen" IS here rather than in each host's footer, because a report
// carrying a `messageFolder` always offers it and a host that forgot would drop
// the only path to the run's output. The button emits; what opening means is the
// host's business (the dialog dismisses with a role, a page calls the facade).
//
// `messages` render as TEXT. They carry filesystem-derived names, so
// `[innerHtml]` would be an HTML sink fed from disk.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import { IonButton, IonIcon, IonNote } from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { folderOpenOutline } from 'ionicons/icons';
import type { ClientReport } from '../../model/client.types';

@Component({
  selector: 'app-report-view',
  templateUrl: 'report-view.component.html',
  styleUrls: ['report-view.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonButton, IonIcon, IonNote],
})
export class ReportViewComponent {
  readonly report = input.required<ClientReport>();

  readonly openFolder = output<string>();

  constructor() {
    addIcons({ folderOpenOutline });
  }
}
