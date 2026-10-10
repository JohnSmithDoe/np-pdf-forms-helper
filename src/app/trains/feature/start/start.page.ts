// ─── why ────────────────────────────────────────────────────────
// The document side of the app, and what it opens on: clean the sender's
// sheets, then write them into the customer's own master file. That is the
// whole job for now, so this hub has three ways in and no counts beyond the
// documents — Bereinigen (the action), Dokumente (where „In Master übertragen“
// starts), Master-Datei (which file it writes into).
//
// The Schattensystem lives apart, under „ERP“ in the menu (`/trains/erp`):
// importing into it is a different job from keeping the master current, and one
// hub for both made the master update a tile among a dozen.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonContent,
  IonHeader,
  IonIcon,
  IonMenuButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  colorWandOutline,
  documentsOutline,
  gridOutline,
} from 'ionicons/icons';
import { MasterFileFacade, TrainsFacade } from '../../data';
import { fileOf } from '../../util/path.util';

interface StartTile {
  route: string;
  label: string;
  icon: string;
  description: string;
  subtitle: string;
}

@Component({
  selector: 'app-page-trains-start',
  templateUrl: 'start.page.html',
  styleUrls: ['start.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonButtons,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonContent,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonTitle,
    IonToolbar,
  ],
})
export class TrainsStartPage {
  readonly #facade = inject(TrainsFacade);
  readonly #masterFile = inject(MasterFileFacade);
  readonly #router = inject(Router);

  protected readonly cleanTile: StartTile = {
    route: '/trains/clean',
    label: 'Bereinigen',
    icon: 'color-wand-outline',
    description:
      'Einen Ordner oder Dateien einlesen, erkennen lassen und bereinigen.',
    subtitle: 'Hier kommen die Dateien herein',
  };

  protected readonly tiles = computed<StartTile[]>(() => {
    const count = this.#facade.loaded()
      ? this.#facade.counts().dokumente
      : undefined;
    const target = this.#masterFile.target();
    return [
      {
        route: '/trains/documents',
        label: 'Dokumente',
        icon: 'documents-outline',
        description:
          'Die bereinigten Dateien — von hier aus in die Master-Datei übertragen.',
        subtitle:
          count === undefined
            ? 'wird geladen …'
            : count === 1
              ? '1 Dokument'
              : `${count} Dokumente`,
      },
      {
        route: '/trains/master-file',
        label: 'Master-Datei',
        icon: 'grid-outline',
        description:
          'Ihre Master-Datei, in die übertragen wird. Vor jedem Schreiben wird sie gesichert.',
        subtitle: target ? fileOf(target) : 'Noch keine gewählt',
      },
    ];
  });

  constructor() {
    addIcons({ colorWandOutline, documentsOutline, gridOutline });
  }

  protected open(tile: StartTile): void {
    void this.#router.navigate([tile.route]);
  }
}
