export const CFDFHeader =
  String.fromCharCode(226) +
  String.fromCharCode(227) +
  String.fromCharCode(207) +
  String.fromCharCode(211);

// \n on windows only...
export const CFDFStarter =
  '%FDF-1.2\n%' + CFDFHeader + '\n1 0 obj \n<<\n/FDF \n<<\n/Fields [\n';
export const CFDFTrailer =
  ']\n>>\n>>\nendobj \ntrailer\n\n<<\n/Root 1 0 R\n>>\n%%EOF\n';

export const CFDFTagOpen = '<<';
export const CFDFTagClose = '>>';
export const CFDFTagKids = '/Kids';
export const CFDFTagValue = '/V';
export const CFDFTagTarget = '/T';

export const CFDFError = 'Could not parse FDF. Wrong Format provided!';

export interface IFDFValue {
  parent: IFDFModel;
  value: string;
  path: string;
  target: string;
}

export interface IFDFModel {
  parent: IFDFModel | null;
  kids: IFDFModel[];
  values: IFDFValue[];
  target: string;
}
