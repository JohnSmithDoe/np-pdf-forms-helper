import { NPService } from '../np-services';
import {
  IMappedDocument,
  IMappedInput,
  TTemplateType,
} from '../../bridge/shared.model';
import * as path from 'path';
import * as fs from 'fs';
import { getFileStats, removeTempCopy } from '../../utils/file.utils';

export class ResourceService extends NPService {
  async createDocument(
    document: IMappedDocument,
    outputPath: string,
    inputs: IMappedInput[],
    outputMsgs: string[]
  ): Promise<string> {
    const { tmpCopy, basename } = this.copyAndValidateOriginalToTemp(
      document,
      outputMsgs
    );
    const outCopy = path.join(outputPath, basename);
    fs.copyFileSync(tmpCopy, outCopy);
    removeTempCopy(tmpCopy, this.config.TMP_PATH);
    return basename;
  }
}
