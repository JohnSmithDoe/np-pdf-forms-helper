import { ChangeDetectionStrategy, Component } from '@angular/core';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-page-filler',
  templateUrl: 'filler.page.html',
  styleUrls: ['filler.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [MatCardModule],
})
export class FillerPage {}
