// ─── why ────────────────────────────────────────────────────────
// The wizard's value step: one input per MAPPED NAME, and nothing else. It is
// the middle of `ExportPanelComponent` without the run button, the suffix or the
// summary — the wizard puts each of those on its own step, and a step that shows
// the next step's controls is not a step.
//
// It reads `exportFields` off the facade rather than taking them as an input,
// because that derivation is what makes one name fill every field sharing it.
// An input would need a parent computing the same grouping a second time.
//
// A leaf, like every `smart-ui`: it writes values through the facade and emits
// nothing, because typing a value cannot fail — there is no command behind it
// until the run step.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { IonInput } from '@ionic/angular/standalone';
import { inputValue } from '../../../@shared/util/input-value.utility';
import { FillerFacade } from '../../data';

@Component({
  selector: 'app-value-form',
  templateUrl: 'value-form.component.html',
  styleUrls: ['value-form.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonInput],
})
export class ValueFormComponent {
  readonly #facade = inject(FillerFacade);

  protected readonly exportFields = this.#facade.exportFields;

  protected setFieldValue(mappedName: string, event: Event): void {
    this.#facade.setFieldValue(mappedName, inputValue(event));
  }
}
