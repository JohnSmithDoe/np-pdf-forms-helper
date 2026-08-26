// ─── why ────────────────────────────────────────────────────────
// The shell reaches a domain only through its own routes manifest and imports
// nothing else from it, so a domain stays replaceable from here.
//
// Three domains — `filler` fills documents, `trains` imports maintenance data,
// `info` states what the program is — and they never reach into each other.
// `@shared` is the only other unit, and it is never routed to.
//
// `filler` is mounted at `documents` and not at `filler`: the path is what the
// user sees in a wizard's URL, and it names the thing rather than the code. It
// resolves further to the expert page or to the wizard depending on the stored
// view mode, which is `filler.routes.ts`'s business and not the shell's.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: 'documents',
    loadChildren: () =>
      import('./filler/routes/filler.routes').then((m) => m.fillerRoutes),
  },
  {
    path: 'trains',
    loadChildren: () =>
      import('./trains/routes/trains.routes').then((m) => m.trainsRoutes),
  },
  {
    path: 'about',
    loadChildren: () =>
      import('./info/routes/info.routes').then((m) => m.infoRoutes),
  },
  {
    path: '**',
    redirectTo: 'documents',
  },
];
