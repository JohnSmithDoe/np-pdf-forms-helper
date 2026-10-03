// ─── why ────────────────────────────────────────────────────────
// One entity of the import walk — a Partner, a Wagen or a Radsatz as the file
// names it, however many rows it was named on — and the one question it asks:
// which existing record it is, a new one, or none.
//
// The choices are built from the RESOLUTION and nothing else, so the card
// cannot offer what the backend did not find: a `known` group offers its match
// and „nicht übernehmen“, a `likely` its suggestion beside „neu anlegen“, an
// `ambiguous` one every candidate with the reason it was found, a `new` one
// only „neu anlegen“. Nothing is preselected that the walk did not default,
// which is how an unanswered card stays visibly unanswered.
//
// Radio buttons, not a select: every option is visible at once, and a select's
// popover is the kind of Ionic internal the e2e suite deliberately leaves alone.
//
// The protocol lines are what the cleaning did to exactly THIS value, folded
// away behind a count. They are evidence for the decision, not part of it.
//
// `ui`: it emits the decision and the page sends it on. `dropped` says the
// group's rows all went with a declined Wagen — shown, not asked.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
  signal,
} from '@angular/core';
import {
  IonButton,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
  IonRadio,
  IonRadioGroup,
} from '@ionic/angular/standalone';
import type {
  EntityDecision,
  EntityGroup,
  PartnerRolle,
} from '../../model/trains.types';

interface Choice {
  value: string;
  label: string;
  hint?: string;
  decision: EntityDecision;
}

const ROLLEN: Record<PartnerRolle, string> = {
  werkstatt: 'Werkstatt',
  halter: 'Halter',
  eigentuemer: 'Eigentümer',
};

function valueOf(decision: EntityDecision | undefined): string | undefined {
  if (!decision) return undefined;
  return decision.action === 'use' ? `use:${decision.id}` : decision.action;
}

@Component({
  selector: 'app-entity-group',
  templateUrl: 'entity-group.component.html',
  styleUrls: ['entity-group.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    IonRadio,
    IonRadioGroup,
  ],
})
export class EntityGroupComponent {
  readonly group = input.required<EntityGroup>();
  readonly decision = input<EntityDecision>();
  readonly dropped = input(false);

  readonly decide = output<EntityDecision>();

  protected readonly showChanges = signal(false);

  protected readonly title = computed(() => {
    const group = this.group();
    return group.spellings.join(' · ') || group.key;
  });

  protected readonly subtitle = computed(() => {
    const group = this.group();
    const rows =
      group.rows.length === 1 ? '1 Zeile' : `${group.rows.length} Zeilen`;
    return group.rolle ? `${ROLLEN[group.rolle]} · ${rows}` : rows;
  });

  protected readonly status = computed(() => {
    const resolution = this.group().resolution;
    switch (resolution.state) {
      case 'known':
        return `Bekannt als „${resolution.name}“.`;
      case 'likely':
        return `Vermutlich „${resolution.name}“ — ${resolution.hint}`;
      case 'ambiguous':
        return 'Mehrere vorhandene Einträge kommen in Frage. Bitte wählen.';
      case 'new':
        return 'Noch nicht im Schattensystem.';
      case 'missing':
        return 'Kein Wert in der Datei.';
    }
  });

  protected readonly choices = computed<Choice[]>(() => {
    const group = this.group();
    const resolution = group.resolution;
    const skip: Choice = {
      value: 'skip',
      label:
        group.kind === 'wagen'
          ? 'Nicht übernehmen — die Zeilen entfallen'
          : 'Nicht übernehmen — Feld bleibt leer',
      decision: { action: 'skip' },
    };
    const create: Choice = {
      value: 'create',
      label: 'Neu anlegen',
      decision: { action: 'create' },
    };
    switch (resolution.state) {
      case 'known':
        return [
          {
            value: `use:${resolution.id}`,
            label: `Zuordnen: ${resolution.name}`,
            decision: { action: 'use', id: resolution.id },
          },
          skip,
        ];
      case 'likely':
        return [
          {
            value: `use:${resolution.id}`,
            label: `Ist: ${resolution.name}`,
            hint: resolution.hint,
            decision: { action: 'use', id: resolution.id },
          },
          create,
          skip,
        ];
      case 'ambiguous':
        return [
          ...resolution.candidates.map((candidate) => ({
            value: `use:${candidate.id}`,
            label: `Ist: ${candidate.name}`,
            hint: candidate.why,
            decision: { action: 'use', id: candidate.id } as EntityDecision,
          })),
          create,
          skip,
        ];
      case 'new':
        return [create, skip];
      case 'missing':
        return [skip];
    }
  });

  protected readonly value = computed(() => valueOf(this.decision()));

  protected onChoose(event: Event): void {
    const value = (event as CustomEvent<{ value?: string }>).detail.value;
    const choice = this.choices().find((entry) => entry.value === value);
    if (choice) this.decide.emit(choice.decision);
  }

  protected toggleChanges(): void {
    this.showChanges.update((shown) => !shown);
  }
}
