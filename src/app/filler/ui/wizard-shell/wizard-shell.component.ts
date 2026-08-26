// ─── why ────────────────────────────────────────────────────────
// The chrome every wizard step wears: the toolbar, the "Schritt 2 von 4"
// progress, the body, and the Zurück/Weiter pair. Eight step pages share it, so
// what a step actually owns is its content and its two handlers.
//
// Ionic ships no stepper, so this is hand-built — and deliberately built out of
// `ion-progress-bar` plus a counted line rather than a row of numbered circles.
// A circle row is where a component starts naming colours to show "done" and
// "current"; a progress bar carries that in a role Ionic already themes, in both
// palettes, for free.
//
// THIS element carries `.ion-page`, put there by the step page's template, not
// the routed component around it — the class has to sit on whatever actually
// holds the header/content/footer trio, which is this. Nothing here may set
// `:host { display: … }` either: `.ion-page` is a flex column, and overriding it
// stops `ion-content` flexing and pushes the footer off the bottom of the
// viewport. Same rule and same reason as `@shared/feature/item-lists/list-page`.
//
// It is `type:ui`: no service, no router, no facade. A step page decides what
// Weiter MEANS and navigates; this decides only what the pair looks like and
// when Weiter is dead. That is what keeps the same chrome over a step that
// imports a file and a step that shows a report.
//
// `nextDisabled` is an input rather than something inferred here, because the
// condition differs per step and only the step knows it — and a Weiter that
// looks alive but does nothing is worse than one visibly not ready.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
} from '@angular/core';
import {
  IonButton,
  IonButtons,
  IonContent,
  IonFooter,
  IonHeader,
  IonIcon,
  IonMenuButton,
  IonNote,
  IonProgressBar,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { chevronBackOutline, chevronForwardOutline } from 'ionicons/icons';

@Component({
  selector: 'app-wizard-shell',
  templateUrl: 'wizard-shell.component.html',
  styleUrls: ['wizard-shell.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonButtons,
    IonContent,
    IonFooter,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonNote,
    IonProgressBar,
    IonTitle,
    IonToolbar,
  ],
})
export class WizardShellComponent {
  readonly heading = input.required<string>();
  readonly step = input.required<number>();
  readonly stepCount = input.required<number>();

  readonly backLabel = input('Zurück');
  readonly nextLabel = input('Weiter');
  readonly nextColor = input('primary');
  readonly nextDisabled = input(false);
  readonly showBack = input(true);
  readonly showNext = input(true);

  readonly back = output<void>();
  readonly next = output<void>();

  protected readonly progress = computed(() => this.step() / this.stepCount());

  constructor() {
    addIcons({ chevronBackOutline, chevronForwardOutline });
  }
}
