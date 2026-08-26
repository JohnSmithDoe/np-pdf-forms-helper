// ─── why ────────────────────────────────────────────────────────
// The kernel, factored out of main.ts so bootstrap stays one call and a
// provider is added HERE, where its reason is next to it.
//
// `withHashLocation` is not a preference: the packaged app is served to a
// webview over a custom protocol, where a path-based deep link is resolved by
// the asset handler rather than by a router-aware server and 404s. The dev
// server would work either way, which is exactly how this breaks only in the
// packaged build.
//
// No animations provider — `provideAnimations`/`provideAnimationsAsync` are
// deprecated as of Angular 21, and Ionic drives its own transitions through the
// Web Animations API rather than through Angular's.
//
// `useSetInputAPI` is not a preference and not cosmetic. Ionic hands a modal's
// `componentProps` to the component one of two ways: with this flag it calls
// `ComponentRef.setInput()`, and without it it ASSIGNS onto the instance. This
// app is signals-first, so every modal declares its props as `input()` — and an
// assignment onto an `InputSignal` property does nothing at all, silently. The
// dialogs would open blank. It is `?? false` inside Ionic's AngularDelegate, so
// it has to be said here.
//
// Its one cost, worth knowing before it is met: with the flag on, passing a
// `componentProps` key that is NOT a declared input throws NG0303 instead of
// being quietly dropped. That is the trade — loud on a typo, correct on a signal.
//
// `mode: 'md'` pins Material Design rendering on every platform. Ionic would
// otherwise pick `ios` when it thinks it is on one, and this is a desktop tool
// whose one shell is a Chromium webview — the platform sniff has nothing to say
// about it, and a build that renders differently on the maintainer's Mac than
// on the Windows target is a bug generator.
//
// There is no backend provider: `BackendService` is `providedIn: 'root'` now
// that there is one shell to talk to.
// ────────────────────────────────────────────────────────────────

import {
  EnvironmentProviders,
  Provider,
  provideBrowserGlobalErrorListeners,
  provideZonelessChangeDetection,
} from '@angular/core';
import {
  provideRouter,
  RouteReuseStrategy,
  withHashLocation,
} from '@angular/router';
import {
  IonicRouteStrategy,
  provideIonicAngular,
} from '@ionic/angular/standalone';
import { routes } from './app.routes';

export function provideAppKernel(): Array<Provider | EnvironmentProviders> {
  return [
    provideBrowserGlobalErrorListeners(),
    provideZonelessChangeDetection(),
    provideRouter(routes, withHashLocation()),
    { provide: RouteReuseStrategy, useClass: IonicRouteStrategy },
    provideIonicAngular({ mode: 'md', useSetInputAPI: true }),
  ];
}
