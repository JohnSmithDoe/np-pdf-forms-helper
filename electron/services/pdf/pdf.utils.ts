import {
  CFDFError,
  CFDFStarter,
  CFDFTagClose,
  CFDFTagKids,
  CFDFTagOpen,
  CFDFTagTarget,
  CFDFTagValue,
  CFDFTrailer,
  IFDFModel,
  IFDFValue,
} from './pdf.types';

export function stripLastChars(txt: string, chars: string, trim = true) {
  const stripped = txt.endsWith(chars)
    ? txt.substring(0, txt.length - chars.length)
    : txt;
  return trim ? stripped.trim() : stripped;
}

function stripFirstChars(txt: string, chars: string, trim = true) {
  const stripped = txt.startsWith(chars) ? txt.substring(chars.length) : txt;
  return trim ? stripped.trim() : stripped;
}

function stripBrackets(txt: string, trim = true) {
  txt = trim ? txt.trim() : txt;
  if (txt.startsWith('(') && txt.endsWith(')')) {
    txt = txt.substring(1, txt.length - 1);
  }
  return txt;
}

export function validateFDF(fileContent: string, strip = true): string {
  if (
    !fileContent.startsWith(CFDFStarter) ||
    !fileContent.endsWith(CFDFTrailer)
  ) {
    throw new Error(CFDFError);
  }
  if (strip) {
    fileContent = fileContent.substring(
      CFDFStarter.length,
      fileContent.length - CFDFTrailer.length
    );
    fileContent = fileContent.replace(/\n/g, '');
    // fileContent = fileContent.replace(/\s/g, '');
    if (!fileContent.endsWith(CFDFTagClose)) {
      throw new Error(CFDFError);
    }
  }

  return fileContent;
}

export function parseFields(data: string) {
  const fdf: IFDFModel = { kids: [], values: [], parent: null, target: '' };
  let current = fdf;
  const allValues = [];

  while (!!data.length) {
    if (data.startsWith(CFDFTagOpen)) {
      data = stripFirstChars(data, CFDFTagOpen);
    } else if (data.startsWith(CFDFTagClose)) {
      data = stripFirstChars(data, CFDFTagClose);
    } else if (data.startsWith('[')) {
      data = stripFirstChars(data, '[');
    } else if (data.startsWith(']')) {
      data = stripFirstChars(data, ']');
    } else if (data.startsWith(CFDFTagKids)) {
      const kid: IFDFModel = {
        kids: [],
        values: [],
        parent: current,
        target: '',
      };
      current.kids.push(kid);
      current = kid;
      data = stripFirstChars(data, CFDFTagKids);
    } else if (data.startsWith(CFDFTagValue)) {
      const value = stripBrackets(
        data.substring(CFDFTagValue.length, data.indexOf(CFDFTagTarget))
      );
      data = data.substring(data.indexOf(CFDFTagTarget));
      const target = stripBrackets(
        data.substring(
          data.indexOf(CFDFTagTarget) + CFDFTagTarget.length,
          data.indexOf(CFDFTagClose)
        )
      );

      const valueObj = { value, target, path: target, parent: current };
      allValues.push(valueObj);
      current.values.push(valueObj);
      data = data
        .substring(data.indexOf(CFDFTagClose) + CFDFTagClose.length)
        .trim();
    } else if (data.startsWith(CFDFTagTarget)) {
      current.target = stripBrackets(
        data.substring(
          data.indexOf(CFDFTagTarget) + CFDFTagTarget.length,
          data.indexOf(CFDFTagClose)
        )
      );
      current = current.parent;
      data = data
        .substring(data.indexOf(CFDFTagClose) + CFDFTagClose.length)
        .trim();
    } else {
      // If an unexpected token occurs
      throw new Error(CFDFError + data.substring(0, 6));
    }
  }
  return { fdf, allValues };
}

/** strips the header and trailer of the fdf file and returns only the fields part */
export function parseFDF(data: string): {
  fdf: IFDFModel;
  allValues: IFDFValue[];
} {
  data = validateFDF(data);
  const { fdf, allValues } = parseFields(data);
  updateFDFValuePaths(allValues);
  return { fdf, allValues };
}

// update paths of values with the value of a tree walk up
export function updateFDFValuePaths(allValues: any[]) {
  function getPath(value: IFDFModel) {
    return (value.parent ? getPath(value.parent) : '') + value.target;
  }

  allValues.forEach((value) => {
    value.path = getPath(value.parent) + value.target;
  });
}

export function assembleFDF(fdf: IFDFModel): string {
  let output = '';

  function flattenFDF(current: IFDFModel) {
    if (!!current.kids.length) {
      current.kids.forEach((kid) => {
        output += CFDFTagOpen + '\n';
        output += CFDFTagKids + ' [' + '\n';
        flattenFDF(kid);
        output = stripLastChars(output, '\n');
        output = stripLastChars(output, ' ');
        output += ']' + '\n';
        output += CFDFTagTarget + ' (' + kid.target + ')\n';
        output += CFDFTagClose + ' \n';
      });
    }
    current.values.forEach((value) => {
      output +=
        CFDFTagOpen +
        '\n' +
        CFDFTagValue +
        (value.value.startsWith('/')
          ? ' ' + value.value
          : ' (' + value.value + ')') +
        '\n' +
        CFDFTagTarget +
        ' (' +
        value.target +
        ')\n' +
        CFDFTagClose +
        ' \n';
    });
  }

  flattenFDF(fdf);
  output = stripLastChars(output, '\n');
  output = stripLastChars(output, ' ');
  return CFDFStarter + output + CFDFTrailer;
}
