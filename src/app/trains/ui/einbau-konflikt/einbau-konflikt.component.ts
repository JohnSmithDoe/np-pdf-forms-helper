// ─── why ────────────────────────────────────────────────────────
// One Einbau conflict of the master import: a Radsatz already fitted on this
// Wagen, and a later sheet naming the same fitting with another install date.
// It is one fitting with a disputed date, not a movement, so the question is
// only WHICH DATE — never whether the Radsatz was removed.
//
// Keeping the stored date is preselected because it writes nothing; taking the
// new one corrects the stored fitting in place. Radio buttons for the reason
// `entity-group` gives: both options visible, no popover.
//
// `ui`: it emits the choice and the page sends it on.
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
  IonItem,
  IonLabel,
  IonRadio,
  IonRadioGroup,
} from '@ionic/angular/standalone';
import type { EinbauKonflikt } from '../../model/trains.types';
import { formatIsoDate } from '../../util/uic.utility';

@Component({
  selector: 'app-einbau-konflikt',
  templateUrl: 'einbau-konflikt.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonItem,
    IonLabel,
    IonRadio,
    IonRadioGroup,
  ],
})
export class EinbauKonfliktComponent {
  readonly konflikt = input.required<EinbauKonflikt>();
  readonly uebernehmen = input.required<boolean>();
  readonly take = output<boolean>();

  protected readonly bisher = computed(() => {
    const konflikt = this.konflikt();
    const date = konflikt.bisher
      ? formatIsoDate(konflikt.bisher)
      : 'ohne Datum';
    return konflikt.bisherQuelle ? `${date} (${konflikt.bisherQuelle})` : date;
  });
  protected readonly neu = computed(() => {
    const konflikt = this.konflikt();
    const rows = konflikt.rows.join(', ');
    return `${formatIsoDate(konflikt.neu)} (Zeile ${rows})`;
  });

  protected onChoose(event: Event): void {
    const value = (event as CustomEvent<{ value: string }>).detail.value;
    this.take.emit(value === 'neu');
  }
}
