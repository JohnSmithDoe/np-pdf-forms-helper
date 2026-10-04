// ─── why ────────────────────────────────────────────────────────
// Seven JSON files under `data/trains/`, following `crate::filler::db` — same
// `IndexMap`, same atomic temp-and-rename, same "a file the user can open in
// Notepad is part of the support story".
//
// SPLIT PER STORE, and that is not cosmetic. Saving one partner must not rewrite
// the events file, which is the big one; the same asymmetry already splits
// documents from profiles next door.
//
// BATCHED, and that is the load-bearing part. At the volume this is for — a few
// thousand wagen, tens of thousands of events — a full rewrite is fine PER
// MUTATION and fatal PER ROW: a two thousand row import at one write per row is
// minutes, and more again on the network share the originals live on. So
// `transaction` is the API rather than a convention. Everything inside it runs
// in memory and each touched store is flushed once at the end; a failed flush
// restores what was there, because a store that reports success and did not
// write is the failure `filler::db` already refuses to allow.
//
// THE ROLLBACK COPY IS TAKEN PER STORE, ON FIRST WRITE, and that is what the
// `*_mut` accessors are for: saving one partner must not clone fifty thousand
// Instandhaltungen to protect a rollback that cannot reach them. The accessor is
// the ONLY way to a `&mut` on a store, so the copy cannot be forgotten — an
// ordinary `touch()` flag would have had to be called before the mutation by
// convention, and the next mutator written would have silently broken rollback.
// Being `Some` is also what marks a store dirty, so the flush list and the
// rollback list cannot drift apart. The indexes are not copied: a rollback
// rebuilds them with `reindex`.
//
// Four indexes are rebuilt on load and maintained on write — `Wagennummer →
// wagen`, `match_key → partner` including every alias, `dedupe_key → event`, and
// `match_key → radsaetze`. The last one maps a key to SEVERAL ids, because two
// senders may legitimately use one Radsatznummer for two different radsaetze —
// see `resolve::radsatz`. Without them, resolving two thousand rows against five thousand wagen and fifty
// thousand events is a hundred million comparisons; with them it is two thousand
// lookups.
//
// A `Dokument` filed before it carried its `folder` gets it on load, from its
// original's parent — the folder IS that parent, by construction in `adopt`.
//
// The SEVENTH store, `dokumente`, is the load ledger: one record per file the
// app has taken ownership of, indexed by the ORIGINAL's content hash, which is
// what makes "the same bytes dropped twice" one document. The files themselves
// live under `dokumente/<id>/` beside the stores; `reset` removes them too, or a
// reset would leave records' files behind with no record pointing at them.
//
// A WRITE THAT CLEARS a store must `reindex` after it. The `put_*` calls keep
// the indexes current, but a `clear()` inside a transaction does not, and a
// stale `dedupe_key` index made every row of a file re-imported after a reset
// stage as a duplicate until the next start.
//
// `clear_mirror` is the master import's narrow wipe: the FACTS go (Wagen,
// Radsatz, Einbau, Instandhaltung) because the master brings them back whole;
// IDENTITY and configuration stay — Partner with its learned aliases, templates
// with their learned readings, and the filed Dokumente the refresh reads, which
// become importable again. A kept alias only survives with what it points at,
// which is why Partner is kept rather than its aliases extracted.
//
// `einstellungen.json` is not a store of items but ONE settings object, read
// with defaults (a missing file or key is the default, so an old data folder
// loads) and written alone, atomically. It sits outside `transaction` because
// it changes nothing a rollback would have to restore alongside. `master.json`
// is the same shape for the same reasons, and `reset` keeps both: they are
// configuration, not imported data. The master's read copy under `master/` is
// removed like the owned documents: it is derived, and `master::bindings::sync`
// rebuilds it on the next read because the copy is missing.
//
// The SHIPPED templates are never stored. `templates` merges them in at read
// time, minus any a user copy shadows through its `origin`, so a reset cannot
// lose them and an update of the program can change them — see `builtin`.
//
// `version` is written and not read, exactly as `filler::db` does it. There is
// no deployed store to migrate from and no shipped shape to be compatible with,
// so a ladder now would be a guess about a format nobody has written yet. The
// field being there is all that is needed to add the branch later.
// ────────────────────────────────────────────────────────────────

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::trains::model::{
    Dokument, Einbau, ImportTemplate, Instandhaltung, MasterSettings, Partner, PartnerRolle,
    Radsatz, RadsatzAlias, TrainsCounts, TrainsSettings, Wagen,
};

const VERSION: u32 = 1;
const SETTINGS: &str = "einstellungen.json";
const MASTER: &str = "master.json";

pub fn remove_folder(folder: &Path) -> AppResult<()> {
    match std::fs::remove_dir_all(folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io(folder, error)),
    }
}

/// Its own key AND every alias key, so a spelling learnt for one sender finds
/// the radsatz at all — whether it then DECIDES is `resolve::radsatz`'s call.
fn radsatz_keys(radsatz: &Radsatz) -> Vec<String> {
    let mut keys = vec![radsatz.match_key.clone()];
    for alias in &radsatz.aliases {
        if !keys.contains(&alias.match_key) {
            keys.push(alias.match_key.clone());
        }
    }
    keys
}

