// ─── why ────────────────────────────────────────────────────────
// The shell: `<ion-app>`, the menu, and the router outlet. Ionic expects
// `ion-app` to be the single root that overlays are teleported into — a modal,
// an alert and a toast all mount as children of it, not of whatever opened them.
//
// The toolbar and wordmark still live in the PAGE, not here. That is Ionic's
// layout contract: a page owns its `ion-header` + `ion-content` pair, and a
// header hoisted above `ion-router-outlet` sits outside the `.ion-page` box that
// Ionic sizes and scrolls, which is how you get two scrollbars. The menu is the
// one exception the contract allows, because it is a SIBLING of the outlet
// rather than a wrapper around it — and the outlet keeps its own `.ion-page`
// wrapper so the content half stays a proper page box.
//
// The `ion-split-pane` is `[disabled]` — the menu stays an overlay at every
// width, summoned by the burger button in the page toolbar. A permanently open
// sidebar costs a column of a desktop window to hold a single entry, and a
// disabled split pane keeps `ion-menu-button` visible instead of letting Ionic
// auto-hide it above the breakpoint. Dropping the attribute is the whole way
// back once there are enough entries to earn the column.
//
// The menu is MARKUP here rather than a component, because the ladder puts
// `type:ui` out of the shell's reach and there is nothing to reuse — there is
// one menu. The entry list stays here for the same reason it is not in
// `@shared`: what the app is made of is a shell-level fact, and this is the only
// other place besides `app.routes.ts` that names a domain.
//
// ONE ENTRY PER JOB, not per screen. Trains used to put its eight screens here
// as siblings, so the menu was a flat list in which nothing said that seven of
// them were entities of one module and the eighth was how data gets in. Now it
// has two entries, one per hub: „Dokumente“ (`/trains` — clean the sender's
// sheets and write them into the customer's master, the MVP) and „Schattensystem“
// (`/trains/erp`, the Schattensystem's dashboard). Same domain, different jobs;
// the menu says what the app is made of, a hub says what a part is made of.
//
// „Dokumente“ LEADS and the filler sits below under „Werkzeuge“ as „Formulare
// ausfüllen“ — not „Dokumente“, which is trains' own now, and two menu-level
// things with one name would be a guess every time.
//
// Icons are registered in the constructor because this is the component that
// RENDERS them, and importing the symbol makes a typo a TypeScript error rather
// than an invisible row.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import {
  IonApp,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonMenu,
  IonMenuToggle,
  IonRouterOutlet,
  IonSplitPane,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  documentsOutline,
  documentTextOutline,
  informationCircleOutline,
  trainOutline,
} from 'ionicons/icons';
import { APP_WORDMARK } from './@shared/model/app.consts';
import type { NavGroup } from './@shared/model/nav.types';

const NAV_GROUPS: NavGroup[] = [
  {
    items: [
      {
        route: '/trains',
        label: 'Dokumente',
        icon: 'documents-outline',
        exact: true,
      },
      { route: '/trains/erp', label: 'Schattensystem', icon: 'train-outline' },
      { route: '/about', label: 'Info', icon: 'information-circle-outline' },
    ],
  },
  {
    header: 'Werkzeuge',
    items: [
      {
        route: '/documents',
        label: 'Formulare ausfüllen',
        icon: 'document-text-outline',
      },
    ],
  },
];

@Component({
  selector: 'app-root',
  templateUrl: 'app.component.html',
  styleUrls: ['app.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonApp,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonMenu,
    IonMenuToggle,
    IonRouterOutlet,
    IonSplitPane,
    RouterLink,
    RouterLinkActive,
  ],
})
export class AppComponent {
  protected readonly wordmark = APP_WORDMARK;
  protected readonly navGroups = NAV_GROUPS;

  constructor() {
    addIcons({
      documentsOutline,
      documentTextOutline,
      informationCircleOutline,
      trainOutline,
    });
  }
}
