// ─── why ────────────────────────────────────────────────────────
// ONE page for a Wagen, a Radsatz and a Partner: it renders the `EntityDetail`
// Rust built (`get_entity_detail`) — fields, then sections of rows — and
// decides nothing about what is shown or how it is spelled. The kind comes from
// route `data`, the id from the param; the view reloads when either changes,
// because following a link from one Wagen to another reuses the page.
//
// A row or field with a `link` is a button that opens that entity's page, which
// is what makes the information hub navigable: Wagen → its Radsatz → the other
// Wagen it ran in → the Werkstatt that worked on it.
//
// Removing lives here, not on the list: a list row is a card, and the card is
// the button that opens this page. After removing, the page goes back to the
// kind's list — the entity it showed no longer exists.
//
// A row's `tone` becomes an icon in Ionic's colour, never a CSS colour.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
  untracked,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router } from '@angular/router';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  alertCircleOutline,
  timeOutline,
  trashOutline,
  warningOutline,
} from 'ionicons/icons';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { TrainsFacade } from '../../data';
import type {
  DetailLink,
  DetailTone,
  EntityDetail,
  EntityRef,
} from '../../model/trains.types';

const ROUTES: Record<EntityRef, string> = {
  wagen: '/trains/wagen',
  radsatz: '/trains/radsaetze',
  partner: '/trains/partner',
};

const LISTS: Record<EntityRef, string> = {
  wagen: '/trains/wagen',
  radsatz: '/trains/radsaetze',
  partner: '/trains',
};

const TONE_ICONS: Record<DetailTone, string> = {
  danger: 'alert-circle-outline',
  warning: 'warning-outline',
  medium: 'time-outline',
};

@Component({
  selector: 'app-page-entity-detail',
  templateUrl: 'entity-detail.page.html',
  styleUrls: ['entity-detail.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonBackButton,
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class EntityDetailPage {
  readonly #trains = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);
  readonly #route = inject(ActivatedRoute);
  readonly #params = toSignal(this.#route.paramMap);
  readonly #data = toSignal(this.#route.data);

  protected readonly kind = computed<EntityRef>(
    () => (this.#data()?.['kind'] as EntityRef | undefined) ?? 'wagen'
  );
  protected readonly id = computed(() => this.#params()?.get('id') ?? '');
  protected readonly detail = signal<EntityDetail | undefined>(undefined);
  protected readonly backHref = computed(() => LISTS[this.kind()]);
  protected readonly toneIcons = TONE_ICONS;

  constructor() {
    addIcons({ alertCircleOutline, timeOutline, trashOutline, warningOutline });
    effect(() => {
      const kind = this.kind();
      const id = this.id();
      untracked(() => void this.#load(kind, id));
    });
  }

  protected onOpen(link: DetailLink | undefined): void {
    if (!link) return;
    void this.#router.navigate([ROUTES[link.kind], link.id]);
  }

  protected async onRemove(): Promise<void> {
    const detail = this.detail();
    if (!detail) return;
    const confirmed = await this.#overlays.confirm(question(detail));
    if (!confirmed) return;
    const removed = await this.#reports.run(() => this.#remove(detail));
    if (removed) {
      await this.#router.navigate([LISTS[detail.kind]], { replaceUrl: true });
    }
  }

  #remove(detail: EntityDetail): Promise<void> {
    switch (detail.kind) {
      case 'wagen':
        return this.#trains.removeWagen(detail.id);
      case 'radsatz':
        return this.#trains.removeRadsatz(detail.id);
      case 'partner':
        return this.#trains.removePartner(detail.id);
    }
  }

  async #load(kind: EntityRef, id: string): Promise<void> {
    this.detail.set(undefined);
    if (!id) return;
    await this.#reports.run(async () => {
      const detail = await this.#trains.entityDetail(kind, id);
      if (kind === this.kind() && id === this.id()) this.detail.set(detail);
    });
  }
}

function question(detail: EntityDetail): string {
  switch (detail.kind) {
    case 'wagen':
      return `Wagen ${detail.title} wirklich entfernen? Seine Wartungen, Einbauten und sein Wagen-Zustand werden mit entfernt.`;
    case 'radsatz':
      return `Radsatz ${detail.title} wirklich entfernen? Die Ein- und Ausbau-Historie wird mit entfernt.`;
    case 'partner':
      return `${detail.title} wirklich entfernen? Wagen und Wartungen bleiben erhalten und verlieren die Zuordnung.`;
  }
}
