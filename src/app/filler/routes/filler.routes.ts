// ─── why ────────────────────────────────────────────────────────
// One tree under a pathless parent, so `clientDataResolver` runs once on entry
// rather than once per step: the parent stays activated while the children
// change, and a resolver on each step would re-read the database on every
// Weiter.
//
// The empty child redirects by MODE, and does it with a `RedirectFunction`
// because that runs in an injection context and can read `SettingsService`
// synchronously. It has to be synchronous: the redirect resolves inside
// `checkGuards`, one phase before any resolver, so nothing fetched could answer
// in time. That is the whole reason the preference is in localStorage.
//
// Route paths are ENGLISH and titles are German — the convention `/filler` +
// `'Dokumente ausfüllen'` already set. `setup/documents/:id` reuses
// `SetupFieldsPage`, np-commlink's `data: { … }` pattern the same way
// `trains.routes.ts` shares one page between owners and workshops: the two
// differ only in which document is shown and where Zurück goes.
//
// The first step of each wizard carries NO guard, deliberately. Every other step
// redirects there when its precondition fails, and a guard on the destination
// would bounce a cold deep link forever.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';
import { inject } from '@angular/core';
import { SettingsService } from '../../@shared/data/settings/settings.service';
import { clientDataResolver } from './client-data.resolver';
import {
  exportFieldsGuard,
  importedDocumentsGuard,
  runReportGuard,
  setupReportGuard,
} from './wizard.guards';

export const fillerRoutes: Routes = [
  {
    path: '',
    resolve: { clientData: clientDataResolver },
    children: [
      {
        path: '',
        pathMatch: 'full',
        redirectTo: () =>
          inject(SettingsService).viewMode() === 'expert'
            ? '/documents/expert'
            : '/documents/start',
      },
      {
        path: 'start',
        title: 'Was möchten Sie tun?',
        loadComponent: () =>
          import('../feature/wizard-start/wizard-start.page').then(
            (m) => m.WizardStartPage
          ),
      },
      {
        path: 'expert',
        title: 'Dokumente ausfüllen',
        loadComponent: () =>
          import('../feature/filler-page/filler.page').then(
            (m) => m.FillerPage
          ),
      },
      {
        path: 'setup',
        children: [
          { path: '', pathMatch: 'full', redirectTo: 'source' },
          {
            path: 'source',
            title: 'Dokument auswählen',
            loadComponent: () =>
              import('../feature/setup-source/setup-source.page').then(
                (m) => m.SetupSourcePage
              ),
          },
          {
            path: 'fields',
            title: 'Felder zuordnen',
            canActivate: [importedDocumentsGuard],
            loadComponent: () =>
              import('../feature/setup-fields/setup-fields.page').then(
                (m) => m.SetupFieldsPage
              ),
          },
          {
            path: 'documents',
            title: 'Verknüpfte Dokumente',
            canActivate: [importedDocumentsGuard],
            loadComponent: () =>
              import('../feature/setup-documents/setup-documents.page').then(
                (m) => m.SetupDocumentsPage
              ),
          },
          {
            path: 'documents/:id',
            title: 'Felder zuordnen',
            canActivate: [importedDocumentsGuard],
            loadComponent: () =>
              import('../feature/setup-fields/setup-fields.page').then(
                (m) => m.SetupFieldsPage
              ),
          },
          {
            path: 'result',
            title: 'Einrichtung abgeschlossen',
            canActivate: [setupReportGuard],
            loadComponent: () =>
              import('../feature/setup-result/setup-result.page').then(
                (m) => m.SetupResultPage
              ),
          },
        ],
      },
      {
        path: 'wizard',
        children: [
          { path: '', pathMatch: 'full', redirectTo: 'selection' },
          {
            path: 'selection',
            title: 'Dokumente auswählen',
            loadComponent: () =>
              import('../feature/export-selection/export-selection.page').then(
                (m) => m.ExportSelectionPage
              ),
          },
          {
            path: 'values',
            title: 'Werte eingeben',
            canActivate: [exportFieldsGuard],
            loadComponent: () =>
              import('../feature/export-values/export-values.page').then(
                (m) => m.ExportValuesPage
              ),
          },
          {
            path: 'generate',
            title: 'Dokumente erzeugen',
            canActivate: [exportFieldsGuard],
            loadComponent: () =>
              import('../feature/export-generate/export-generate.page').then(
                (m) => m.ExportGeneratePage
              ),
          },
          {
            path: 'result',
            title: 'Ergebnis',
            canActivate: [runReportGuard],
            loadComponent: () =>
              import('../feature/export-result/export-result.page').then(
                (m) => m.ExportResultPage
              ),
          },
        ],
      },
    ],
  },
];
