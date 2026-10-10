// ─── why ────────────────────────────────────────────────────────
// The wizards' steps are routes, so every step is reachable by URL and by the
// back gesture. These are what make the illegal ones unreachable.
//
// A guard here NEVER loads. It is a pure predicate over the store plus a
// redirect, which is the only shape that works: guards run a whole phase before
// the parent's resolver, so a guard that waited for data would be waiting for
// something the router has not started yet. `loaded` is therefore part of every
// predicate — it distinguishes an empty install from an unanswered backend, and
// both correctly send the user back to step one.
//
// The first step of each wizard is deliberately UNGUARDED. A guard on it would
// have nowhere to redirect but itself, and a cold deep link would bounce
// forever.
//
// The two result steps guard on their OWN report slice. One shared slice would
// let a finished setup satisfy the export wizard and render "3 Dokument(e)
// wurden hinzugefügt" under the heading "Ergebnis".
// ────────────────────────────────────────────────────────────────

import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { FillerFacade } from '../data';

const SETUP_START = '/documents/setup/source';
const EXPORT_START = '/documents/wizard/selection';

function stepGuard(
  reached: (facade: FillerFacade) => boolean,
  fallback: string
): CanActivateFn {
  return (): true | UrlTree => {
    const facade = inject(FillerFacade);
    const router = inject(Router);
    return (facade.loaded() && reached(facade)) || router.parseUrl(fallback);
  };
}

export const importedDocumentsGuard = stepGuard(
  (facade) => facade.importedDocuments().length > 0,
  SETUP_START
);

export const setupReportGuard = stepGuard(
  (facade) => facade.setupReport() !== undefined,
  SETUP_START
);

export const exportFieldsGuard = stepGuard(
  (facade) => facade.hasExportFields(),
  EXPORT_START
);

export const runReportGuard = stepGuard(
  (facade) => facade.runReport() !== undefined,
  EXPORT_START
);
