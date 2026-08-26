// ─── why ────────────────────────────────────────────────────────
// The domain's own manifest, so the shell reaches trains through one import and
// nothing else.
//
// `owners` and `werkstaetten` are the SAME page and the same facade with the role in
// route `data` — np-commlink's `data: { listId }` pattern. A real param would say
// the value varies; it does not, there are exactly two.
//
// The empty path is the DASHBOARD and no longer a redirect to `import`. It is the
// domain's one menu entry, so what it lands on has to be the way to all of the
// rest — a redirect would have made the import the hub and left seven lists
// reachable only by URL.
//
// The entry load hangs on a PATHLESS parent so every route below it shares one
// resolver run. Any of the six can be the first one reached, and the alternative
// is the same `load()` in six page constructors.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';
import { trainsDataResolver } from './trains-data.resolver';

const pages: Routes = [
  {
    path: '',
    pathMatch: 'full',
    title: 'Schattensystem',
    loadComponent: () =>
      import('../feature/dashboard/dashboard.page').then(
        (m) => m.TrainsDashboardPage
      ),
  },
  {
    path: 'import',
    title: 'Import',
    loadComponent: () =>
      import('../feature/import-page/import.page').then((m) => m.ImportPage),
  },
  {
    path: 'wagen',
    title: 'Wagen',
    loadComponent: () =>
      import('../feature/wagen-list/wagen-list.page').then(
        (m) => m.WagenListPage
      ),
  },
  {
    path: 'radsaetze',
    title: 'Radsätze',
    loadComponent: () =>
      import('../feature/radsatz-list/radsatz-list.page').then(
        (m) => m.RadsatzListPage
      ),
  },
  {
    path: 'halter',
    title: 'Halter',
    data: { role: 'halter' },
    loadComponent: () =>
      import('../feature/partner-list/partner-list.page').then(
        (m) => m.PartnerListPage
      ),
  },
  {
    path: 'eigentuemer',
    title: 'Eigentümer',
    data: { role: 'eigentuemer' },
    loadComponent: () =>
      import('../feature/partner-list/partner-list.page').then(
        (m) => m.PartnerListPage
      ),
  },
  {
    path: 'werkstaetten',
    title: 'Werkstätten',
    data: { role: 'werkstatt' },
    loadComponent: () =>
      import('../feature/partner-list/partner-list.page').then(
        (m) => m.PartnerListPage
      ),
  },
  {
    path: 'instandhaltungen',
    title: 'Instandhaltungen',
    loadComponent: () =>
      import('../feature/event-list/event-list.page').then(
        (m) => m.EventListPage
      ),
  },
  {
    path: 'templates',
    title: 'Zuordnungs-Vorlagen',
    loadComponent: () =>
      import('../feature/template-list/template-list.page').then(
        (m) => m.TemplateListPage
      ),
  },
];

export const trainsRoutes: Routes = [
  { path: '', resolve: { data: trainsDataResolver }, children: pages },
];
