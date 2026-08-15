import {Component, OnInit} from '@angular/core';
import {MatDialogRef} from '@angular/material/dialog';

@Component({
             selector:    'app-add-profile-dialog',
             templateUrl: './add-profile-dialog.component.html',
             styleUrls:   ['./add-profile-dialog.component.scss']
           })
export class AddProfileDialogComponent implements OnInit {

  public name: string = '';

  constructor(
    public dialogRef: MatDialogRef<AddProfileDialogComponent>,
  ) { }

  ngOnInit(): void {
  }


  confirm() {
    this.dialogRef.close(this.name)
  }

  cancel() {
    this.dialogRef.close();
  }
}
