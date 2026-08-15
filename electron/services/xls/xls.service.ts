import { IMappedInput, IXlsxDocument } from '../../bridge/shared.model';
import { NPService } from '../np-services';
import { copyToTempFolder, removeTempCopy } from '../../utils/file.utils';
import { v4 as uuidv4 } from 'uuid';
import * as ExcelJS from 'exceljs';
import * as path from 'path';

export class XlsService extends NPService {
  async addDocument(filename: string): Promise<IXlsxDocument> {
    const base = await super.addDocument(filename);
    const workbook = new ExcelJS.Workbook();
    const tmpCopy = copyToTempFolder(filename, this.config.TMP_PATH);
    await workbook.xlsx.readFile(tmpCopy);
    const sheets = workbook.worksheets.map((sheet) => ({
      id: uuidv4(),
      name: sheet.name,
    }));
    if (!sheets) {
      throw new Error('No Worksheets found');
    }
    removeTempCopy(tmpCopy, this.config.TMP_PATH);
    return { ...base, type: 'xlsx', sheets };
  }

  async createDocument(
    document: IXlsxDocument,
    outputPath: string,
    inputs: IMappedInput[],
    outputMsgs: string[]
  ): Promise<string> {
    const { tmpCopy, basename } = this.copyAndValidateOriginalToTemp(
      document,
      outputMsgs
    );
    const workbook = new ExcelJS.Workbook();
    await workbook.xlsx.readFile(tmpCopy);

    const flatIds = inputs.reduce((prev, current) => {
      prev.push(...current.identifiers);
      return prev;
    }, []);
    const fields = document.mapped.filter((field) =>
      flatIds.includes(field.origId)
    );
    fields.forEach((mappedField) => {
      let fullCellId = mappedField.mappedName;
      const sheetName = fullCellId.slice(1, fullCellId.indexOf('.'));
      const sheet = workbook.worksheets.find(
        (sheet) => sheet.name === sheetName
      );
      if (sheet) {
        const cell = fullCellId.slice(fullCellId.indexOf('.') + 1);
        const value = inputs.find((input) =>
          input.identifiers.includes(mappedField.origId)
        )?.value;
        if (value) {
          sheet.getCell(cell).value = value;
        }
      } else {
        throw new Error('Die Excel Datei enthält keine Arbeitsmappen');
      }
    });
    const outCopy = path.join(outputPath, basename);
    await workbook.xlsx.writeFile(outCopy);
    removeTempCopy(tmpCopy, this.config.TMP_PATH);
    return basename;
  }

  async remapDocument(
    original: IXlsxDocument,
    newfilename: string
  ): Promise<void> {
    await super.remapDocument(original, newfilename);
    throw new Error('Not implemented yet');
  }
}
