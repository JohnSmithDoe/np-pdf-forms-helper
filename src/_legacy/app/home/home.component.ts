import {Component, NgZone, OnDestroy, OnInit} from '@angular/core';
import {MatDialog} from '@angular/material/dialog';
import {MessageDialogComponent} from 'app/shared/dialogs/message/message-dialog.component';
import {Subscription} from 'rxjs';
import {
  IClientData,
  IMappedDocument,
  IMappedField,
  IMappedInput,
  IPdfDocument,
  IProfile,
  IXlsxDocument, TTemplateType
} from '../../../bridge/shared.model';
import {APP_CONFIG} from '../../environments/environment';
import {AppService} from '../services/app.service';
import {ElectronService} from '../services/electron.service';
import {FieldDialogComponent} from '../shared/dialogs/add-field/field-dialog.component';
import {AddProfileDialogComponent} from '../shared/dialogs/add-profile/add-profile-dialog.component';
import {ConfirmDialogComponent} from '../shared/dialogs/confirm/confirm-dialog.component';

export type TUIExportInput = IMappedInput & { mappedName: string, info: string };
export type TUIDocumentField = (IMappedField & { export: boolean })
export type TUIDocument = (Omit<IMappedDocument, 'mapped'> & { export: boolean, mapped: TUIDocumentField[] })

@Component({
  selector: 'app-home',
  templateUrl: './home.component.html',
  styleUrls: ['./home.component.scss']
})
export class HomeComponent implements OnInit, OnDestroy {

  dataSource: TUIDocument[] = [];
  export = {
    msgs: [] as string[],
    documents: [] as TUIDocument[],
    fields: [] as TUIExportInput[],
    folder: '',
    suffix: '',
  }
  currentSort: 'aufsteigend' | 'absteigend' = 'aufsteigend';

  profileId = '';
  profiles: IProfile[] = [];

  options = {
    visible: false,
    autoMapFields: true
  }

  private dialogSub: Subscription;
  private subs: Subscription[] = [];

  private changedName = false;

  constructor(
    private readonly electronService: ElectronService,
    private readonly appService: AppService,
    private readonly dialog: MatDialog,
    private readonly ngZone: NgZone
  ) {
    this.subs.push(
      // on return messages from electron call
      electronService.report$.subscribe((data) => {
        console.log('43: report');
        this.ngZone.runTask(() => {
          this.dialog.open(MessageDialogComponent,
            {
              data: {
                headline: data.headline,
                msgs: data.messages,
                folder: data.messageFolder
              },
              autoFocus: 'dialog',
              hasBackdrop: true,
              restoreFocus: true
            });
        });
      }),
      electronService.finishedLoading$.subscribe((data: IClientData) => {
        console.log('58: finish load');
        // finished load gets initiated by the main process so we need to get it into the zone
        this.ngZone.runTask(() => {
          console.log('61: really finished load');
          this.updateDataSource(data.documents);
          this.profiles = data.profiles || [];
          if (data.message) {
            electronService.report$.emit(data.message)
          }
        });
      }),
      electronService.update$.subscribe((data: IClientData) => {
        console.log('65: update');
        this.updateDataSource(data.documents);
        this.updateProfiles(data.profiles);
      }),

      // on start electron call
      electronService.startRequest$.subscribe((channel) => {
        console.log('70: start');
        this.appService.modal$.next(this.electronService.isElectron);
      }),
      // on end electron call
      electronService.stopRequest$.subscribe((channel) => {
        console.log('75: stop');
        this.appService.modal$.next(false);
      }),
      // on error
      electronService.error$.subscribe((err) => {
        this.dialog.open(MessageDialogComponent,
          {
            data: {
              headline: 'Es ist ein Problem aufgetreten',
              msgs: err,
              folder: undefined
            },
            disableClose: true,
            autoFocus: 'dialog',
            hasBackdrop: true,
            panelClass: 'error-panel'
          });
      })
    );
  }


  private updateDataSource(newData: IMappedDocument[]) {

    newData.map(newDoc => {
      const mappedFields = newDoc.mapped?.map(field => ({
        ...field,
        export: !!this.export.fields.find(exp => exp.identifiers.includes(field.origId)),
      })) ?? []
      const origin = this.dataSource.find(doc => doc.id === newDoc.id);
      if (origin) {
        origin.mapped = mappedFields;
        origin.name = newDoc.name;
        origin.filename = newDoc.filename;
        origin.mtime = newDoc.mtime;
        // TODO: hmm not very nice
        if ((origin as unknown as IPdfDocument).fields) {
          (origin as unknown as IPdfDocument).fields = (newDoc as unknown as IPdfDocument).fields;
          (origin as unknown as IPdfDocument).previewfile = (newDoc as unknown as IPdfDocument).previewfile;
        }

      } else {
        this.dataSource.push(
          {
            ...newDoc,
            export: !!this.export.documents.find(exp => exp.id === newDoc.id),
            mapped: mappedFields
          });
      }
    })
    if (this.dataSource?.length) {
      this.sortDocuments();
      this.updateExport();
    }
  }

