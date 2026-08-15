import {CommonModule} from '@angular/common';
import {NgModule} from '@angular/core';
import {FormsModule} from '@angular/forms';
import {MatAutocompleteModule} from '@angular/material/autocomplete';
import {MatButtonModule} from '@angular/material/button';
import {MatCheckboxModule} from '@angular/material/checkbox';
import {MatDialogModule} from '@angular/material/dialog';
import {MatFormFieldModule} from '@angular/material/form-field';
import {MatGridListModule} from '@angular/material/grid-list';
import {MatIconModule} from '@angular/material/icon';
import {MatInputModule} from '@angular/material/input';
import {MatSelectModule} from '@angular/material/select';
import {MatTooltipModule} from '@angular/material/tooltip';
import {MatTreeModule} from '@angular/material/tree';
import {ModalComponent} from './components/modal/modal.component';

import {PageNotFoundComponent} from './components/page-not-found/page-not-found.component';
import {FieldDialogComponent} from './dialogs/add-field/field-dialog.component';
import {AddProfileDialogComponent} from './dialogs/add-profile/add-profile-dialog.component';
import {ConfirmDialogComponent} from './dialogs/confirm/confirm-dialog.component';
import {MessageDialogComponent} from './dialogs/message/message-dialog.component';

@NgModule({
            declarations: [
              PageNotFoundComponent,
              FieldDialogComponent,
              ConfirmDialogComponent,
              MessageDialogComponent,
              AddProfileDialogComponent,
              ModalComponent
            ],
            imports: [
              CommonModule,
              FormsModule,
              MatDialogModule,
              MatFormFieldModule,
              MatSelectModule,
              MatInputModule,
              MatButtonModule,
              MatCheckboxModule,
              MatTooltipModule,
              MatTreeModule,
              MatIconModule,
              MatGridListModule,
              MatAutocompleteModule,
            ],
            exports:      [
              FormsModule,
              FieldDialogComponent,
              ConfirmDialogComponent,
              MessageDialogComponent,
              AddProfileDialogComponent,
              ModalComponent
            ]
          })
export class SharedModule {}
