// The app ships ONE icon font — `material-icons/iconfont/sharp.css`, wired in
// angular.json's `styles`. The Angular 13 app imported the full
// `material-icons.css` (filled, outlined, round, sharp, two-tone = five woff2)
// and then rendered only sharp. Registering sharp as the DEFAULT font set is
// what lets every <mat-icon> stay bare, so no template carries a `fontSet`.
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { MatIconModule, MatIconRegistry } from '@angular/material/icon';
import { MatToolbarModule } from '@angular/material/toolbar';
import { RouterOutlet } from '@angular/router';
import { APP_WORDMARK } from './@shared/model/app.consts';

@Component({
  selector: 'app-root',
  templateUrl: 'app.component.html',
  styleUrl: 'app.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatIconModule, MatToolbarModule, RouterOutlet],
})
export class AppComponent {
  readonly #icons = inject(MatIconRegistry);

  protected readonly wordmark = APP_WORDMARK;

  constructor() {
    this.#icons.setDefaultFontSetClass('material-icons-sharp');
  }
}
