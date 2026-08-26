// ─── why ────────────────────────────────────────────────────────
// The domain's own manifest, so the shell reaches info through one import and
// nothing else. One route today; it stays a manifest because that is the seam
// every other domain is reached through.
// ────────────────────────────────────────────────────────────────

import { Routes } from '@angular/router';

export const infoRoutes: Routes = [
  {
    path: '',
    title: 'Info',
    loadComponent: () =>
      import('../feature/info-page/info.page').then((m) => m.InfoPage),
  },
];
