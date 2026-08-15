import {Component, Inject, OnInit} from '@angular/core';
import {FormControl} from '@angular/forms';
import {MAT_DIALOG_DATA, MatDialogRef} from '@angular/material/dialog';
import {Observable} from 'rxjs';
import {map, startWith} from 'rxjs/operators';
import {IMappedField} from '../../../../../bridge/shared.model';
import {TUIDocumentField} from "../../../home/home.component";

@Component({
             selector:    'app-field-dialog',
             templateUrl: './field-dialog.component.html',
             styleUrls:   ['./field-dialog.component.scss']
           })
export class FieldDialogComponent implements OnInit {
  current = {
    mappedName:   '',
    id:     '',
    column: '',
    row:    undefined,
  };
  fieldNameControl = new FormControl('');
  filteredNames$: Observable<string[]>;

  constructor(
    public dialogRef: MatDialogRef<FieldDialogComponent>,
    @Inject(MAT_DIALOG_DATA) public data: {
      type: 'pdf' | 'xlsx',
      possibleFields: { id: string, path: string, disabled: boolean }[],
      used: IMappedField[],
      fieldNames: string[]
    },
  ) { }

  ngOnInit(): void {
    this.data.possibleFields.forEach(field => {
      field.disabled = !!this.data.used.find(item=> item.origId === field.id);
    })
    this.filteredNames$ = this.fieldNameControl.valueChanges.pipe(
      startWith(''),
      map(name => (name ? this.filter(name) : this.data.fieldNames.slice())),
    );
  }
  private filter(name: string): string[] {
    const filterValue = name.toLowerCase();
    return this.data.fieldNames.filter(field => field.toLowerCase().includes(filterValue));
  }

  createMappedField() {
    // const identifier = this.data.type === 'pdf'
    //   ? this.current.id
    //   : `$${this.current.id}.${this.current.column}${this.current.row}`;
    const orig = this.data.possibleFields.find(field => field.id === this.current.id);
    const mappedName = this.data.type === 'pdf'
      ?  this.current.mappedName
      : `$${orig.path}.${this.current.column}${this.current.row}`;

    const mappedField: TUIDocumentField = {
      origId: this.current.id,
      export: true,
      mappedName
    };

    this.dialogRef.close(mappedField);
  }

  onCloseClick(): void {
    this.dialogRef.close();
  }

  columnChanged(input: string) {
    setTimeout(() => {
      this.current.column = input.replace(/[^a-zA-Z]/g, '').toUpperCase();
    }, 0);
  }
  isDuplicated() {
    const orig = this.data.possibleFields.find(field => field.id === this.current.id);
    if(!orig) return false;
    const current = `$${orig.path}.${this.current.column}${this.current.row}`;
    return this.data.used.find(used => used.mappedName === current)
  }

  isValid() {
    if(!this.data || !this.current.id || !this.current.mappedName.length) return false;
    if(this.data.type === 'pdf') return true;
    return !this.isDuplicated();
  }

  updateFilter($event: any) {
    this.fieldNameControl.setValue($event);
  }

  fieldChosen(id: string) {
    const orig = this.data.possibleFields.find(field => field.id === id);
    if(!orig) return false;
    this.current.mappedName = orig.path;
  }
}
