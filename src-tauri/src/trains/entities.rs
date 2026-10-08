// ─── why ────────────────────────────────────────────────────────
// The import walk asks about ENTITIES, not rows. Forty rows naming „Fa. Müller“
// are one question, and answering it forty times is how a user learns to click
// through. So a staged file is grouped per type, and the answers are expanded
// back into the per-row decisions `commit` has always run on — the commit and
// every gate in it stay exactly as they were.
//
// A group's KEY is that type's identity rule, already established elsewhere:
//
//   Partner  role + `match_key`. Resolution is role-scoped (`find_partner`), so
//            the same name as Werkstatt and as Halter is two questions.
//   Wagen    the canonical twelve digits — the one trustworthy field.
//   Radsatz  `match_key` + the sender STAGING used. A Radsatznummer is not a
//            key: the same string from two senders may be two radsaetze, so it
//            is two groups. The sender is the one staging resolved against and
//            deliberately NOT the walk's Partner answer — the steps stay
//            independent, and a Werkstatt first confirmed in this walk makes its
//            radsaetze a question rather than a silent match.
//
// Rejected rows are not grouped: they cannot be committed, so asking about
// their entities asks about nothing. Duplicates ARE grouped — the user may
// still take one.
//
// The decision default is SKIP. A group the walk sent no answer for is not
// created; that is the same direction every other default in the import takes.
//
// A group carries the protocol lines of ITS cells — the rows it was read from,
// in the column its field was mapped to — so the walk can show what the cleaning
// did to exactly this value.
//
// EINBAU CONFLICTS are the one question that is not an entity. The master is
// several sheets of one portal report at different dates, imported one after
// the other, so a Radsatz already open on a Wagen can arrive again on the SAME
// Wagen with another install date. That is one fitting with a disputed date,
// not a movement, and closing the stored one would invent a removal that never
// happened. `einbau_konflikte` finds them against the STORE — `group` sees only
// the staging, so it cannot — keyed by the Radsatz id; `expand` hands the answer
// to each of its rows, and a missing answer keeps what is stored. Only the
// master path asks: everywhere else a new date is still a movement.
// ────────────────────────────────────────────────────────────────

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use super::db::TrainsDb;
use super::model::{
    CommitDecisions, EinbauKonflikt, EntityChoice, EntityDecision, EntityDecisions, EntityGroup,
    EntityGroups, EntityKind, FieldKind, PartnerRolle, ProtocolLine, Resolution, RowDecision,
    RowStatus, StagedImport, StagedRow,
};
use super::resolve::{partner, radsatz};
use super::sanitise::{format, Value};
use super::stage::RowValues;

const PARTNERS: [(PartnerRolle, FieldKind); 3] = [
    (PartnerRolle::Werkstatt, FieldKind::Werkstatt),
    (PartnerRolle::Halter, FieldKind::Halter),
    (PartnerRolle::Eigentuemer, FieldKind::Eigentuemer),
];

struct RowKeys {
    wagen: Option<String>,
    werkstatt: Option<String>,
    halter: Option<String>,
    eigentuemer: Option<String>,
    radsatz: Option<String>,
}

