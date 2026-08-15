import * as fs from 'fs';
import * as path from 'path';
import { TTemplateType } from '../bridge/shared.model';

export function copyToTempFolder(filename: string, tmpPath: string) {
  const basename = path.basename(filename);
  const tmpCopy = path.join(tmpPath, basename);
  fs.copyFileSync(filename, tmpCopy);
  return tmpCopy;
}

export function removeTempCopy(
  filename: string,
  tmpPath: string,
  dryrun = false
) {
  const basename = path.basename(filename);
  const tmpCopy = path.join(tmpPath, basename);
  if (!dryrun) fs.rmSync(tmpCopy);
  return tmpCopy;
}

export function getFileInfo(filename): {
  basename: string;
  basenameNoExt: string;
  type: TTemplateType;
} {
  const ext = path.extname(filename);
  const basename = path.basename(filename);
  const basenameNoExt = path.basename(filename, ext);
  const type = ext === '.pdf' ? 'pdf' : ext === '.xlsx' ? 'xlsx' : 'resource';
  return { basename, basenameNoExt, type };
}

export function getFileStats(filename: string): {
  basename: string;
  basenameNoExt: string;
  type: TTemplateType;
  mtimeMs: number;
} {
  const { mtimeMs } = fs.statSync(filename);
  return { ...getFileInfo(filename), mtimeMs };
}

export function getFilesFromFolderSync(foldername: string) {
  return fs.readdirSync(foldername);
}

export function createAndValidateFolder(outputFolder: string) {
  let result: string[] = [];
  let valid = true;
  if (!fs.existsSync(outputFolder)) {
    fs.mkdirSync(outputFolder);
    result.push(`Ordner wurde erstellt: ${outputFolder}`);
  } else {
    valid = false;
    result.push(`Ordner existierte bereits: ${outputFolder}`);
    result.push(
      'Erstellen wurde abgebrochen damit keine Daten überschrieben werden.'
    );
    result.push(
      'Bitte warte eine Minute, lösche den Ordner oder verwende ein anderes suffix'
    );
  }
  return { outputMsgs: result, valid };
}
