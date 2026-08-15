import {Component, Inject, NgZone, OnInit} from '@angular/core';
import {MAT_DIALOG_DATA, MatDialogRef} from '@angular/material/dialog';
import {ElectronService} from '../../../services/electron.service';

@Component({
             selector:    'app-message-dialog',
             templateUrl: './message-dialog.component.html',
             styleUrls:   ['./message-dialog.component.scss']
           })
export class MessageDialogComponent implements OnInit {

  constructor(
    public dialogRef: MatDialogRef<MessageDialogComponent>,
    @Inject(MAT_DIALOG_DATA) public data: {headline: string, msgs: string[], folder?: string},
    private readonly ngZone: NgZone,
    private readonly electronService: ElectronService
  ) { }

  ngOnInit(): void {
  }

  openFolder(){
    this.electronService.openOutputFolder(this.data.folder);
  }

  confirm() {
    this.ngZone.runTask(()=>{
      this.dialogRef.close(true);
    });
  }

}
