import {
  IAppConfig,
  IMappedDocument,
  IMappedInput,
} from '../bridge/shared.model';
import { v4 as uuidv4 } from 'uuid';
import { copyToTempFolder, getFileStats } from '../utils/file.utils';
import * as fs from 'fs';
import * as path from 'path';

export interface INPService {
  addDocument(filename: string): Promise<IMappedDocument>;
  createDocument(
    document: IMappedDocument,
    outputPath: string,
    inputs: IMappedInput[],
    outputMsgs: string[]
  ): Promise<string>;
}

export abstract class NPService implements INPService {
  constructor(protected config: IAppConfig) {}

  protected copyAndValidateOriginalToTemp(
    document: IMappedDocument,
    messages: string[]
  ) {
    const basename = path.basename(document.filename);
    const tmpCopy = copyToTempFolder(document.filename, this.config.TMP_PATH);
    const { mtimeMs } = fs.statSync(document.filename);
    if (document.mtime !== mtimeMs) {
      document.mtime = mtimeMs;
      messages.push('*******************');
      messages.push(
        `WARNUNG: Die Original Datei [${document.name}] wurde verändert!!!`
      );
      messages.push(`Bitte überprüfe ob die Felder noch passen.`);
      messages.push(
        `Wenn nicht lösche am besten das Dokument und füge es danach noch einmal neu hinzu.`
      );
      messages.push('*******************');
    }
    return { tmpCopy, basename };
  }

  abstract createDocument(
    document: IMappedDocument,
    outputPath: string,
    inputs: IMappedInput[],
    outputMsgs: string[]
  ): Promise<string>;

  async addDocument(filename: string): Promise<IMappedDocument> {
    const stats = getFileStats(filename);
    return {
      id: uuidv4(),
      name: stats.basename,
      filename,
      type: stats.type,
      mtime: stats.mtimeMs,
    };
  }
  async remapDocument(original: IMappedDocument, newfilename: string) {
    const { basename, mtimeMs } = getFileStats(newfilename);
    original.filename = newfilename;
    original.name = basename;
    original.mtime = mtimeMs;
  }
}
