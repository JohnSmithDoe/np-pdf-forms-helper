import {NgModule} from '@angular/core';
import {MAT_FORM_FIELD_DEFAULT_OPTIONS} from '@angular/material/form-field';
import {BrowserModule} from '@angular/platform-browser';
import {BrowserAnimationsModule} from '@angular/platform-browser/animations';

import {AppRoutingModule} from './app-routing.module';

import {AppComponent} from './app.component';

import {HomeModule} from './home/home.module';
import {AppService} from './services/app.service';
import {ElectronService} from './services/electron.service';
import {SharedModule} from './shared/shared.module';

// noinspection JSUnusedGlobalSymbols
@NgModule({
            declarations: [AppComponent],
            imports:      [
              BrowserModule,
              BrowserAnimationsModule,
              HomeModule,
              AppRoutingModule,
              SharedModule,
            ],
            providers:    [
              {provide: MAT_FORM_FIELD_DEFAULT_OPTIONS, useValue: {appearance: 'outline', floatLabel: 'always'}},
              ElectronService,
              AppService,
            ],
            bootstrap:    [AppComponent]
          })
export class AppModule {}
