// ─── why ────────────────────────────────────────────────────────
// The domain's own manifest, so the shell reaches trains through one import and
// nothing else.
//
// `owners` and `werkstaetten` are the SAME page and the same facade with the role in
// route `data` — np-commlink's `data: { listId }` pattern. A real param would say
// the value varies; it does not, there are exactly two.
//
// TWO HUBS, two menu entries. The empty path is „Dokumente“ (`feature/start`):
// clean, file, write into the customer's master — the MVP, and what the app
// opens on. `erp` is the Schattensystem's DASHBOARD, the way to every entity
// list, the import and the master import; a redirect instead would have left
// seven lists reachable only by URL.
//
// The guided import is TWO walks, and the URLs say which one a page belongs to.
// `clean` is the cleaning hub (drop a folder, check the list), `clean/file`
// one file's review, `clean/summary` the batch's end and `clean/template` the
// mapper an unknown file is sent to. `documents` lists what the cleaning
// filed, and `import/*` walks ONE document into the Schattensystem by type.
// Partner, Wagen and Radsätze are one page with the kind in route `data`. All
// steps are guarded in `intake.guards.ts`.
//
// `master-file` is the customer's master workbook, chosen where it lies. It is
// the only master: `master/*` below reads it.
//
// `master/export/*` is the third walk, „In Master übertragen“: one filed
// document into the client master — sheets, column check, preview to approve,
// result — started from the document list, written into the customer's file
// after a backup.
//
// The entry load hangs on a PATHLESS parent so every route below it shares one
// resolver run. Any of the six can be the first one reached, and the alternative
// is the same `load()` in six page constructors.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';
import {
  batchSummaryGuard,
  cleaningGuard,
  importResultGuard,
  importWalkGuard,
  masterExportGuard,
  masterExportPreviewGuard,
  masterExportResultGuard,
} from './intake.guards';
import { trainsDataResolver } from './trains-data.resolver';

