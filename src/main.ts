import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { provideAppKernel } from './app/app.providers';

try {
  await bootstrapApplication(AppComponent, {
    providers: provideAppKernel(),
  });
} catch (error) {
  console.error(error);
}
