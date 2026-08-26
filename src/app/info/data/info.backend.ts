// ─── why ────────────────────────────────────────────────────────
// One command, so one file. It sits in the domain rather than in `@shared` for
// the same reason `FillerBackend` does: `@shared/data/backend` carries the
// transport and knows no command names.
//
// `app_info` answers with a bare `AppInfo` and not a `ClientData`, so it can
// carry no report — which is correct. Asking what version is running is a
// question, and the transport's report channel is for things a command wants to
// SAY on its way past.
//
// The `& BackendResponse` is what says that in the type system rather than a
// workaround for it: `call` constrains its result to something that MAY carry a
// report, and TypeScript rejects a type sharing no property with an all-optional
// one. The intersection widens `AppInfo` by the one optional key the transport
// looks for and nothing else, so no cast is needed anywhere.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import {
  BackendResponse,
  BackendService,
} from '../../@shared/data/backend/backend.service';
import type { AppInfo } from '../model/info.types';

@Injectable({ providedIn: 'root' })
export class InfoBackend {
  readonly #backend = inject(BackendService);

  appInfo(): Promise<AppInfo> {
    return this.#backend.call<AppInfo & BackendResponse>({
      command: 'app_info',
      payload: {},
    });
  }
}
