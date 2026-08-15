import {EventEmitter, Injectable} from '@angular/core';
import {ipcRenderer} from 'electron';
import {
  EAppChannels,
  IClientData,
  IClientReportData,
  IMappedDocument,
  IMappedInput,
  IProfile
} from '../../../bridge/shared.model';
import IpcRendererEvent = Electron.IpcRendererEvent;

@Injectable({
  providedIn: 'root'
})
export class ElectronService {

  public startRequest$ = new EventEmitter<EAppChannels>();
  public stopRequest$ = new EventEmitter<EAppChannels>();

  public update$ = new EventEmitter<IClientData>();
  public finishedLoading$ = new EventEmitter<IClientData | undefined>();
  public error$ = new EventEmitter<string[]>();
  public report$ = new EventEmitter<IClientReportData>();

  private readonly ipcRenderer: typeof ipcRenderer;

  constructor() {
    // Conditional imports
    if (this.isElectron) {
      this.ipcRenderer = window.require('electron').ipcRenderer;
      this.ipcRenderer.on(EAppChannels.FINISHED_LOAD, (event: IpcRendererEvent, data: IClientData) => {
        this.stopRequest$.emit(EAppChannels.FINISHED_LOAD);
        this.finishedLoading$.emit(data);
      });
      this.ipcRenderer.on(EAppChannels.CLIENT_UPDATE, (event: IpcRendererEvent, data: IClientData | undefined) => {
        if (data.documents || data.profiles) {
          this.update$.emit(data);
        }
        if (data.message) {
          this.report$.emit(data.message);
        }
      });
      this.ipcRenderer.on(EAppChannels.CLIENT_ERROR, (event: IpcRendererEvent, err: any) => this.error$.emit(err));
      // this.webFrame = window.require('electron').webFrame;
      // this.childProcess = window.require('child_process');
      // this.fs = window.require('fs');

      // Notes :
      // * A NodeJS's dependency imported with 'window.require' MUST BE present in `dependencies` of both `app/package.json`
      // and `package.json (root folder)` in order to make it work here in Electron's Renderer process (src folder)
      // because it will load at runtime by Electron.
      // * A NodeJS's dependency imported with TS module import (ex: import { Dropbox } from 'dropbox') CAN only be present
      // in `dependencies` of `package.json (root folder)` because it is loaded during build phase and does not need to be
      // in the final bundle. Reminder : only if not used in Electron's Main process (app folder)

      // If you want to use a NodeJS 3rd party deps in Renderer process,
      // ipcRenderer.invoke can serve many common use cases.
      // https://www.electronjs.org/docs/latest/api/ipc-renderer#ipcrendererinvokechannel-args
    }
  }

  get isElectron(): boolean {
    return !!(window && window.process && window.process.type);
  }

  private get renderer() {
    return this.isElectron ? this.ipcRenderer : undefined;
  }

  private send(channel: EAppChannels, ...data: any) {
    console.log('Send on channel: ', channel);
    this.startRequest$.emit(channel);
    this.renderer?.invoke(channel, ...data).then((data) => {
      console.log('Response on channel: ', channel, data);
      this.stopRequest$.emit(channel);
    });
  }

  addFileTemplate(autoMap: boolean) {
    this.send(EAppChannels.ADD, autoMap);
  }
  addFileTemplates(autoMap: boolean) {
    this.send(EAppChannels.ADD, autoMap, true);
  }

  save(data: IMappedDocument) {
    this.send(EAppChannels.SAVE, data);
  }

  export(foldername: string, data: IMappedInput[]) {
    this.send(EAppChannels.EXPORT, data, foldername);
  }

  openFileWithExplorer(filename: string) {
    this.send(EAppChannels.OPEN, filename);
  }

  openOutputFolder(folder: string) {
    this.send(EAppChannels.OPEN_OUTPUT, folder);
  }

  createDocuments(exportFolder: string, exportDocumentIds: string[], exportedFields: IMappedInput[]) {
    this.send(EAppChannels.EXPORT,
      exportFolder,
      exportDocumentIds,
      exportedFields.map(({identifiers, value, ...field}) => ({identifiers, value})));
  }

  removeDocument(template: IMappedDocument) {
    this.send(EAppChannels.REMOVE, template.id);
  }
  resetApp() {
    this.send(EAppChannels.REMOVE, -1, true);
  }

  saveProfile(profiles: IProfile[]) {
    this.send(EAppChannels.SAVE_PROFILES, profiles);
  }

  remapFileTemplate(doc: IMappedDocument) {
    this.send(EAppChannels.REMAP, doc.id);
  }
}