#[derive(Deserialize)]
struct Store<T> {
    items: IndexMap<String, T>,
}

#[derive(Serialize)]
struct StoreRef<'a, T> {
    version: u32,
    items: &'a IndexMap<String, T>,
}

pub struct TrainsDb {
    folder: PathBuf,
    wagen: IndexMap<String, Wagen>,
    partner: IndexMap<String, Partner>,
    instandhaltungen: IndexMap<String, Instandhaltung>,
    templates: IndexMap<String, ImportTemplate>,
    radsaetze: IndexMap<String, Radsatz>,
    einbauten: IndexMap<String, Einbau>,
    dokumente: IndexMap<String, Dokument>,
    settings: TrainsSettings,
    master: MasterSettings,
    by_wagennummer: HashMap<String, String>,
    by_match_key: HashMap<String, String>,
    by_dedupe: HashSet<String>,
    by_radsatznummer: HashMap<String, Vec<String>>,
    by_hash: HashMap<String, String>,
}

#[derive(Default)]
struct Rollback {
    wagen: Option<IndexMap<String, Wagen>>,
    partner: Option<IndexMap<String, Partner>>,
    instandhaltungen: Option<IndexMap<String, Instandhaltung>>,
    templates: Option<IndexMap<String, ImportTemplate>>,
    radsaetze: Option<IndexMap<String, Radsatz>>,
    einbauten: Option<IndexMap<String, Einbau>>,
    dokumente: Option<IndexMap<String, Dokument>>,
}

pub struct Tx<'a> {
    db: &'a mut TrainsDb,
    rollback: Rollback,
}

impl TrainsDb {
    pub fn load(config: &AppConfig) -> AppResult<Self> {
        let folder = config.data_path.join("trains");
        std::fs::create_dir_all(&folder).map_err(|error| AppError::io(&folder, error))?;

        let mut db = Self {
            wagen: read(&folder.join("wagen.db"))?,
            partner: read(&folder.join("partner.db"))?,
            instandhaltungen: read(&folder.join("instandhaltungen.db"))?,
            templates: read(&folder.join("templates.db"))?,
            radsaetze: read(&folder.join("radsaetze.db"))?,
            einbauten: read(&folder.join("einbauten.db"))?,
            dokumente: read(&folder.join("dokumente.db"))?,
            settings: read_or_default(&folder.join(SETTINGS))?,
            master: read_or_default(&folder.join(MASTER))?,
            folder,
            by_wagennummer: HashMap::new(),
            by_match_key: HashMap::new(),
            by_dedupe: HashSet::new(),
            by_radsatznummer: HashMap::new(),
            by_hash: HashMap::new(),
        };
        for dokument in db.dokumente.values_mut() {
            if dokument.folder.is_empty() {
                if let Some(parent) = Path::new(&dokument.original).parent() {
                    dokument.folder = parent.to_string_lossy().into_owned();
                }
            }
        }
        db.reindex();
        Ok(db)
    }

    fn reindex(&mut self) {
        self.by_wagennummer = self
            .wagen
            .values()
            .map(|wagen| (wagen.nummer.clone(), wagen.id.clone()))
            .collect();
        self.by_match_key = self
            .partner
            .values()
            .flat_map(|partner| {
                std::iter::once(partner.match_key.clone())
                    .chain(partner.aliases.iter().cloned())
                    .map(|key| (key, partner.id.clone()))
            })
            .collect();
        self.by_dedupe = self
            .instandhaltungen
            .values()
            .map(|event| event.dedupe_key.clone())
            .collect();
        self.by_hash = self
            .dokumente
            .values()
            .map(|dokument| (dokument.original_hash.clone(), dokument.id.clone()))
            .collect();
        self.reindex_radsaetze();
    }

    fn reindex_radsaetze(&mut self) {
        self.by_radsatznummer = HashMap::new();
        for radsatz in self.radsaetze.values() {
            for key in radsatz_keys(radsatz) {
                self.by_radsatznummer
                    .entry(key)
                    .or_default()
                    .push(radsatz.id.clone());
            }
        }
    }

    pub fn radsaetze(&self) -> Vec<Radsatz> {
        self.radsaetze.values().cloned().collect()
    }

    pub fn einbauten(&self) -> Vec<Einbau> {
        self.einbauten.values().cloned().collect()
    }

    pub fn einbau_of(
        &self,
        radsatz_id: &str,
        wagen_id: &str,
        eingebaut_am: Option<&str>,
    ) -> Option<&Einbau> {
        self.einbauten.values().find(|einbau| {
            einbau.radsatz_id == radsatz_id
                && einbau.wagen_id == wagen_id
                && einbau.eingebaut_am.as_deref() == eingebaut_am
        })
    }

    pub fn wagen_refs(&self) -> impl Iterator<Item = &Wagen> {
        self.wagen.values()
    }