pub fn group(staging: &StagedImport, protocol: &[ProtocolLine]) -> EntityGroups {
    let mut lines: HashMap<(u32, u32), Vec<&ProtocolLine>> = HashMap::new();
    for line in protocol {
        lines.entry((line.row, line.column)).or_default().push(line);
    }
    let column = |field: FieldKind| staging.plan.binding(field).map(|binding| binding.index);

    let mut partners = Groups::default();
    let mut wagen = Groups::default();
    let mut radsaetze = Groups::default();

    for row in staging
        .rows
        .iter()
        .filter(|row| row.status != RowStatus::Rejected)
    {
        let keys = keys_of(row);
        if let Some(key) = keys.wagen {
            wagen.add(
                key,
                EntityKind::Wagen,
                None,
                row,
                FieldKind::Wagennummer,
                &row.wagen,
            );
        }
        for ((rolle, field), key) in
            PARTNERS
                .into_iter()
                .zip([keys.werkstatt, keys.halter, keys.eigentuemer])
        {
            if let Some(key) = key {
                partners.add(
                    key,
                    EntityKind::Partner,
                    Some(rolle),
                    row,
                    field,
                    partner_of(row, rolle),
                );
            }
        }
        if let Some(key) = keys.radsatz {
            radsaetze.add(
                key,
                EntityKind::Radsatz,
                None,
                row,
                FieldKind::Radsatznummer,
                &row.radsatz,
            );
        }
    }

    let attach = |groups: Groups| -> Vec<EntityGroup> {
        groups
            .0
            .into_values()
            .map(|(mut group, field)| {
                if let Some(index) = column(field) {
                    group.changes = group
                        .rows
                        .iter()
                        .flat_map(|row| lines.get(&(*row, index)).into_iter().flatten())
                        .map(|line| (*line).clone())
                        .collect();
                }
                group
            })
            .collect()
    };

    EntityGroups {
        partner: attach(partners),
        wagen: attach(wagen),
        radsaetze: attach(radsaetze),
        einbauten: Vec::new(),
    }
}

pub fn einbau_konflikte(
    db: &TrainsDb,
    staging: &StagedImport,
    values: &[RowValues],
) -> Vec<EinbauKonflikt> {
    let installed: HashMap<u32, String> = values
        .iter()
        .filter_map(|entry| match entry.value(FieldKind::EingebautAm) {
            Some(Value::Date(date)) => Some((entry.row, date.to_iso())),
            _ => None,
        })
        .collect();

    let mut found: IndexMap<String, EinbauKonflikt> = IndexMap::new();
    for row in staging
        .rows
        .iter()
        .filter(|row| row.status != RowStatus::Rejected)
    {
        let (
            Resolution::Known {
                id: radsatz_id,
                name: radsatz,
            },
            Resolution::Known { id: wagen_id, .. },
            Some(neu),
        ) = (&row.radsatz, &row.wagen, installed.get(&row.row))
        else {
            continue;
        };
        let Some(open) = db.open_einbau(radsatz_id) else {
            continue;
        };
        if open.wagen_id != *wagen_id || open.eingebaut_am.as_deref() == Some(neu.as_str()) {
            continue;
        }
        let konflikt = found
            .entry(radsatz_id.clone())
            .or_insert_with(|| EinbauKonflikt {
                key: radsatz_id.clone(),
                radsatz: radsatz.clone(),
                wagen: db.wagen_by_id(wagen_id).map_or_else(String::new, |wagen| {
                    format::uic_in(&wagen.nummer, db.settings().wagennummer)
                }),
                bisher: open.eingebaut_am.clone(),
                bisher_quelle: Some(format!("{}, Zeile {}", open.source.sheet, open.source.row)),
                neu: neu.clone(),
                rows: Vec::new(),
            });
        konflikt.rows.push(row.row);
    }
    found.into_values().collect()
}