  private findTemplateWithMappedOriginFieldId(origId: string) {
    return this.dataSource.find(document => (document.mapped || []).find((field: IMappedField) => field.origId === origId));
  }


  updateExport() {
    this.export.documents = this.dataSource.filter(doc => doc.export);
    this.export.folder = (Date.now().toString()) + (!!this.export.suffix ? '-' + this.export.suffix : '');
    this.export.msgs = this.export.documents.map(document => `<b>Dokument:</b> ${document.name}`)
    if (this.export.msgs.length) {
      this.export.msgs.unshift(`<b>Ordner:</b> ${this.export.folder}`);
    }
    const exportFields = this.export.documents.flatMap(document => document.mapped || []).filter(field => field.export);
    const byName: Record<string, IMappedField[]> = exportFields.reduce((map, current) => {
      if (!map.hasOwnProperty(current.mappedName)) {
        map[current.mappedName] = [];
      }
      map[current.mappedName].push(current);
      return map;
    }, {});

    const newFields: TUIExportInput[] = [];
    for (const mappedName in byName) {
      const fields = byName[mappedName];
      const docs = this.export.documents.filter(doc => doc.mapped.reduce(
        (found, current) => {
        return found || fields.includes(current);
      }, false));

      newFields.push({
        mappedName: mappedName,
        value: this.export.fields.find(field => field.mappedName === mappedName)?.value ?? '',
        identifiers: fields.map(field => field.origId),
        info: docs.map(doc => doc.name).join(', '),
      });
    }
    this.export.fields = newFields;
  }

  // use confirm dialog

  private removeDocument(template: TUIDocument) {
    if (this.dataSource) {
      this.dataSource.splice(this.dataSource.indexOf(template), 1);
      this.updateExport();
      this.electronService.removeDocument(template);
    }
  }

  // use confirm dialog

  private removeField(field: TUIDocumentField) {
    if (this.dataSource) {
      const document =
        this.dataSource.filter(template => template.type !== 'resource')
            .find(template => template.mapped.find(tfield => tfield.origId === field.origId));
      document.mapped.splice(document.mapped.indexOf(field), 1);
      this.updateExport();
      this.electronService.save(document);
    }
  }

  ngOnInit(): void {
    console.log('Init App');
    // this.electronService.getTemplates();
    if (!this.electronService.isElectron) {
      setTimeout(() => {
        this.appService.modal$.next(false);
        this.profiles = [{id: 'p1', name: 'P1', documentIds: [], fieldIds: []}, {
          id: 'p2',
          name: 'P2',
          documentIds: [],
          fieldIds: []
        }];
        this.updateDataSource(APP_CONFIG.testData);
      }, 200);
    }
  }

  ngOnDestroy() {
    [...this.subs, /*this.snackSub,*/ this.dialogSub].forEach(sub => !sub.closed && sub.unsubscribe());
  }

  addFileTemplate() {
    this.electronService.addFileTemplate(this.options.autoMapFields);
  }

  addFileTemplates() {
    this.electronService.addFileTemplates(this.options.autoMapFields);
  }

  remapDocument(doc: IMappedDocument, $event: MouseEvent) {
    $event.stopPropagation(); // do not toggle accordion
    this.electronService.remapFileTemplate(doc);
  }

  createDocuments() {
    this.updateExport();
    const documentIds = this.export.documents.map(doc => doc.id);
    this.electronService.createDocuments(this.export.folder, documentIds, this.export.fields);
  }

  openOutputFolder() {
    this.electronService.openOutputFolder('');
  }

  changedExportOnDocument(doc: Omit<IMappedDocument<TTemplateType>, "mapped"> & { export: boolean; mapped: TUIDocumentField[] }) {
    doc.mapped.forEach(field => field.export = doc.export);
    this.updateExport();
  }

  changedExportOnField() {
    this.updateExport();
  }

  showConfirmDialog(document: TUIDocument, field: TUIDocumentField, profile: string | null, $event: MouseEvent,) {
    $event.stopPropagation(); // dont trigger the expandable panel
    const profileObj = profile ? this.profiles.find(prof => prof.id === profile) : null;
    const type = !!document ? 'Dokument' : !!field ? 'Feld' : 'Export Profil';
    const name = !!document ? document.name : !!field ? field.mappedName : profileObj?.name ?? 'Error';
    const dialogRef = this.dialog.open(ConfirmDialogComponent, {data: `${type} ${name} wirklich entfernen?`});
    if (this.dialogSub && !this.dialogSub.closed) {
      this.dialogSub.unsubscribe();
    }
    this.dialogSub = dialogRef.afterClosed().subscribe((result: boolean) => {
      if (result) {
        if (field) {
          this.removeField(field);
        } else if (document) {
          this.removeDocument(document);
        } else if (profile) {
          this.removeProfile();
        }
      }
    });
  }

