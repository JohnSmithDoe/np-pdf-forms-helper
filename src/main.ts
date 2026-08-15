import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { provideAppKernel } from './app/app.providers';

void bootstrapApplication(AppComponent, {
  providers: provideAppKernel(),
}).catch((error: unknown) => console.error(error));