    pub fn radsatz_refs(&self) -> impl Iterator<Item = &Radsatz> {
        self.radsaetze.values()
    }

    pub fn open_einbau(&self, radsatz_id: &str) -> Option<&Einbau> {
        self.einbauten
            .values()
            .find(|einbau| einbau.radsatz_id == radsatz_id && einbau.is_open())
    }

    pub fn radsatz(&self, id: &str) -> Option<&Radsatz> {
        self.radsaetze.get(id)
    }

    /// SEVERAL, never one: two senders may legitimately use one number for two
    /// different radsaetze, so this cannot answer with a single hit.
    pub fn radsaetze_by_key(&self, key: &str) -> Vec<&Radsatz> {
        self.by_radsatznummer
            .get(key)
            .map(|ids| ids.iter().filter_map(|id| self.radsaetze.get(id)).collect())
            .unwrap_or_default()
    }

    /// Tier two, and only for leading zeros — see `resolve::radsatz`.
    pub fn radsaetze_without_leading_zeros(&self, stripped: &str) -> Vec<&Radsatz> {
        self.radsaetze
            .values()
            .filter(|radsatz| {
                crate::trains::resolve::radsatz::without_leading_zeros(&radsatz.match_key)
                    == stripped
            })
            .collect()
    }

    pub fn wagen(&self) -> Vec<Wagen> {
        self.wagen.values().cloned().collect()
    }

    pub fn partner(&self) -> Vec<Partner> {
        self.partner.values().cloned().collect()
    }

    pub fn templates(&self) -> Vec<ImportTemplate> {
        let shadowed: HashSet<&str> = self
            .templates
            .values()
            .filter_map(|template| template.origin.as_deref())
            .collect();
        super::builtin::all()
            .into_iter()
            .filter(|template| !shadowed.contains(template.id.as_str()))
            .chain(self.templates.values().cloned())
            .collect()
    }

    pub fn counts(&self) -> TrainsCounts {
        TrainsCounts {
            wagen: self.wagen.len() as u32,
            partner: self.partner.len() as u32,
            instandhaltungen: self.instandhaltungen.len() as u32,
            radsaetze: self.radsaetze.len() as u32,
            dokumente: self.dokumente.len() as u32,
        }
    }

    pub fn settings(&self) -> TrainsSettings {
        self.settings
    }

    pub fn save_settings(&mut self, settings: TrainsSettings) -> AppResult<()> {
        write_whole(&self.folder.join(SETTINGS), &settings)?;
        self.settings = settings;
        Ok(())
    }

    pub fn master(&self) -> &MasterSettings {
        &self.master
    }

    pub fn save_master(&mut self, master: MasterSettings) -> AppResult<()> {
        write_whole(&self.folder.join(MASTER), &master)?;
        self.master = master;
        Ok(())
    }

    pub fn dokumente(&self) -> Vec<Dokument> {
        self.dokumente.values().cloned().collect()
    }

    pub fn dokument(&self, id: &str) -> Option<&Dokument> {
        self.dokumente.get(id)
    }

    pub fn dokument_by_hash(&self, hash: &str) -> Option<&Dokument> {
        self.by_hash.get(hash).and_then(|id| self.dokumente.get(id))
    }

    pub fn dokumente_folder(&self) -> PathBuf {
        self.folder.join("dokumente")
    }

    pub fn master_folder(&self) -> PathBuf {
        self.folder.join("master")
    }

    pub fn wagen_by_id(&self, id: &str) -> Option<&Wagen> {
        self.wagen.get(id)
    }

    pub fn partner_by_id(&self, id: &str) -> Option<&Partner> {
        self.partner.get(id)
    }

    pub fn wagen_by_nummer(&self, uic: &str) -> Option<&Wagen> {
        self.by_wagennummer
            .get(uic)
            .and_then(|id| self.wagen.get(id))
    }

    pub fn partner_by_key(&self, key: &str) -> Option<&Partner> {
        self.by_match_key
            .get(key)
            .and_then(|id| self.partner.get(id))
    }

    pub fn wagen_starting_with(&self, stem: &str) -> Vec<&Wagen> {
        self.wagen
            .values()
            .filter(|wagen| wagen.nummer.starts_with(stem))
            .collect()
    }

    pub fn partner_with_rolle(&self, role: PartnerRolle) -> Vec<&Partner> {
        self.partner
            .values()
            .filter(|partner| partner.has_rolle(role))
            .collect()
    }

    pub fn event_exists(&self, dedupe_key: &str) -> bool {
        self.by_dedupe.contains(dedupe_key)
    }

    pub fn template(&self, id: &str) -> Option<ImportTemplate> {
        self.templates.get(id).cloned().or_else(|| {
            super::builtin::all()
                .into_iter()
                .find(|template| template.id == id)
        })
    }

    pub fn user_copy_of(&self, builtin_id: &str) -> Option<&ImportTemplate> {
        self.templates
            .values()
            .find(|template| template.origin.as_deref() == Some(builtin_id))
    }

