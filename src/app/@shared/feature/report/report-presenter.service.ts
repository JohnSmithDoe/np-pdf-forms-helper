// ─── why ────────────────────────────────────────────────────────
// "A report is a toast unless it needs the user", applied. `needsDialog` decides
// which; this owns the two ways of showing it.
//
// It exists because there are ten pages now rather than one. The rule used to
// live in `FillerPage` as a private method, which was right while that page was
// the only subscriber to `report$` — every wizard step that can fail would
// otherwise carry its own copy, and ten copies of a routing rule drift.
//
// It RETURNS the folder the user asked to open instead of opening it. `@shared`
// may not reach a domain's facade, and opening a folder is a backend command
// belonging to whichever domain produced the report — so the decision is here
// and the action stays with the caller.
//
// A failure is the same rule in `danger`: a single line is still a toast, and a
// multi-line failure still earns the dialog it would have got as a success.
//
// It is a service in the FEATURE layer, which is where the ladder puts it rather
// than a preference: it composes `ReportDialog`, and `type:data` may not reach
// `type:smart-ui`. Composing an overlay is a feature-layer job whether the thing
// doing it is a component or not.
//
// `run` is here rather than in `@shared/util` for the same ceiling: the helper
// has to NAME `BackendError`, which lives in `data`, and `type:util` may not
// reach it. It answers `false` instead of rethrowing so a caller can say "and
// only then navigate" without a second try/catch — which is what most of the
// wizard steps need. Anything that is NOT a `BackendError` is a renderer fault
// rather than a backend answer, so it propagates untouched.
//
// The `smart-ui` leaves keep their own copy of the same shape deliberately: they
// EMIT the failure instead of presenting it, and injecting a presenter into a
// strict leaf is what the ladder is stopping.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import { ModalController } from '@ionic/angular/standalone';
import { BackendError } from '../../data/backend/backend.service';
import { ToastService } from '../../data/toast/toast.service';
import type { ClientReport } from '../../model/client.types';
import type { ToastColor } from '../../model/toast.types';
import { ReportDialog } from '../../smart-ui/report-dialog/report.dialog';
import { needsDialog } from '../../util/report.util';

const FAILURE_HEADLINE = 'Es ist ein Problem aufgetreten';

@Injectable({ providedIn: 'root' })
export class ReportPresenterService {
  readonly #modals = inject(ModalController);
  readonly #toasts = inject(ToastService);

  async show(
    report: ClientReport,
    color: ToastColor = 'success'
  ): Promise<string | undefined> {
    if (needsDialog(report)) return await this.#dialog(report);
    await this.#toasts.show({
      header: report.headline,
      message: report.messages[0],
      color,
    });
    return undefined;
  }

  async showError(error: BackendError): Promise<string | undefined> {
    return await this.show(
      { headline: FAILURE_HEADLINE, messages: error.messages },
      'danger'
    );
  }

  async run(action: () => Promise<void>): Promise<boolean> {
    try {
      await action();
      return true;
    } catch (error) {
      if (!(error instanceof BackendError)) throw error;
      await this.showError(error);
      return false;
    }
  }

  async #dialog(report: ClientReport): Promise<string | undefined> {
    const modal = await this.#modals.create({
      component: ReportDialog,
      componentProps: { report },
    });
    await modal.present();

    const { data, role } = await modal.onWillDismiss<string>();
    return role === 'folder' && data ? data : undefined;
  }
}
