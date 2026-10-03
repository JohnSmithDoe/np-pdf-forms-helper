// ─── why ────────────────────────────────────────────────────────
// The guided import's steps are routes, so each is reachable by URL and by the
// back gesture; these make the illegal ones unreachable. Same shape as filler's
// `wizard.guards.ts`, for the same reason: a guard NEVER loads. Guards run a
// whole phase before the parent's resolver, so a predicate over the store plus
// a redirect is the only thing that can answer in time — and `loaded` is part
// of every predicate, so a cold deep link lands on the hub.
//
// The hub is the first step and deliberately UNGUARDED; a guard on it would
// have nowhere to redirect but itself.
//
// Each step guards on the slot it renders. The preview additionally needs a
// file being WALKED, so a staging left behind by the manual import cannot be
// committed from the guided preview as if it were the cleaned copy.
// ────────────────────────────────────────────────────────────────

import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { IntakeFacade } from '../data';

const HUB = '/trains/import';

function stepGuard(reached: (facade: IntakeFacade) => boolean): CanActivateFn {
  return (): true | UrlTree => {
    const facade = inject(IntakeFacade);
    const router = inject(Router);
    return (facade.loaded() && reached(facade)) || router.parseUrl(HUB);
  };
}

export const cleaningGuard = stepGuard(
  (facade) => facade.cleaning() !== undefined
);

export const guidedPreviewGuard = stepGuard((facade) =>
  facade.inGuidedPreview()
);

export const guidedResultGuard = stepGuard(
  (facade) => facade.commitReport() !== null
);