    /// Every event, borrowed and in insertion order. The exporters write the
    /// whole table and care about neither order nor ownership, so they must not
    /// go through `instandhaltungen_page`, which sorts and clones all of it.
    pub fn instandhaltungen(&self) -> impl Iterator<Item = &Instandhaltung> {
        self.instandhaltungen.values()
    }

    /// Newest first, which is the order a maintenance list is read in.
    pub fn instandhaltungen_page(
        &self,
        wagen_id: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> (Vec<Instandhaltung>, u32) {
        let mut matching: Vec<&Instandhaltung> = self
            .instandhaltungen
            .values()
            .filter(|event| wagen_id.is_none_or(|id| event.wagen_id == id))
            .collect();
        matching.sort_by(|a, b| match (&a.datum, &b.datum) {
            (Some(left), Some(right)) => right.cmp(left),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        let total = matching.len() as u32;
        let rows = matching
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        (rows, total)
    }

    pub fn transaction<T>(
        &mut self,
        work: impl FnOnce(&mut Tx<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut tx = Tx {
            db: self,
            rollback: Rollback::default(),
        };
        let outcome = work(&mut tx).and_then(|value| tx.flush().map(|()| value));
        let rollback = tx.rollback;
        match outcome {
            Ok(value) => Ok(value),
            Err(error) => {
                if let Some(items) = rollback.wagen {
                    self.wagen = items;
                }
                if let Some(items) = rollback.partner {
                    self.partner = items;
                }
                if let Some(items) = rollback.instandhaltungen {
                    self.instandhaltungen = items;
                }
                if let Some(items) = rollback.templates {
                    self.templates = items;
                }
                if let Some(items) = rollback.radsaetze {
                    self.radsaetze = items;
                }
                if let Some(items) = rollback.einbauten {
                    self.einbauten = items;
                }
                if let Some(items) = rollback.dokumente {
                    self.dokumente = items;
                }
                self.reindex();
                Err(error)
            }
        }
    }

    pub fn reset(&mut self) -> AppResult<()> {
        self.transaction(|tx| {
            tx.wagen_mut().clear();
            tx.partner_mut().clear();
            tx.instandhaltungen_mut().clear();
            tx.templates_mut().clear();
            tx.radsaetze_mut().clear();
            tx.einbauten_mut().clear();
            tx.dokumente_mut().clear();
            Ok(())
        })?;
        self.reindex();
        remove_folder(&self.dokumente_folder())?;
        remove_folder(&self.master_folder())
    }
}

impl TrainsDb {
    pub fn clear_mirror(&mut self) -> AppResult<()> {
        self.transaction(|tx| {
            tx.wagen_mut().clear();
            tx.instandhaltungen_mut().clear();
            tx.radsaetze_mut().clear();
            tx.einbauten_mut().clear();
            for dokument in tx.dokumente_mut().values_mut() {
                dokument.importiert_am = None;
            }
            Ok(())
        })?;
        self.reindex();
        Ok(())
    }
}

impl Tx<'_> {
    fn wagen_mut(&mut self) -> &mut IndexMap<String, Wagen> {
        let Tx { db, rollback } = self;
        rollback.wagen.get_or_insert_with(|| db.wagen.clone());
        &mut db.wagen
    }

    fn partner_mut(&mut self) -> &mut IndexMap<String, Partner> {
        let Tx { db, rollback } = self;
        rollback.partner.get_or_insert_with(|| db.partner.clone());
        &mut db.partner
    }

    fn instandhaltungen_mut(&mut self) -> &mut IndexMap<String, Instandhaltung> {
        let Tx { db, rollback } = self;
        rollback
            .instandhaltungen
            .get_or_insert_with(|| db.instandhaltungen.clone());
        &mut db.instandhaltungen
    }

    fn templates_mut(&mut self) -> &mut IndexMap<String, ImportTemplate> {
        let Tx { db, rollback } = self;
        rollback
            .templates
            .get_or_insert_with(|| db.templates.clone());
        &mut db.templates
    }

    fn radsaetze_mut(&mut self) -> &mut IndexMap<String, Radsatz> {
        let Tx { db, rollback } = self;
        rollback
            .radsaetze
            .get_or_insert_with(|| db.radsaetze.clone());
        &mut db.radsaetze
    }

    fn einbauten_mut(&mut self) -> &mut IndexMap<String, Einbau> {
        let Tx { db, rollback } = self;
        rollback
            .einbauten
            .get_or_insert_with(|| db.einbauten.clone());
        &mut db.einbauten
    }

    fn dokumente_mut(&mut self) -> &mut IndexMap<String, Dokument> {
        let Tx { db, rollback } = self;
        rollback
            .dokumente
            .get_or_insert_with(|| db.dokumente.clone());
        &mut db.dokumente
    }

    pub fn put_dokument(&mut self, dokument: Dokument) {
        self.db
            .by_hash
            .insert(dokument.original_hash.clone(), dokument.id.clone());
        self.dokumente_mut().insert(dokument.id.clone(), dokument);
    }

    pub fn mark_imported(&mut self, id: &str, stamp: &str) {
        if let Some(dokument) = self.dokumente_mut().get_mut(id) {
            dokument.importiert_am = Some(stamp.to_string());
        }
    }

    pub fn put_wagen(&mut self, wagen: Wagen) {
        self.db
            .by_wagennummer
            .insert(wagen.nummer.clone(), wagen.id.clone());
        self.wagen_mut().insert(wagen.id.clone(), wagen);
    }

    pub fn remove_wagen(&mut self, id: &str) {
        if let Some(wagen) = self.wagen_mut().shift_remove(id) {
            self.db.by_wagennummer.remove(&wagen.nummer);
        }
        self.instandhaltungen_mut()
            .retain(|_, event| event.wagen_id != id);
        self.einbauten_mut()
            .retain(|_, einbau| einbau.wagen_id != id);
    }

    pub fn put_partner(&mut self, partner: Partner) {
        for key in std::iter::once(partner.match_key.clone()).chain(partner.aliases.iter().cloned())
        {
            self.db.by_match_key.insert(key, partner.id.clone());
        }
        self.partner_mut().insert(partner.id.clone(), partner);
    }

    pub fn remove_partner(&mut self, id: &str) {
        if let Some(partner) = self.partner_mut().shift_remove(id) {
            let removed = partner.id.clone();
            self.db
                .by_match_key
                .retain(|_, partner_id| partner_id != &removed);
        }
        for wagen in self.wagen_mut().values_mut() {
            if wagen.halter_id.as_deref() == Some(id) {
                wagen.halter_id = None;
            }
        }
        for event in self.instandhaltungen_mut().values_mut() {
            if event.werkstatt_id.as_deref() == Some(id) {
                event.werkstatt_id = None;
            }
        }
    }

    /// Records a raw source spelling against a partner, so the next file from
    /// that sender resolves without asking. This is the learning loop.
    pub fn add_rolle(&mut self, partner_id: &str, role: PartnerRolle) {
        if self
            .db
            .partner
            .get(partner_id)
            .is_none_or(|partner| partner.has_rolle(role))
        {
            return;
        }
        if let Some(partner) = self.partner_mut().get_mut(partner_id) {
            partner.rollen.push(role);
        }
    }

    pub fn learn_alias(&mut self, partner_id: &str, alias: &str) {
        let Some(partner) = self.db.partner.get(partner_id) else {
            return;
        };
        if alias.is_empty()
            || partner.match_key == alias
            || partner.aliases.iter().any(|a| a == alias)
        {
            return;
        }
        if let Some(partner) = self.partner_mut().get_mut(partner_id) {
            partner.aliases.push(alias.to_string());
        }
        self.db
            .by_match_key
            .insert(alias.to_string(), partner_id.to_string());
    }

    pub fn put_radsatz(&mut self, radsatz: Radsatz) {
        self.radsaetze_mut().insert(radsatz.id.clone(), radsatz);
        self.db.reindex_radsaetze();
    }

    /// Kept, never matched on: the EN 13261 stamp on the axle is the only
    /// near-global identifier a radsatz has, and the system id is unique only in
    /// ONE sender's system — no rule may lean on either until real files prove
    /// it. Only fills a blank, so one sloppy file cannot overwrite a good value.
    pub fn fill_identifiers(
        &mut self,
        radsatz_id: &str,
        welle: Option<&str>,
        system_id: Option<&str>,
    ) {
        let filled = |value: Option<&str>| {
            value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let (welle, system_id) = (filled(welle), filled(system_id));
        let Some(radsatz) = self.db.radsaetze.get(radsatz_id) else {
            return;
        };
        let welle = welle.filter(|_| radsatz.wellennummer.is_none());
        let system_id = system_id.filter(|_| radsatz.system_id.is_none());
        if welle.is_none() && system_id.is_none() {
            return;
        }
        if let Some(radsatz) = self.radsaetze_mut().get_mut(radsatz_id) {
            if welle.is_some() {
                radsatz.wellennummer = welle;
            }
            if system_id.is_some() {
                radsatz.system_id = system_id;
            }
        }
    }

    /// Learns "THIS SENDER calls it this", which is what turns the next file
    /// from that sender into a `Known`. A repeat is a no-op, so confirming the
    /// same row twice cannot grow the list.
    pub fn learn_radsatz_alias(&mut self, radsatz_id: &str, key: &str, partner_id: Option<&str>) {
        let Some(radsatz) = self.db.radsaetze.get(radsatz_id) else {
            return;
        };
        if radsatz.known_to(key, partner_id) {
            return;
        }
        let alias = RadsatzAlias {
            match_key: key.to_string(),
            partner_id: partner_id.map(str::to_string),
        };
        if let Some(radsatz) = self.radsaetze_mut().get_mut(radsatz_id) {
            radsatz.aliases.push(alias);
        }
        self.db.reindex_radsaetze();
    }

    pub fn remove_radsatz(&mut self, id: &str) {
        if self.radsaetze_mut().shift_remove(id).is_some() {
            self.db.reindex_radsaetze();
        }
        self.einbauten_mut()
            .retain(|_, einbau| einbau.radsatz_id != id);
        for event in self.instandhaltungen_mut().values_mut() {
            if event.radsatz_id.as_deref() == Some(id) {
                event.radsatz_id = None;
            }
        }
    }

    pub fn put_einbau(&mut self, einbau: Einbau) {
        self.einbauten_mut().insert(einbau.id.clone(), einbau);
    }

    /// Closes the radsatz's open fitting, if it has one. A radsatz cannot be
    /// under two wagen, so fitting it somewhere else ends the previous one.
    pub fn close_open_einbau(&mut self, radsatz_id: &str, ausgebaut_am: Option<String>) {
        if self.db.einbauten.is_empty() {
            return;
        }
        let open: Vec<String> = self
            .db
            .einbauten
            .values()
            .filter(|einbau| einbau.radsatz_id == radsatz_id && einbau.is_open())
            .map(|einbau| einbau.id.clone())
            .collect();
        let einbauten = self.einbauten_mut();
        for id in open {
            if let Some(einbau) = einbauten.get_mut(&id) {
                einbau.ausgebaut_am = ausgebaut_am.clone();
            }
        }
    }

    pub fn put_instandhaltung(&mut self, event: Instandhaltung) {
        self.db.by_dedupe.insert(event.dedupe_key.clone());
        self.instandhaltungen_mut().insert(event.id.clone(), event);
    }

    pub fn put_template(&mut self, template: ImportTemplate) {
        self.templates_mut().insert(template.id.clone(), template);
    }

    pub fn remove_template(&mut self, id: &str) {
        self.templates_mut().shift_remove(id);
    }

    pub fn db(&self) -> &TrainsDb {
        self.db
    }

    fn flush(&mut self) -> AppResult<()> {
        let folder = &self.db.folder;
        if self.rollback.wagen.is_some() {
            write(&folder.join("wagen.db"), &self.db.wagen)?;
        }
        if self.rollback.partner.is_some() {
            write(&folder.join("partner.db"), &self.db.partner)?;
        }
        if self.rollback.instandhaltungen.is_some() {
            write(
                &folder.join("instandhaltungen.db"),
                &self.db.instandhaltungen,
            )?;
        }
        if self.rollback.templates.is_some() {
            write(&folder.join("templates.db"), &self.db.templates)?;
        }
        if self.rollback.radsaetze.is_some() {
            write(&folder.join("radsaetze.db"), &self.db.radsaetze)?;
        }
        if self.rollback.einbauten.is_some() {
            write(&folder.join("einbauten.db"), &self.db.einbauten)?;
        }
        if self.rollback.dokumente.is_some() {
            write(&folder.join("dokumente.db"), &self.db.dokumente)?;
        }
        Ok(())
    }
}

fn read<T: DeserializeOwned>(path: &Path) -> AppResult<IndexMap<String, T>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(IndexMap::new()),
        Err(error) => return Err(AppError::io(path, error)),
    };
    let store: Store<T> =
        serde_json::from_str(&text).map_err(|error| AppError::json(path, error))?;
    Ok(store.items)
}

fn read_or_default<T: DeserializeOwned + Default>(path: &Path) -> AppResult<T> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|error| AppError::json(path, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn write_whole<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let json = serde_json::to_vec(value).map_err(|error| AppError::json(path, error))?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, &json).map_err(|error| AppError::io(&temp, error))?;
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

fn write<T: Serialize>(path: &Path, items: &IndexMap<String, T>) -> AppResult<()> {
    let store = StoreRef {
        version: VERSION,
        items,
    };
    let json = serde_json::to_vec(&store).map_err(|error| AppError::json(path, error))?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, &json).map_err(|error| AppError::io(&temp, error))?;
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::Provenance;

    fn wagen(id: &str, uic: &str) -> Wagen {
        Wagen {
            id: id.into(),
            nummer: uic.into(),
            halter_id: None,
            eigentuemer_id: None,
            bauart: None,
            bemerkung: None,
            created_at: "2026-08-16".into(),
            source: None,
        }
    }

    fn partner(id: &str, name: &str, key: &str) -> Partner {
        Partner {
            id: id.into(),
            rollen: vec![PartnerRolle::Werkstatt],
            name: name.into(),
            match_key: key.into(),
            aliases: Vec::new(),
            bemerkung: None,
            created_at: "2026-08-16".into(),
        }
    }

    fn event(id: &str, wagen_id: &str, date: &str, dedupe: &str) -> Instandhaltung {
        Instandhaltung {
            id: id.into(),
            wagen_id: wagen_id.into(),
            werkstatt_id: None,
            radsatz_id: None,
            datum: Some(date.into()),
            leistung: "Bremsprobe".into(),
            betrag_cent: Some(1234),
            bemerkung: None,
            dedupe_key: dedupe.into(),
            source: Provenance {
                file: "a.xlsx".into(),
                sheet: "Tabelle1".into(),
                row: 2,
                imported_at: "2026-08-16".into(),
            },
        }
    }

    fn db(folder: &TempDir) -> TrainsDb {
        TrainsDb::load(&folder.config()).expect("loads")
    }

    #[test]
    fn a_reset_forgets_the_dedupe_keys_too() {
        let folder = TempDir::new("trains-reset-index");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "218124712173"));
                tx.put_instandhaltung(event("e1", "w1", "2026-01-01", "k1"));
                Ok(())
            })
            .unwrap();
        assert!(store.event_exists("k1"));

        store.reset().unwrap();
        // A stale index made every row of a re-imported file a duplicate.
        assert!(!store.event_exists("k1"));
    }

    #[test]
    fn clearing_the_mirror_keeps_identity_and_reopens_documents() {
        let folder = TempDir::new("trains-clear-mirror");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "218124712173"));
                tx.put_partner(partner("p1", "Rundlauf Radsatztechnik", "RUNDLAUF"));
                tx.put_instandhaltung(event("e1", "w1", "2026-01-01", "k1"));
                Ok(())
            })
            .unwrap();

        store.clear_mirror().unwrap();

        assert_eq!(store.counts().wagen, 0);
        assert!(!store.event_exists("k1"));
        assert!(store.wagen_by_nummer("218124712173").is_none());
        assert_eq!(
            store.partner().len(),
            1,
            "a Partner is identity, not a fact"
        );
        assert!(store.partner_by_key("RUNDLAUF").is_some());

        let reloaded = db(&folder);
        assert_eq!(
            reloaded.counts().wagen,
            0,
            "the wipe was written, not only held"
        );
        assert_eq!(reloaded.partner().len(), 1);
    }

    #[test]
    fn a_missing_store_loads_as_empty_rather_than_failing() {
        let folder = TempDir::new("trains-empty");
        let db = db(&folder);
        assert_eq!(db.counts(), TrainsCounts::default());
    }

    #[test]
    fn what_a_transaction_writes_survives_a_reload() {
        let folder = TempDir::new("trains-roundtrip");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "318047401234"));
                tx.put_partner(partner("p1", "Müller", "mueller"));
                tx.put_instandhaltung(event("e1", "w1", "2025-12-31", "k1"));
                Ok(())
            })
            .unwrap();

        let reloaded = db(&folder);
        assert_eq!(reloaded.counts().wagen, 1);
        assert_eq!(reloaded.counts().partner, 1);
        assert_eq!(reloaded.counts().instandhaltungen, 1);
        assert_eq!(reloaded.wagen_by_nummer("318047401234").unwrap().id, "w1");
        assert_eq!(reloaded.partner_by_key("mueller").unwrap().id, "p1");
        assert!(reloaded.event_exists("k1"));
    }

    /// The whole point of the API: one import is a handful of file writes, not
    /// one per row.
    #[test]
    fn a_transaction_writes_each_touched_store_once() {
        let folder = TempDir::new("trains-batched");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                for index in 0..50 {
                    tx.put_wagen(wagen(
                        &format!("w{index}"),
                        &format!("3180474012{index:02}"),
                    ));
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(db(&folder).counts().wagen, 50);
        // Only the touched store exists on disk; partners were never written.
        assert!(folder.join("data/trains/wagen.db").exists());
        assert!(!folder.join("data/trains/partners.db").exists());
    }

    /// A store that reports success and did not write is the failure this
    /// refuses to allow — so a failed body leaves memory as it was.
    #[test]
    fn a_failed_transaction_rolls_the_memory_back() {
        let folder = TempDir::new("trains-rollback");
        let mut store = db(&folder);
        let outcome: AppResult<()> = store.transaction(|tx| {
            tx.put_wagen(wagen("w1", "318047401234"));
            Err(AppError::Report(vec!["Abbruch".into()]))
        });
        assert!(outcome.is_err());
        assert_eq!(store.counts().wagen, 0);
        assert!(store.wagen_by_nummer("318047401234").is_none());
    }

    /// The rollback copy is per store and taken on first write, so a failure
    /// must undo the stores that WERE written and leave the rest — including
    /// their indexes — exactly as they were.
    #[test]
    fn a_failed_transaction_rolls_back_only_what_it_touched() {
        let folder = TempDir::new("trains-rollback-partial");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "318047401234"));
                tx.put_partner(partner("p1", "Müller", "mueller"));
                Ok(())
            })
            .unwrap();

        let outcome: AppResult<()> = store.transaction(|tx| {
            tx.put_partner(partner("p2", "Bahn", "bahn"));
            Err(AppError::Report(vec!["Abbruch".into()]))
        });

        assert!(outcome.is_err());
        assert_eq!(store.counts().partner, 1, "the touched store is undone");
        assert!(store.partner_by_key("bahn").is_none(), "index too");
        assert_eq!(store.counts().wagen, 1, "the untouched store survives");
        assert_eq!(
            store.wagen_by_nummer("318047401234").unwrap().id,
            "w1",
            "and so does its index"
        );
    }

    #[test]
    fn an_alias_is_learnt_once_and_then_resolves() {
        let folder = TempDir::new("trains-alias");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_partner(partner("p1", "Müller", "mueller"));
                tx.learn_alias("p1", "fa mueller co kg");
                tx.learn_alias("p1", "fa mueller co kg");
                Ok(())
            })
            .unwrap();
        assert_eq!(store.partner_by_key("fa mueller co kg").unwrap().id, "p1");
        assert_eq!(store.partner_by_id("p1").unwrap().aliases.len(), 1);
        assert_eq!(
            db(&folder).partner_by_key("fa mueller co kg").unwrap().id,
            "p1"
        );
    }

    #[test]
    fn removing_a_wagen_takes_its_events_with_it() {
        let folder = TempDir::new("trains-cascade-wagen");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "318047401234"));
                tx.put_instandhaltung(event("e1", "w1", "2025-12-31", "k1"));
                tx.put_instandhaltung(event("e2", "w2", "2025-12-31", "k2"));
                Ok(())
            })
            .unwrap();
        store
            .transaction(|tx| {
                tx.remove_wagen("w1");
                Ok(())
            })
            .unwrap();
        assert_eq!(store.counts().instandhaltungen, 1);
        assert!(store.wagen_by_nummer("318047401234").is_none());
    }

    /// A removed partner must not leave wagen and events pointing at an id
    /// that is gone — the same cascade the filler store already owes profiles.
    #[test]
    fn removing_a_partner_unhooks_it_everywhere() {
        let folder = TempDir::new("trains-cascade-partner");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_partner(partner("p1", "Müller", "mueller"));
                let mut owned = wagen("w1", "318047401234");
                owned.halter_id = Some("p1".into());
                tx.put_wagen(owned);
                let mut serviced = event("e1", "w1", "2025-12-31", "k1");
                serviced.werkstatt_id = Some("p1".into());
                tx.put_instandhaltung(serviced);
                Ok(())
            })
            .unwrap();

        store
            .transaction(|tx| {
                tx.remove_partner("p1");
                Ok(())
            })
            .unwrap();

        assert!(store.wagen_by_id("w1").unwrap().halter_id.is_none());
        assert!(store.partner_by_key("mueller").is_none());
        let (rows, _) = store.instandhaltungen_page(None, 0, 10);
        assert!(rows[0].werkstatt_id.is_none());
    }

    #[test]
    fn events_come_back_newest_first_and_paged() {
        let folder = TempDir::new("trains-page");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_instandhaltung(event("e1", "w1", "2025-01-01", "k1"));
                tx.put_instandhaltung(event("e2", "w1", "2025-12-31", "k2"));
                tx.put_instandhaltung(event("e3", "w2", "2025-06-15", "k3"));
                Ok(())
            })
            .unwrap();

        let (rows, total) = store.instandhaltungen_page(None, 0, 2);
        assert_eq!(total, 3);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datum.as_deref(), Some("2025-12-31"));
        assert_eq!(rows[1].datum.as_deref(), Some("2025-06-15"));

        let (rows, total) = store.instandhaltungen_page(Some("w1"), 0, 10);
        assert_eq!(total, 2);
        assert!(rows.iter().all(|event| event.wagen_id == "w1"));

        let (rows, _) = store.instandhaltungen_page(None, 2, 10);
        assert_eq!(rows.len(), 1);
    }

    /// Compact is the default, a missing file is the default, and a saved
    /// choice survives a restart.
    #[test]
    fn the_settings_default_to_compact_and_survive_a_reload() {
        use crate::trains::model::UicStyle;
        let folder = TempDir::new("trains-settings");
        let mut store = TrainsDb::load(&folder.config()).unwrap();
        assert_eq!(store.settings().wagennummer, UicStyle::Compact);

        store
            .save_settings(TrainsSettings {
                wagennummer: UicStyle::Grouped,
            })
            .unwrap();
        let reloaded = TrainsDb::load(&folder.config()).unwrap();
        assert_eq!(reloaded.settings().wagennummer, UicStyle::Grouped);
    }

    #[test]
    fn a_reset_empties_every_store() {
        let folder = TempDir::new("trains-reset");
        let mut store = db(&folder);
        store
            .transaction(|tx| {
                tx.put_wagen(wagen("w1", "318047401234"));
                tx.put_instandhaltung(event("e1", "w1", "2025-12-31", "k1"));
                Ok(())
            })
            .unwrap();
        store.reset().unwrap();
        assert_eq!(store.counts(), TrainsCounts::default());
        assert_eq!(db(&folder).counts(), TrainsCounts::default());
    }

    #[test]
    fn a_corrupt_store_is_an_error_rather_than_a_silent_empty() {
        let folder = TempDir::new("trains-corrupt");
        std::fs::create_dir_all(folder.join("data/trains")).unwrap();
        std::fs::write(folder.join("data/trains/wagen.db"), "{kaputt").unwrap();
        assert!(TrainsDb::load(&folder.config()).is_err());
    }
}
