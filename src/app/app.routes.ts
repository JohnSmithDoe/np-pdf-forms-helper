// ─── why ────────────────────────────────────────────────────────
// The shell reaches a domain only through its own routes manifest and imports
// nothing else from it, so a domain stays replaceable from here.
//
// One domain — `filler` — is the whole app: a single workflow, not a deck of
// them. `@shared` is the only other unit, and it is never routed to.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: 'filler',
    loadChildren: () =>
      import('./filler/routes/filler.routes').then((m) => m.fillerRoutes),
  },
  {
    path: '**',
    redirectTo: 'filler',
  },
];
