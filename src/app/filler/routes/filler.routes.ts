import { Routes } from '@angular/router';

export const fillerRoutes: Routes = [
  {
    path: '',
    title: 'Dokumente ausfüllen',
    loadComponent: () =>
      import('../feature/filler-page/filler.page').then((m) => m.FillerPage),
  },
];