const pages: Routes = [
  {
    path: '',
    pathMatch: 'full',
    title: 'Dokumente',
    loadComponent: () =>
      import('../feature/start/start.page').then((m) => m.TrainsStartPage),
  },
  {
    path: 'erp',
    title: 'ERP',
    loadComponent: () =>
      import('../feature/dashboard/dashboard.page').then(
        (m) => m.TrainsDashboardPage
      ),
  },
  {
    path: 'clean',
    children: [
      {
        path: '',
        pathMatch: 'full',
        title: 'Bereinigen',
        loadComponent: () =>
          import('../feature/clean-hub/clean-hub.page').then(
            (m) => m.CleanHubPage
          ),
      },
      {
        path: 'file',
        title: 'Datei bereinigen',
        canActivate: [cleaningGuard],
        loadComponent: () =>
          import('../feature/clean-file/clean-file.page').then(
            (m) => m.CleanFilePage
          ),
      },
      {
        path: 'summary',
        title: 'Bereinigung abgeschlossen',
        canActivate: [batchSummaryGuard],
        loadComponent: () =>
          import('../feature/clean-batch/clean-batch.page').then(
            (m) => m.CleanBatchPage
          ),
      },
      {
        path: 'template',
        title: 'Vorlage anlegen',
        loadComponent: () =>
          import('../feature/template-mapper/template-mapper.page').then(
            (m) => m.TemplateMapperPage
          ),
      },
    ],
  },
  {
    path: 'documents',
    title: 'Dokumente',
    loadComponent: () =>
      import('../feature/document-list/document-list.page').then(
        (m) => m.DocumentListPage
      ),
  },
  {
    path: 'import',
    children: [
      { path: '', pathMatch: 'full', redirectTo: '/trains/documents' },
      {
        path: 'partners',
        title: 'Partner prüfen',
        data: { kind: 'partner' },
        canActivate: [importWalkGuard],
        loadComponent: () =>
          import('../feature/import-entities/import-entities.page').then(
            (m) => m.ImportEntitiesPage
          ),
      },
      {
        path: 'wagons',
        title: 'Wagen prüfen',
        data: { kind: 'wagen' },
        canActivate: [importWalkGuard],
        loadComponent: () =>
          import('../feature/import-entities/import-entities.page').then(
            (m) => m.ImportEntitiesPage
          ),
      },
      {
        path: 'wheelsets',
        title: 'Radsätze prüfen',
        data: { kind: 'radsatz' },
        canActivate: [importWalkGuard],
        loadComponent: () =>
          import('../feature/import-entities/import-entities.page').then(
            (m) => m.ImportEntitiesPage
          ),
      },
      {
        path: 'entries',
        title: 'Einträge prüfen',
        canActivate: [importWalkGuard],
        loadComponent: () =>
          import('../feature/import-entries/import-entries.page').then(
            (m) => m.ImportEntriesPage
          ),
      },
      {
        path: 'summary',
        title: 'Zusammenfassung',
        canActivate: [importWalkGuard],
        loadComponent: () =>
          import('../feature/import-summary/import-summary.page').then(
            (m) => m.ImportSummaryPage
          ),
      },
      {
        path: 'result',
        title: 'Import abgeschlossen',
        canActivate: [importResultGuard],
        loadComponent: () =>
          import('../feature/import-result/import-result.page').then(
            (m) => m.ImportResultPage
          ),
      },
    ],
  },
  {
    path: 'master-file',
    title: 'Master-Datei',
    loadComponent: () =>
      import('../feature/master-file/master-file.page').then(
        (m) => m.MasterFilePage
      ),
  },
  {
    path: 'master',
    title: 'Master-Import',
    loadComponent: () =>
      import('../feature/master/master.page').then((m) => m.TrainsMasterPage),
  },
  {
    path: 'master/export',
    children: [
      { path: '', pathMatch: 'full', redirectTo: '/trains/documents' },
      {
        path: 'sheets',
        title: 'Blätter wählen',
        canActivate: [masterExportGuard],
        loadComponent: () =>
          import('../master/feature/export-sheets/export-sheets.page').then(
            (m) => m.ExportSheetsPage
          ),
      },
      {
        path: 'structure',
        title: 'Spalten abgleichen',
        canActivate: [masterExportPreviewGuard],
        loadComponent: () =>
          import('../master/feature/export-structure/export-structure.page').then(
            (m) => m.ExportStructurePage
          ),
      },
      {
        path: 'preview',
        title: 'Vorschau',
        canActivate: [masterExportPreviewGuard],
        loadComponent: () =>
          import('../master/feature/export-preview/export-preview.page').then(
            (m) => m.ExportPreviewPage
          ),
      },
      {
        path: 'result',
        title: 'Master aktualisiert',
        canActivate: [masterExportResultGuard],
        loadComponent: () =>
          import('../master/feature/export-result/export-result.page').then(
            (m) => m.ExportResultPage
          ),
      },
    ],
  },
  {
    path: 'master/sheets/:sheet',
    title: 'Master-Blatt',
    loadComponent: () =>
      import('../master/feature/master-sheet/master-sheet.page').then(
        (m) => m.MasterSheetPage
      ),
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
    path: 'telematik',
    title: 'Telematik',
    loadComponent: () =>
      import('../feature/telematik-list/telematik-list.page').then(
        (m) => m.TelematikListPage
      ),
  },
  {
    path: 'wagen/:id',
    title: 'Wagen',
    data: { kind: 'wagen' },
    loadComponent: () =>
      import('../feature/entity-detail/entity-detail.page').then(
        (m) => m.EntityDetailPage
      ),
  },
  {
    path: 'radsaetze/:id',
    title: 'Radsatz',
    data: { kind: 'radsatz' },
    loadComponent: () =>
      import('../feature/entity-detail/entity-detail.page').then(
        (m) => m.EntityDetailPage
      ),
  },
  {
    path: 'partner/:id',
    title: 'Partner',
    data: { kind: 'partner' },
    loadComponent: () =>
      import('../feature/entity-detail/entity-detail.page').then(
        (m) => m.EntityDetailPage
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
    path: 'settings',
    title: 'Einstellungen',
    loadComponent: () =>
      import('../feature/settings/settings.page').then(
        (m) => m.TrainsSettingsPage
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
