import {Component} from '@angular/core';
import {MatIconRegistry} from '@angular/material/icon';
import {APP_CONFIG} from '../environments/environment';
import {AppService} from './services/app.service';
import {ElectronService} from './services/electron.service';

@Component({
             selector:    'app-root',
             templateUrl: './app.component.html',
             styleUrls:   ['./app.component.scss']
           })
export class AppComponent {

  constructor(
    private readonly electronService: ElectronService,
    private readonly iconService: MatIconRegistry,
    public readonly appService: AppService,
  ) {
    this.iconService.setDefaultFontSetClass('material-icons-sharp');
    console.log('APP_CONFIG', APP_CONFIG);

    if (electronService.isElectron) {
      console.log('Run in electron');
    } else {
      console.log('Run in browser');
    }
  }
}
