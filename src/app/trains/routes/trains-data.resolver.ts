// ─── why ────────────────────────────────────────────────────────
// The one entry load, hung on the `trains` parent route so it happens once per
// entry into the tree rather than in whichever page got there first. Six routes
// share the domain and any of them can be the first one reached, so a load in a
// page constructor had to be repeated in every page — and was, five times, none
// of which handled its own failure.
//
// It SWALLOWS the failure instead of rejecting, for the reason
// `filler/routes/client-data.resolver.ts` records: a rejected resolver cancels
// the navigation, which leaves an empty window and no way to find out why. The
// commonest failure is `pnpm start` with no Tauri underneath, whose whole point
// is the German sentence saying so.
//
// Loading only when `loaded` is false is not caching for its own sake: the
// backend echoes the lists back on every mutation, so a second read can only
// return what the store already holds. Hopping between the five sibling lists
// would otherwise re-send every wagen and partner each time.
// ────────────────────────────────────────────────────────────────

import { inject } from '@angular/core';
import { ResolveFn } from '@angular/router';
import { BackendError } from '../../@shared/data/backend/backend.service';
import { ToastService } from '../../@shared/data/toast/toast.service';
import { TrainsFacade } from '../data';

export const trainsDataResolver: ResolveFn<boolean> = async () => {
  const facade = inject(TrainsFacade);
  const toasts = inject(ToastService);
  if (facade.loaded()) return true;

  try {
    await facade.load();
    return true;
  } catch (cause) {
    const messages =
      cause instanceof BackendError
        ? cause.messages
        : ['Es ist ein unbekannter Fehler aufgetreten.'];
    await toasts.show({
      header: 'Es ist ein Problem aufgetreten',
      message: messages[0],
      color: 'danger',
    });
    return false;
  }
};
