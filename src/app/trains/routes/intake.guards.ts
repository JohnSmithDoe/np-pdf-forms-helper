// ─── why ────────────────────────────────────────────────────────
// The steps of both walks are routes, so each is reachable by URL and by the
// back gesture; these make the illegal ones unreachable. Same shape as filler's
// `wizard.guards.ts`, for the same reason: a guard NEVER loads. Guards run a
// whole phase before the parent's resolver, so a predicate over the store plus
// a redirect is the only thing that can answer in time — and `loaded` is part
// of every predicate, so a cold deep link lands on the walk's way in.
//
// The cleaning hub and the document list are those ways in and deliberately
// UNGUARDED; a guard on either would have nowhere to redirect but itself.
//
// Each step guards on the slot it renders. The import steps need a STAGED
// DOCUMENT, not merely a staging: the template mapper holds one too, and its
// staging is not a document anybody may import. The result step needs only
// the parked report, because the commit has already let go of the staging.
// ────────────────────────────────────────────────────────────────

import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { ImportWalkFacade, IntakeFacade } from '../data';

function guard<T>(
  facade: new (...args: never[]) => T,
  loaded: (facade: T) => boolean,
  reached: (facade: T) => boolean,
  redirect: string
): CanActivateFn {
  return (): true | UrlTree => {
    const instance = inject(facade);
    const router = inject(Router);
    return (loaded(instance) && reached(instance)) || router.parseUrl(redirect);
  };
}

const CLEAN = '/trains/clean';
const DOCUMENTS = '/trains/documents';

export const cleaningGuard = guard(
  IntakeFacade,
  (facade) => facade.loaded(),
  (facade) => facade.cleaning() !== undefined,
  CLEAN
);

export const batchSummaryGuard = guard(
  IntakeFacade,
  (facade) => facade.loaded(),
  (facade) => facade.scan() !== undefined,
  CLEAN
);

export const importWalkGuard = guard(
  ImportWalkFacade,
  (facade) => facade.loaded(),
  (facade) => facade.walking(),
  DOCUMENTS
);

export const importResultGuard = guard(
  ImportWalkFacade,
  (facade) => facade.loaded(),
  (facade) => facade.report() !== null,
  DOCUMENTS
);
