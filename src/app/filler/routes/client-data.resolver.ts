// ─── why ────────────────────────────────────────────────────────
// The one load, hung on the `documents` parent route so it happens once per
// entry into the tree rather than in whichever page got there first. Three pages
// can now be the first one — the expert page, either wizard — and a load in a
// page constructor would have to be repeated in all of them.
//
// It does NOT run before the step guards. Angular's transition pipeline is
// `checkGuards` → `resolveData` → `loadComponents`: guards for the whole tree
// run in one phase, resolvers in the next, so nesting cannot get this in front
// of a child's guard. That is deliberate rather than tolerated — the guards read
// `loaded`, and a cold deep link into a later step has no valid selection to
// return to anyway, because selection and typed values are memory-only.
//
// It SWALLOWS the failure instead of rejecting. A rejected resolver cancels the
// navigation, which leaves an empty window and no way to find out why; the
// commonest failure here is `pnpm start` with no Tauri underneath, whose whole
// point is the German sentence saying so. `ReportPresenterService.run` gives it
// the same treatment every other caller gets — `needsDialog` picks toast or
// dialog — rather than a second hand-rolled toast with its own copy of the
// headline.
//
// Loading only when `loaded` is false is not caching for its own sake: the
// backend echoes the full state onto every mutation, so a second read can only
// return what the store already holds, and re-entering from `/about` would
// otherwise re-fetch and repaint for nothing.
// ────────────────────────────────────────────────────────────────

import { inject } from '@angular/core';
import { ResolveFn } from '@angular/router';
import { ReportPresenterService } from '../../@shared/feature/report/report-presenter.service';
import { FillerFacade } from '../data';

export const clientDataResolver: ResolveFn<boolean> = async () => {
  const facade = inject(FillerFacade);
  const reports = inject(ReportPresenterService);
  if (facade.loaded()) return true;

  return await reports.run(() => facade.load());
};
