// ─── why ────────────────────────────────────────────────────────
// Steps one to three of the import walk — Partner, Wagen, Radsätze — as ONE
// page, with the kind in route `data`. The three ask the same question of
// different groups, and three copies of one page would be three places for the
// gate below to drift.
//
// The order is the dependency order of the commit and is fixed: partners
// first, then the Wagen the rows hang off, then the Radsätze. A declined Wagen
// therefore shows on the Radsatz step as „entfällt“ rather than as a question,
// because its rows are not going to be written at all.
//
// Weiter stays dead while any group of this step is unanswered — per entity,
// the never-auto-resolve rule. Nothing is written by any step; the commit on
// the summary is the only write, and leaving the walk forgets every answer.
//
// The bulk buttons answer only the groups still open, and only offer what the
// resolution allows — see `ImportWalkFacade`. Skipping all open Wagen asks
// first, because it drops every one of their rows.
//
// The first step's back button CANCELS the walk, and asks first: answers given
// are lost, and the cleaned document stays untouched in the list. A master
// sheet's walk returns to the master page instead — its run stays incomplete.
//
// On the Radsätze step a master sheet may also bring EINBAU conflicts: a
// Radsatz already fitted on the same Wagen under another date. They never
// block Weiter, because keeping the stored date is the preselected answer.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import {
  IonButton,
  IonLabel,
  IonListHeader,
  IonNote,
} from '@ionic/angular/standalone';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import {
  ImportWalkFacade,
  type BulkAnswer,
  type EinbauView,
  type GroupView,
} from '../../data';
import type {
  EntityDecision,
  EntityKind,
  PartnerRolle,
} from '../../model/trains.types';
import { IMPORT_PHASE, IMPORT_STEPS } from '../../model/import-walk';
import { EinbauKonfliktComponent } from '../../ui/einbau-konflikt/einbau-konflikt.component';
import { EntityGroupComponent } from '../../ui/entity-group/entity-group.component';

interface StepConfig {
  heading: string;
  step: number;
  intro: string;
  empty: string;
  back: string | undefined;
  next: string;
}

interface Section {
  label: string | undefined;
  views: GroupView[];
}

const STEPS: Record<EntityKind, StepConfig> = {
  partner: {
    heading: 'Partner prüfen',
    step: 1,
    intro:
      'Werkstätten, Halter und Eigentümer, wie die Datei sie nennt — je Name einmal, gleich auf wie vielen Zeilen.',
    empty: 'In dieser Datei kommen keine Partner vor.',
    back: undefined,
    next: '/trains/import/wagons',
  },
  wagen: {
    heading: 'Wagen prüfen',
    step: 2,
    intro:
      'Die Wagen, an denen die Zeilen hängen. Ein Wagen, der nicht übernommen wird, nimmt seine Zeilen mit.',
    empty: 'In dieser Datei kommt kein lesbarer Wagen vor.',
    back: '/trains/import/partners',
    next: '/trains/import/wheelsets',
  },
  radsatz: {
    heading: 'Radsätze prüfen',
    step: 3,
    intro:
      'Eine Radsatznummer gilt je Absender: dieselbe Nummer von zwei Werkstätten wird zweimal gefragt.',
    empty: 'In dieser Datei kommen keine Radsätze vor.',
    back: '/trains/import/wagons',
    next: '/trains/import/entries',
  },
};

const ROLLEN: readonly [PartnerRolle, string][] = [
  ['werkstatt', 'Werkstätten'],
  ['halter', 'Halter'],
  ['eigentuemer', 'Eigentümer'],
];

@Component({
  selector: 'app-page-import-entities',
  templateUrl: 'import-entities.page.html',
  styleUrls: ['import-entities.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    EinbauKonfliktComponent,
    EntityGroupComponent,
    IonButton,
    IonLabel,
    IonListHeader,
    IonNote,
    WizardShellComponent,
  ],
})
export class ImportEntitiesPage {
  protected readonly facade = inject(ImportWalkFacade);
  readonly #router = inject(Router);
  readonly #overlays = inject(OverlayService);

  protected readonly kind = inject(ActivatedRoute).snapshot.data[
    'kind'
  ] as EntityKind;
  protected readonly config = STEPS[this.kind];
  protected readonly phase = IMPORT_PHASE;
  protected readonly steps = IMPORT_STEPS;

  protected readonly views = computed(() => {
    switch (this.kind) {
      case 'partner': {
        return this.facade.partner();
      }
      case 'wagen': {
        return this.facade.wagen();
      }
      case 'radsatz': {
        return this.facade.radsaetze();
      }
    }
  });

  protected readonly sections = computed<Section[]>(() => {
    const views = this.views();
    if (this.kind !== 'partner') return [{ label: undefined, views }];
    return ROLLEN.map(([rolle, label]) => ({
      label,
      views: views.filter((view) => view.group.rolle === rolle),
    })).filter((section) => section.views.length > 0);
  });

  protected readonly einbauten = computed(() =>
    this.kind === 'radsatz' ? this.facade.einbauten() : []
  );

  protected readonly open = computed(() => this.facade.undecided()[this.kind]);
  protected readonly bulk = computed(() => this.facade.bulkCounts(this.kind));

  protected dropped(view: GroupView): boolean {
    const gone = this.facade.goneRows();
    return (
      this.kind !== 'wagen' && view.group.rows.every((row) => gone.has(row))
    );
  }

  protected onDecide(view: GroupView, decision: EntityDecision): void {
    this.facade.decide(view.group, decision);
  }

  protected onTake(view: EinbauView, uebernehmen: boolean): void {
    this.facade.takeEinbau([view.konflikt.key], uebernehmen);
  }

  protected onTakeAll(uebernehmen: boolean): void {
    this.facade.takeEinbau(
      this.einbauten().map((view) => view.konflikt.key),
      uebernehmen
    );
  }

  protected async onBulk(answer: BulkAnswer): Promise<void> {
    if (answer === 'skip' && this.kind === 'wagen') {
      const confirmed = await this.#overlays.confirm(
        `${this.bulk().skip} offene Wagen nicht übernehmen? Ihre Zeilen entfallen.`
      );
      if (!confirmed) return;
    }
    this.facade.answerOpen(this.kind, answer);
  }

  protected async onBack(): Promise<void> {
    if (this.config.back) {
      await this.#router.navigate([this.config.back]);
      return;
    }
    const master = this.facade.masterSheet() !== undefined;
    const confirmed = await this.#overlays.confirm(
      master
        ? 'Master-Import abbrechen? Die Entscheidungen gehen verloren, der Spiegel bleibt unvollständig.'
        : 'Import abbrechen? Die Entscheidungen gehen verloren, das bereinigte Dokument bleibt.'
    );
    if (!confirmed) return;
    await this.facade.cancel();
    await this.#router.navigate([
      master ? '/trains/master' : '/trains/documents',
    ]);
  }

  protected async onNext(): Promise<void> {
    await this.#router.navigate([this.config.next]);
  }
}
