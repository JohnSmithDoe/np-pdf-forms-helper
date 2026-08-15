import {CommonModule} from '@angular/common';
import {NgModule} from '@angular/core';
import {MatButtonModule} from '@angular/material/button';
import {MatCardModule} from '@angular/material/card';
import {MatCheckboxModule} from '@angular/material/checkbox';
import {MatDividerModule} from '@angular/material/divider';
import {MatExpansionModule} from '@angular/material/expansion';
import {MatFormFieldModule} from '@angular/material/form-field';
import {MatIconModule} from '@angular/material/icon';
import {MatInputModule} from '@angular/material/input';
import {MatListModule} from '@angular/material/list';
import {MatSelectModule} from '@angular/material/select';
import {MatSnackBarModule} from '@angular/material/snack-bar';
import {MatToolbarModule} from '@angular/material/toolbar';
import {MatTooltipModule} from '@angular/material/tooltip';
import {MatTreeModule} from '@angular/material/tree';
import {SharedModule} from '../shared/shared.module';

import {HomeRoutingModule} from './home-routing.module';

import {HomeComponent} from './home.component';

@NgModule({
            declarations: [
              HomeComponent,
            ],
            imports: [
              CommonModule,
              SharedModule,
              HomeRoutingModule,
              MatCardModule,
              MatDividerModule,
              MatListModule,
              MatIconModule,
              MatFormFieldModule,
              MatTreeModule,
              MatButtonModule,
              MatTooltipModule,
              MatInputModule,
              MatSnackBarModule,
              MatExpansionModule,
              MatCheckboxModule,
              MatSelectModule,
              MatToolbarModule,
            ],

          })
export class HomeModule {}
