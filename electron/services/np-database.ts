import { IAppConfig, IMappedDocument, IProfile } from '../bridge/shared.model';
import * as fs from 'fs';

type TDocumentDatabase = {
  version: number;
  documents: { [key: string]: IMappedDocument };
};

type TProfileDatabase = {
  version: number;
  profiles: { [key: string]: IProfile };
};

export class NpDatabase {
  private readonly version = 1;
  private _database: TDocumentDatabase;
  private _profiles_db: TProfileDatabase;

  constructor(private config: IAppConfig) {
    this.readDocumentDatabase();
    this.readProfilesDatabase();
    this.migrateDatabase();
  }

  get profiles(): IProfile[] {
    return Object.values(this._profiles_db?.profiles ?? {});
  }

  get documents(): IMappedDocument[] {
    return Object.values(this._database?.documents ?? {});
  }

  reset() {
    this._profiles_db.profiles = {};
    this._database.documents = {};
    this.writeProfilesDatabase();
    this.writeDocumentDatabase();
  }

  //<editor-fold desc="*** File handling *** ">

  private readDocumentDatabase() {
    if (fs.existsSync(this.config.DB_FILE)) {
      const content = fs.readFileSync(this.config.DB_FILE, {
        encoding: 'utf8',
      });
      this._database = JSON.parse(content);
    } else {
      console.warn('new database');
      this._database = { version: this.version, documents: {} };
    }
  }

  private readProfilesDatabase() {
    if (fs.existsSync(this.config.PROFILE_FILE)) {
      const content = fs.readFileSync(this.config.PROFILE_FILE, {
        encoding: 'utf8',
      });
      this._profiles_db = JSON.parse(content);
    } else {
      this._profiles_db = { version: this.version, profiles: {} };
    }
  }

  private migrateDatabase() {
    // not yet
    console.log('Database version: ' + this.version);
  }

  private writeDocumentDatabase() {
    try {
      fs.writeFileSync(
        this.config.DB_FILE,
        JSON.stringify(this._database, (key, value) => {
          if (['export', 'disabled'].includes(key)) return undefined;
          return value;
        }),
        { encoding: 'utf8' }
      );
    } catch (e) {
      console.error(e);
    }
  }

  private writeProfilesDatabase() {
    try {
      fs.writeFileSync(
        this.config.PROFILE_FILE,
        JSON.stringify(this._profiles_db),
        { encoding: 'utf8' }
      );
    } catch (e) {
      console.error(e);
    }
  }

  //</editor-fold>

  //<editor-fold desc="*** Document handling ***">

  documentExists(filename: string) {
    return !!this.documents.find((doc) => doc.filename === filename);
  }

  documentIdExists(id: string) {
    return !!this._database?.documents?.hasOwnProperty(id);
  }

  getDocument(id: string) {
    return this._database.documents[id];
  }

  addDocument(document: IMappedDocument) {
    if (!this.documentIdExists(document.id)) {
      this._database.documents[document.id] = document;
      this.writeDocumentDatabase();
    }
    return this.documents;
  }

  removeDocument(id: string) {
    if (this.documentIdExists(id)) {
      const document = this._database.documents[id];
      delete this._database.documents[id];
      this.writeDocumentDatabase();
      this.updateProfilesOnDocumentRemove(document);
    } else {
      throw new Error('Document did not exist');
    }
    return this.documents;
  }

  updateDocument(document: IMappedDocument, forceUpdate = false) {
    if (this.documentIdExists(document.id)) {
      const original = this._database.documents[document.id];
      this._database.documents[document.id] = document;
      this.writeDocumentDatabase();
      if (forceUpdate || original.mapped?.length !== document.mapped?.length) {
        this.updateProfilesOnDocumentChange(document);
      }
    }
    return this.documents;
  }

  //</editor-fold>

  //<editor-fold desc="*** Profiles handling ***">
  removeProfile(id: string) {
    if (!!this._profiles_db?.profiles?.hasOwnProperty(id)) {
      delete this._profiles_db.profiles[id];
      this.writeProfilesDatabase();
    }
    return this.profiles;
  }

  updateProfiles(profiles: IProfile[]) {
    this._profiles_db.profiles = profiles.reduce((prev, current) => {
      prev[current.id] = current;
      return prev;
    }, {});
    this.writeProfilesDatabase();
    return this.profiles;
  }

  //</editor-fold>
  private updateProfilesOnDocumentChange(document: IMappedDocument) {
    this.profiles.forEach((profile) => {
      if (profile.documentIds.includes(document.id)) {
        profile.fieldIds = profile.fieldIds.filter((origId) =>
          this.mappedFieldExists(origId)
        );
      }
    });
    this.writeProfilesDatabase();
  }

  private updateProfilesOnDocumentRemove(document: IMappedDocument) {
    this.profiles.forEach((profile) => {
      if (profile.documentIds.includes(document.id)) {
        profile.documentIds.splice(profile.documentIds.indexOf(document.id), 1);
        profile.fieldIds = profile.fieldIds.filter((origId) =>
          this.mappedFieldExists(origId)
        );
      }
    });
    this.writeProfilesDatabase();
  }
  private mappedFieldExists(origId: string) {
    return !!this.documents.find(
      (document) =>
        !!document.mapped.find((mappedField) => mappedField.origId === origId)
    );
  }
}
