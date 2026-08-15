import {Injectable} from '@angular/core';
import {BehaviorSubject} from 'rxjs';

@Injectable({providedIn: 'root'})
export class AppService {
  public readonly modal$: BehaviorSubject<boolean> = new BehaviorSubject<boolean>(true);
}
