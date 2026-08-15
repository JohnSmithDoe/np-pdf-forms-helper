import { ipcMain, IpcMainEvent } from 'electron';
import {
  EAppChannels,
  IAppConfig,
  IClientData,
  IMappedDocument,
  IMappedInput,
  IProfile,
} from './bridge/shared.model';
import { NpAssistant } from './np-assistant';
import { startWithExpolorer } from './utils/shell.utils';
import * as path from 'path';
import * as fs from 'fs';

type TApiDescription = {
  [key in EAppChannels]: (
    event: IpcMainEvent,
    ...args: any[]
  ) => IClientData | Promise<IClientData> | false;
};

export class ApiController {
  private api: Omit<
    TApiDescription,
    | EAppChannels.CLIENT_UPDATE
    | EAppChannels.CLIENT_ERROR
    | EAppChannels.FINISHED_LOAD
  > = {
    [EAppChannels.GET]: (event: IpcMainEvent) =>
      this.npAssistant.getClientData(true, false),
    [EAppChannels.REMOVE]: (
      event: IpcMainEvent,
      id: string,
      everything?: boolean
    ) => this.npAssistant.removeDocument(id, everything),
    [EAppChannels.OPEN]: (event: IpcMainEvent, filename: string) =>
      startWithExpolorer(filename),
    [EAppChannels.OPEN_OUTPUT]: (event: IpcMainEvent, folder: string) =>
      this.openOutputFolderWithExplorer(folder),
    [EAppChannels.ADD]: (
      event: IpcMainEvent,
      autoMapFields?: boolean,
      folder?: boolean
    ) => this.npAssistant.addDocuments(!!autoMapFields, !!folder),
    [EAppChannels.REMAP]: (event: IpcMainEvent, id: string) =>
      this.npAssistant.remapDocument(id),
    [EAppChannels.SAVE]: (event: IpcMainEvent, document: IMappedDocument) =>
      this.npAssistant.updateDocument(document),
    [EAppChannels.EXPORT]: (
      event: IpcMainEvent,
      exportFolder: string,
      exportDocuments: string[],
      exportFields: IMappedInput[]
    ) =>
      this.npAssistant.createDocuments(
        exportFolder,
        exportDocuments,
        exportFields
      ),
    [EAppChannels.GET_PROFILES]: (event: IpcMainEvent) =>
      this.npAssistant.getClientData(false, true),
    [EAppChannels.SAVE_PROFILES]: (event: IpcMainEvent, profiles: IProfile[]) =>
      this.npAssistant.updateProfiles(profiles),
  };

  constructor(
    private readonly npAssistant: NpAssistant,
    private readonly config: IAppConfig
  ) {
    for (const channel in this.api) {
      ipcMain.handle(channel, async (ev, ...args) => {
        console.log('Received on ', channel);
        try {
          const result = await this.api[channel](ev, ...args);
          ev.sender.send(EAppChannels.CLIENT_UPDATE, result);
        } catch (e) {
          ev.sender.send(EAppChannels.CLIENT_ERROR, e.message.split('||'));
        }
      });
    }
  }

  // -------------------------- Sync --------------------------------------------
  private openOutputFolderWithExplorer(folder: string): false {
    if (fs.existsSync(folder)) {
      startWithExpolorer(folder);
    } else {
      startWithExpolorer(this.config.OUTPUT_PATH);
    }
    return false;
  }
}
