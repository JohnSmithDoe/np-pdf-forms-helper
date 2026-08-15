import {NgModule} from '@angular/core';
import {RouterModule, Routes} from '@angular/router';

import {HomeRoutingModule} from './home/home-routing.module';
import {PageNotFoundComponent} from './shared/components/page-not-found/page-not-found.component';

const routes: Routes = [
  {
    path:       '',
    redirectTo: 'home',
    pathMatch:  'full'
  },
  {
    path:      '**',
    component: PageNotFoundComponent
  }
];

@NgModule({
            imports: [
              RouterModule.forRoot(routes, {
                enableTracing: false,
                useHash: false,
                relativeLinkResolution: 'legacy',
                onSameUrlNavigation: 'ignore'
              }),
              HomeRoutingModule,
            ],
            exports: [RouterModule]
          })
export class AppRoutingModule {}