pub fn expand(staging: &StagedImport, decisions: &EntityDecisions) -> CommitDecisions {
    let lookup = |choices: &[EntityChoice]| -> HashMap<String, EntityDecision> {
        choices
            .iter()
            .map(|choice| (choice.key.clone(), choice.decision.clone()))
            .collect()
    };
    let partners = lookup(&decisions.partner);
    let wagen = lookup(&decisions.wagen);
    let radsaetze = lookup(&decisions.radsaetze);
    let uebernehmen: HashSet<u32> = staging
        .entities
        .iter()
        .flat_map(|groups| &groups.einbauten)
        .filter(|konflikt| {
            decisions
                .einbauten
                .iter()
                .any(|choice| choice.key == konflikt.key && choice.uebernehmen)
        })
        .flat_map(|konflikt| konflikt.rows.iter().copied())
        .collect();
    let decided = |map: &HashMap<String, EntityDecision>, key: Option<String>| {
        key.and_then(|key| map.get(&key).cloned())
            .unwrap_or(EntityDecision::Skip)
    };

    let by_number: HashMap<u32, &StagedRow> =
        staging.rows.iter().map(|row| (row.row, row)).collect();
    let mut seen = HashSet::new();
    let rows = decisions
        .rows
        .iter()
        .filter(|number| seen.insert(**number))
        .map(|number| match by_number.get(number) {
            Some(row) => {
                let keys = keys_of(row);
                RowDecision {
                    row: *number,
                    wagen: decided(&wagen, keys.wagen),
                    werkstatt: decided(&partners, keys.werkstatt),
                    halter: decided(&partners, keys.halter),
                    eigentuemer: decided(&partners, keys.eigentuemer),
                    radsatz: decided(&radsaetze, keys.radsatz),
                    einbau_uebernehmen: uebernehmen.contains(number),
                }
            }
            None => RowDecision {
                row: *number,
                wagen: EntityDecision::Skip,
                werkstatt: EntityDecision::Skip,
                halter: EntityDecision::Skip,
                eigentuemer: EntityDecision::Skip,
                radsatz: EntityDecision::Skip,
                einbau_uebernehmen: false,
            },
        })
        .collect();

    CommitDecisions {
        staging_id: decisions.staging_id.clone(),
        rows,
    }
}

#[derive(Default)]
struct Groups(IndexMap<String, (EntityGroup, FieldKind)>);

impl Groups {
    fn add(
        &mut self,
        key: String,
        kind: EntityKind,
        rolle: Option<PartnerRolle>,
        row: &StagedRow,
        field: FieldKind,
        resolution: &Resolution,
    ) {
        let spelling = raw_of(row, field).unwrap_or_default().trim().to_string();
        let (group, _) = self.0.entry(key.clone()).or_insert_with(|| {
            (
                EntityGroup {
                    key,
                    kind,
                    rolle,
                    spellings: Vec::new(),
                    resolution: resolution.clone(),
                    rows: Vec::new(),
                    changes: Vec::new(),
                },
                field,
            )
        });
        if !spelling.is_empty() && !group.spellings.contains(&spelling) {
            group.spellings.push(spelling);
        }
        group.rows.push(row.row);
    }
}

fn keys_of(row: &StagedRow) -> RowKeys {
    let partner_key = |rolle: PartnerRolle, field: FieldKind| {
        if matches!(partner_of(row, rolle), Resolution::Missing) {
            return None;
        }
        let key = partner::match_key(raw_of(row, field)?);
        (!key.is_empty()).then(|| format!("{}:{key}", rolle_key(rolle)))
    };
    let wagen = (!matches!(row.wagen, Resolution::Missing))
        .then(|| {
            row.cells
                .iter()
                .find(|cell| cell.field == FieldKind::Wagennummer && cell.ok)
                .map(|cell| cell.parsed.clone())
        })
        .flatten()
        .filter(|parsed| !parsed.is_empty());
    let radsatz = (!matches!(row.radsatz, Resolution::Missing))
        .then(|| raw_of(row, FieldKind::Radsatznummer).map(radsatz::match_key))
        .flatten()
        .filter(|key| !key.is_empty())
        .map(|key| format!("{key}@{}", row.sender.as_deref().unwrap_or("")));

    RowKeys {
        wagen,
        werkstatt: partner_key(PartnerRolle::Werkstatt, FieldKind::Werkstatt),
        halter: partner_key(PartnerRolle::Halter, FieldKind::Halter),
        eigentuemer: partner_key(PartnerRolle::Eigentuemer, FieldKind::Eigentuemer),
        radsatz,
    }
}

fn partner_of(row: &StagedRow, rolle: PartnerRolle) -> &Resolution {
    match rolle {
        PartnerRolle::Werkstatt => &row.werkstatt,
        PartnerRolle::Halter => &row.halter,
        PartnerRolle::Eigentuemer => &row.eigentuemer,
    }
}