  showFieldMappingDialog(node: IXlsxDocument | IPdfDocument | any, $event: MouseEvent) {
    $event.stopPropagation(); // dont trigger the expandable panel
    const possibleFields = node.type === 'pdf' ? node.fields : node.sheets;
    const fieldNames: string[] = this.dataSource
                                     .flatMap((docu) => (docu.mapped || []).map(field => field.mappedName))
                                     .filter((item, idx, arr) => arr.indexOf(item) === idx);
    const dialogRef = this.dialog.open(FieldDialogComponent, {
      data: {
        possibleFields,
        used: node.mapped,
        type: node.type,
        fieldNames
      }
    });

    if (this.dialogSub && !this.dialogSub.closed) {
      this.dialogSub.unsubscribe();
    }
    this.dialogSub = dialogRef.afterClosed().subscribe((result: IMappedField) => {
      if (result) {
        node.mapped = (node.mapped || []);
        node.mapped.push(result);
        this.electronService.save(node);
        this.updateExport();
      }
    });
  }

  blurNameInput(node: IMappedField) {
    if (this.changedName) {
      this.changedName = false;
      const template = this.findTemplateWithMappedOriginFieldId(node.origId);
      this.electronService.save(template);
    }
  }

  changingName() {
    this.changedName = true;
  }

  showFieldMappingDocument(node: any, $event: MouseEvent) {
    $event.stopPropagation(); // dont trigger the expandable panel
    if (!node.previewfile) return;
    this.electronService.openFileWithExplorer(node.previewfile);
  }

  showDocument(node: IMappedDocument, $event: MouseEvent) {
    $event.stopPropagation(); // dont trigger the expandable panel
    this.electronService.openFileWithExplorer(node.filename);
  }

  toggleSort() {
    if (this.currentSort === 'aufsteigend') {
      this.currentSort = 'absteigend';
    } else {
      this.currentSort = 'aufsteigend';
    }
    this.sortDocuments();
  }

  private sortDocuments() {
    this.dataSource.sort((a, b) => this.currentSort === 'aufsteigend' ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name));
  }

  changedProfile() {
    const current = this.profiles.find(profile => profile.id === this.profileId);
    if (!current) this.profileId = '';
    this.dataSource.forEach(document => {
      document.export = !!current?.documentIds.includes(document.id);
      document.mapped?.forEach(field => field.export = !!current?.fieldIds.includes(field.origId));
    });
    this.updateExport();
  }

  saveProfile() {
    const current = this.profiles.find(profile => profile.id === this.profileId);
    current.documentIds = this.dataSource.filter(docu => docu.export).map(docu => docu.id);
    current.fieldIds = this.dataSource
                           .flatMap(docu => docu.mapped || [])
                           .filter(field => field.export)
                           .map(field => field.origId);
    this.electronService.saveProfile(this.profiles);
  }

  // use confirm dialog

  private removeProfile() {
    this.profiles.splice(this.profiles.findIndex(profile => profile.id === this.profileId), 1);
    this.profileId = '';
    this.electronService.saveProfile(this.profiles);
  }

  addProfile() {
    const dialogRef = this.dialog.open(AddProfileDialogComponent, {
      autoFocus: 'dialog',
      hasBackdrop: true,
      restoreFocus: true
    });

    if (this.dialogSub && !this.dialogSub.closed) {
      this.dialogSub.unsubscribe();
    }
    this.dialogSub = dialogRef.afterClosed().subscribe((result: string) => {
      if (result) {
        const fieldIds = this.export.documents.flatMap(document => document.mapped || []).filter(field => field.export)
                             .map(field => field.origId);
        this.profileId = `${Date.now()}`;
        this.profiles.push({
          id: this.profileId,
          name: result,
          fieldIds,
          documentIds: this.export.documents.map(doc => doc.id)
        });
        this.saveProfile();
      }
    });
  }

  private updateProfiles(data: IProfile[]) {
    if (data.length) {
      this.profiles = data;
      this.changedProfile();
    }
  }

  showCustomConfirm(message: string) {
    const dialogRef = this.dialog.open(ConfirmDialogComponent, {data: message});
    return dialogRef.afterClosed().toPromise();
  }

  async resetApp() {
    const doReset = await this.showCustomConfirm('Dies löscht alle Daten aus dem Programm. Sind Sie sich wirklich sicher?');
    if (doReset) {
      this.electronService.resetApp()
      this.dataSource = [];
      this.profiles = [];
      this.profileId = '';
    }
  }
}
