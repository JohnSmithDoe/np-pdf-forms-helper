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
// `import` is a HUB too: the guided import starts there (drop a folder, check
// the list), `manual` is the unchanged single-file import that unknown files
// fall through to, and `guided/*` are the steps of one file's walk, guarded in
// `intake.guards.ts`. The dashboard tile still points at `import`.
//
// The entry load hangs on a PATHLESS parent so every route below it shares one
// resolver run. Any of the six can be the first one reached, and the alternative
// is the same `load()` in six page constructors.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';
import {
  cleaningGuard,
  guidedPreviewGuard,
  guidedResultGuard,
} from './intake.guards';
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
    children: [
      {
        path: '',
        pathMatch: 'full',
        title: 'Import',
        loadComponent: () =>
          import('../feature/import-hub/import-hub.page').then(
            (m) => m.ImportHubPage
          ),
      },
      {
        path: 'manual',
        title: 'Import von Hand',
        loadComponent: () =>
          import('../feature/import-page/import.page').then(
            (m) => m.ImportPage
          ),
      },
      {
        path: 'guided/clean',
        title: 'Datei bereinigen',
        canActivate: [cleaningGuard],
        loadComponent: () =>
          import('../feature/guided-clean/guided-clean.page').then(
            (m) => m.GuidedCleanPage
          ),
      },
      {
        path: 'guided/preview',
        title: 'Zeilen prüfen',
        canActivate: [guidedPreviewGuard],
        loadComponent: () =>
          import('../feature/guided-preview/guided-preview.page').then(
            (m) => m.GuidedPreviewPage
          ),
      },
      {
        path: 'guided/result',
        title: 'Import abgeschlossen',
        canActivate: [guidedResultGuard],
        loadComponent: () =>
          import('../feature/guided-result/guided-result.page').then(
            (m) => m.GuidedResultPage
          ),
      },
    ],
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