fn rolle_key(rolle: PartnerRolle) -> &'static str {
    match rolle {
        PartnerRolle::Werkstatt => "werkstatt",
        PartnerRolle::Halter => "halter",
        PartnerRolle::Eigentuemer => "eigentuemer",
    }
}

fn raw_of(row: &StagedRow, field: FieldKind) -> Option<&str> {
    row.cells
        .iter()
        .find(|cell| cell.field == field)
        .map(|cell| cell.raw.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::db::TrainsDb;
    use crate::trains::model::{ColumnBinding, ImportPlan, Partner, Tier};
    use crate::trains::sheet::grid::Grid;
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;
    use crate::trains::stage::{stage, StageInput};

    fn plan(fields: &[(&str, FieldKind)]) -> ImportPlan {
        ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: fields
                .iter()
                .enumerate()
                .map(|(position, (header, field))| ColumnBinding {
                    header: (*header).into(),
                    index: position as u32 + 1,
                    field: *field,
                    decimal: None,
                    date_order: None,
                })
                .collect(),
            template_id: None,
            date1904: false,
            pruefart: None,
        }
    }

    fn werkstatt(id: &str, name: &str) -> Partner {
        Partner {
            id: id.into(),
            rollen: vec![PartnerRolle::Werkstatt],
            name: name.into(),
            match_key: partner::match_key(name),
            aliases: Vec::new(),
            bemerkung: None,
            created_at: "2026-10-03".into(),
        }
    }

    fn staged(db: &TrainsDb, rows: &[&[&str]], fields: &[(&str, FieldKind)]) -> StagedImport {
        let header: Vec<&str> = fields.iter().map(|(header, _)| *header).collect();
        let mut table: Vec<&[&str]> = vec![&header];
        table.extend_from_slice(rows);
        stage(StageInput {
            id: "s1".into(),
            file: "monat.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid: &Grid::from_text("Tabelle1", &table),
            plan: &plan(fields),
            db,
            master: false,
        })
        .unwrap()
        .wire
    }

    const MONAT: [(&str, FieldKind); 3] = [
        ("Wagennummer", FieldKind::Wagennummer),
        ("Werkstatt", FieldKind::Werkstatt),
        ("Halter", FieldKind::Halter),
    ];

    fn fresh(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    /// Forty rows naming one workshop are one question — but the same name as
    /// Halter is another, because resolution is scoped by role.
    #[test]
    fn one_entity_named_on_many_rows_is_one_group_per_role() {
        let (_f, db) = fresh("ent-groups");
        let wire = staged(
            &db,
            &[
                &["21 81 2471 217-3", "ULA Bebra", ""],
                &["21 81 2471 217-3", "ula  bebra", ""],
                &["33 85 0659 002-9", "ULA Bebra", "ULA Bebra"],
            ],
            &MONAT,
        );
        let groups = group(&wire, &[]);

        assert_eq!(groups.wagen.len(), 2);
        assert_eq!(groups.wagen[0].rows, vec![2, 3]);
        assert_eq!(groups.partner.len(), 2);
        let werkstatt = &groups.partner[0];
        assert_eq!(werkstatt.rolle, Some(PartnerRolle::Werkstatt));
        assert_eq!(werkstatt.rows, vec![2, 3, 4]);
        assert_eq!(werkstatt.spellings, vec!["ULA Bebra", "ula  bebra"]);
        assert_eq!(groups.partner[1].rolle, Some(PartnerRolle::Halter));
        assert!(groups.radsaetze.is_empty());
    }

    /// A Radsatznummer is not a key: the same string from two senders may be
    /// two radsaetze, so it is two questions.
    #[test]
    fn one_radsatznummer_from_two_senders_is_two_groups() {
        let (_f, mut db) = fresh("ent-radsatz");
        db.transaction(|tx| {
            tx.put_partner(werkstatt("p1", "Schienenbein Waggonwerk GmbH"));
            tx.put_partner(werkstatt("p2", "Rundlauf Radsatztechnik"));
            Ok(())
        })
        .unwrap();
        let wire = staged(
            &db,
            &[
                &[
                    "21 81 2471 217-3",
                    "Schienenbein Waggonwerk GmbH",
                    "RS-4711",
                ],
                &["21 81 2471 217-3", "Rundlauf Radsatztechnik", "RS 4711"],
                &["21 81 2471 217-3", "Schienenbein Waggonwerk GmbH", "rs4711"],
            ],
            &[
                ("Wagennummer", FieldKind::Wagennummer),
                ("Werkstatt", FieldKind::Werkstatt),
                ("Radsatz", FieldKind::Radsatznummer),
            ],
        );
        let groups = group(&wire, &[]);

        let keys: Vec<&str> = groups.radsaetze.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys, vec!["RS4711@p1", "RS4711@p2"]);
        assert_eq!(groups.radsaetze[0].rows, vec![2, 4]);
    }

    #[test]
    fn a_group_carries_the_protocol_lines_of_its_own_cells() {
        let (_f, db) = fresh("ent-protocol");
        let wire = staged(&db, &[&["21 81 2471 217-3", "ULA Bebra", ""]], &MONAT);
        let line = |column: u32| ProtocolLine {
            row: 2,
            column,
            header: String::new(),
            raw: "x".into(),
            clean: "y".into(),
            tier: Tier::Format,
            rule: String::new(),
        };
        let groups = group(&wire, &[line(1), line(2), line(9)]);

        assert_eq!(groups.wagen[0].changes, vec![line(1)]);
        assert_eq!(groups.partner[0].changes, vec![line(2)]);
    }

    #[test]
    fn a_rejected_row_raises_no_question() {
        let (_f, db) = fresh("ent-rejected");
        let wire = staged(&db, &[&["keine Nummer", "ULA Bebra", ""]], &MONAT);
        let groups = group(&wire, &[]);
        assert!(groups.wagen.is_empty());
        assert!(groups.partner.is_empty());
    }

    fn choice(key: &str, decision: EntityDecision) -> EntityChoice {
        EntityChoice {
            key: key.into(),
            decision,
        }
    }

    /// One answer per group, fanned out to every row naming it; anything not
    /// answered is not created.
    #[test]
    fn answers_expand_to_every_row_and_silence_means_skip() {
        let (_f, db) = fresh("ent-expand");
        let wire = staged(
            &db,
            &[
                &["21 81 2471 217-3", "ULA Bebra", ""],
                &["33 85 0659 002-9", "ULA Bebra", ""],
            ],
            &MONAT,
        );
        let groups = group(&wire, &[]);
        let decisions = EntityDecisions {
            staging_id: "s1".into(),
            partner: vec![choice(&groups.partner[0].key, EntityDecision::Create)],
            wagen: vec![choice(&groups.wagen[0].key, EntityDecision::Create)],
            radsaetze: Vec::new(),
            einbauten: Vec::new(),
            rows: vec![2, 3, 3],
        };
        let expanded = expand(&wire, &decisions);

        assert_eq!(expanded.staging_id, "s1");
        assert_eq!(expanded.rows.len(), 2, "a row listed twice is taken once");
        assert_eq!(expanded.rows[0].wagen, EntityDecision::Create);
        assert_eq!(expanded.rows[0].werkstatt, EntityDecision::Create);
        assert_eq!(expanded.rows[1].wagen, EntityDecision::Skip);
        assert_eq!(expanded.rows[1].werkstatt, EntityDecision::Create);
        assert_eq!(expanded.rows[0].halter, EntityDecision::Skip);
    }

    #[test]
    fn a_row_not_listed_is_not_committed() {
        let (_f, db) = fresh("ent-rows");
        let wire = staged(&db, &[&["21 81 2471 217-3", "ULA Bebra", ""]], &MONAT);
        let decisions = EntityDecisions {
            staging_id: "s1".into(),
            partner: Vec::new(),
            wagen: Vec::new(),
            radsaetze: Vec::new(),
            einbauten: Vec::new(),
            rows: Vec::new(),
        };
        assert!(expand(&wire, &decisions).rows.is_empty());
    }
}
