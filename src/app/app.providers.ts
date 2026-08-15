// ─── why ────────────────────────────────────────────────────────
// The kernel, factored out of main.ts so bootstrap stays one call and a
// provider is added HERE, where its reason is next to it.
//
// `withHashLocation` is not a preference: the packaged app is loaded by
// Electron over `file://`, where a path-based deep link resolves against the
// filesystem and 404s. The dev server would work either way, which is exactly
// how this breaks only in the packaged build.
//
// No animations provider — `provideAnimations`/`provideAnimationsAsync` are
// deprecated as of Angular 21 and Material 21 drives its own transitions from
// CSS.
// ────────────────────────────────────────────────────────────────

import {
  EnvironmentProviders,
  Provider,
  provideBrowserGlobalErrorListeners,
  provideZonelessChangeDetection,
} from '@angular/core';
import { MAT_FORM_FIELD_DEFAULT_OPTIONS } from '@angular/material/form-field';
import { provideRouter, withHashLocation } from '@angular/router';
import { routes } from './app.routes';

export function provideAppKernel(): Array<Provider | EnvironmentProviders> {
  return [
    provideBrowserGlobalErrorListeners(),
    provideZonelessChangeDetection(),
    provideRouter(routes, withHashLocation()),
    {
      provide: MAT_FORM_FIELD_DEFAULT_OPTIONS,
      useValue: { appearance: 'outline', floatLabel: 'always' },
    },
  ];
}
